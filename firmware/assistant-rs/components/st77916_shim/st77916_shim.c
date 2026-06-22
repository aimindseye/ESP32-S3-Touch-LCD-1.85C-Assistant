#include "st77916_shim.h"

#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <time.h>
#include <dirent.h>

#include "driver/spi_master.h"
#include "driver/gpio.h"
#include "esp_check.h"
#include "esp_lcd_panel_io.h"
#include "esp_lcd_panel_ops.h"
#include "esp_lcd_st77916.h"

#include "esp_vfs_fat.h"
#include "driver/sdmmc_host.h"
#include "sdmmc_cmd.h"
#include "ff.h"
#include "esp_sntp.h"
#include "esp_http_client.h"
#include "esp_adc/adc_oneshot.h"
#include "esp_adc/adc_cali.h"
#include "esp_adc/adc_cali_scheme.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_err.h"
#include "esp_log.h"
#include "esp_timer.h"
#include "esp_heap_caps.h"
#include "jpeg_decoder.h"


#define LCD_OPCODE_READ_CMD                 (0x0BULL)

#define LCD_WIDTH                           360
#define LCD_HEIGHT                          360
#define VIDEO_PREVIEW_MAX_WIDTH             260
#define VIDEO_PREVIEW_MAX_HEIGHT            132
#define VIDEO_PREVIEW_TOP_Y                 82
#define LCD_COLOR_BITS                      16

#define ESP_PANEL_HOST_SPI_ID_DEFAULT       SPI2_HOST
#define ESP_PANEL_LCD_SPI_MODE              0
#define ESP_PANEL_LCD_SPI_CLK_HZ            (80 * 1000 * 1000)
#define ESP_PANEL_LCD_SPI_TRANS_QUEUE_SZ    10
#define ESP_PANEL_LCD_SPI_CMD_BITS          32
#define ESP_PANEL_LCD_SPI_PARAM_BITS        8

#define ESP_PANEL_LCD_SPI_IO_TE             18
#define ESP_PANEL_LCD_SPI_IO_SCK            40
#define ESP_PANEL_LCD_SPI_IO_DATA0          46
#define ESP_PANEL_LCD_SPI_IO_DATA1          45
#define ESP_PANEL_LCD_SPI_IO_DATA2          42
#define ESP_PANEL_LCD_SPI_IO_DATA3          41
#define ESP_PANEL_LCD_SPI_IO_CS             21

#define EXAMPLE_LCD_PIN_NUM_RST             (-1)
#define ESP_PANEL_HOST_SPI_MAX_TRANSFER_SIZE 2048


/*
 * v0.1.30 Persistent SD Mount Foundation
 *
 * All legacy SD helpers still call esp_vfs_fat_sdmmc_mount() and
 * esp_vfs_fat_sdcard_unmount(), but below this point those calls are
 * macro-routed through a single persistent SD session. The first caller
 * performs the real SDMMC mount; later callers acquire the same owner
 * mutex and reuse /sdcard without unmounting the host.
 */
static SemaphoreHandle_t s_sd_owner_mutex = NULL;
static sdmmc_card_t *s_sd_persistent_card = NULL;
static volatile bool s_sd_persistent_mount_ready = false;
static const char *s_sd_owner_label = "NONE";
static volatile uint32_t s_sd_persistent_mount_count = 0;
static volatile uint32_t s_sd_owner_busy_count = 0;
static volatile bool s_runtime_log_debug_enabled = false;

static bool st77916_sd_owner_ensure_mutex(void) {
    if (s_sd_owner_mutex == NULL) {
        s_sd_owner_mutex = xSemaphoreCreateMutex();
    }
    return s_sd_owner_mutex != NULL;
}

static bool st77916_sd_owner_try_acquire(const char *owner, TickType_t wait_ticks) {
    if (!st77916_sd_owner_ensure_mutex()) {
        return false;
    }
    if (xSemaphoreTake(s_sd_owner_mutex, wait_ticks) == pdTRUE) {
        s_sd_owner_label = owner == NULL ? "UNKNOWN" : owner;
        return true;
    }
    s_sd_owner_busy_count++;
    return false;
}

static void st77916_sd_owner_release(const char *owner) {
    (void) owner;
    if (s_sd_owner_mutex != NULL) {
        s_sd_owner_label = "NONE";
        xSemaphoreGive(s_sd_owner_mutex);
    }
}

static esp_err_t st77916_sd_persistent_mount_compat(
    const char *mount_point,
    const sdmmc_host_t *host,
    const sdmmc_slot_config_t *slot_config,
    const esp_vfs_fat_sdmmc_mount_config_t *mount_config,
    sdmmc_card_t **out_card
) {
    if (out_card == NULL) {
        return ESP_ERR_INVALID_ARG;
    }
    *out_card = NULL;

    if (!st77916_sd_owner_try_acquire("SD_COMPAT", pdMS_TO_TICKS(2000))) {
        return ESP_ERR_TIMEOUT;
    }

    if (s_sd_persistent_mount_ready && s_sd_persistent_card != NULL) {
        *out_card = s_sd_persistent_card;
        return ESP_OK;
    }

    sdmmc_card_t *card = NULL;
    esp_err_t err = esp_vfs_fat_sdmmc_mount(mount_point, host, slot_config, mount_config, &card);
    if (err != ESP_OK || card == NULL) {
        st77916_sd_owner_release("SD_COMPAT");
        return err == ESP_OK ? ESP_FAIL : err;
    }

    s_sd_persistent_card = card;
    s_sd_persistent_mount_ready = true;
    s_sd_persistent_mount_count++;
    *out_card = s_sd_persistent_card;
    return ESP_OK;
}

static void st77916_sd_persistent_release_compat(const char *mount_point, sdmmc_card_t *card) {
    (void) mount_point;
    (void) card;
    /*
     * Persistent mount: legacy callers call "unmount" to release their SD
     * ownership, but the SDMMC host remains mounted for the process lifetime.
     */
    st77916_sd_owner_release("SD_COMPAT");
}

bool st77916_sd_persistent_mount_session(void) {
    const char *mount_point = "/sdcard";
    sdmmc_card_t *card = NULL;

    esp_vfs_fat_sdmmc_mount_config_t mount_config = {
        .format_if_mount_failed = false,
        .max_files = 6,
        .allocation_unit_size = 16 * 1024,
    };

    sdmmc_host_t host = SDMMC_HOST_DEFAULT();
    host.max_freq_khz = SDMMC_FREQ_DEFAULT;

    sdmmc_slot_config_t slot_config = SDMMC_SLOT_CONFIG_DEFAULT();
    slot_config.width = 1;
    slot_config.clk = GPIO_NUM_14;
    slot_config.cmd = GPIO_NUM_17;
    slot_config.d0 = GPIO_NUM_16;
    slot_config.flags |= SDMMC_SLOT_FLAG_INTERNAL_PULLUP;

    esp_err_t err = st77916_sd_persistent_mount_compat(
        mount_point,
        &host,
        &slot_config,
        &mount_config,
        &card
    );

    if (err == ESP_OK && card != NULL) {
        st77916_sd_persistent_release_compat(mount_point, card);
        return true;
    }

    return false;
}

bool st77916_sd_persistent_is_ready(void) {
    return s_sd_persistent_mount_ready && s_sd_persistent_card != NULL;
}

uint32_t st77916_sd_persistent_mount_count(void) {
    return s_sd_persistent_mount_count;
}

uint32_t st77916_sd_owner_busy_count(void) {
    return s_sd_owner_busy_count;
}

#define esp_vfs_fat_sdmmc_mount st77916_sd_persistent_mount_compat
#define esp_vfs_fat_sdcard_unmount st77916_sd_persistent_release_compat

static const st77916_lcd_init_cmd_t vendor_specific_init_new[] = {
    {0xF0, (uint8_t[]){0x28}, 1, 0},
    {0xF2, (uint8_t[]){0x28}, 1, 0},
    {0x73, (uint8_t[]){0xF0}, 1, 0},
    {0x7C, (uint8_t[]){0xD1}, 1, 0},
    {0x83, (uint8_t[]){0xE0}, 1, 0},
    {0x84, (uint8_t[]){0x61}, 1, 0},
    {0xF2, (uint8_t[]){0x82}, 1, 0},
    {0xF0, (uint8_t[]){0x00}, 1, 0},
    {0xF0, (uint8_t[]){0x01}, 1, 0},
    {0xF1, (uint8_t[]){0x01}, 1, 0},
    {0xB0, (uint8_t[]){0x56}, 1, 0},
    {0xB1, (uint8_t[]){0x4D}, 1, 0},
    {0xB2, (uint8_t[]){0x24}, 1, 0},
    {0xB4, (uint8_t[]){0x87}, 1, 0},
    {0xB5, (uint8_t[]){0x44}, 1, 0},
    {0xB6, (uint8_t[]){0x8B}, 1, 0},
    {0xB7, (uint8_t[]){0x40}, 1, 0},
    {0xB8, (uint8_t[]){0x86}, 1, 0},
    {0xBA, (uint8_t[]){0x00}, 1, 0},
    {0xBB, (uint8_t[]){0x08}, 1, 0},
    {0xBC, (uint8_t[]){0x08}, 1, 0},
    {0xBD, (uint8_t[]){0x00}, 1, 0},
    {0xC0, (uint8_t[]){0x80}, 1, 0},
    {0xC1, (uint8_t[]){0x10}, 1, 0},
    {0xC2, (uint8_t[]){0x37}, 1, 0},
    {0xC3, (uint8_t[]){0x80}, 1, 0},
    {0xC4, (uint8_t[]){0x10}, 1, 0},
    {0xC5, (uint8_t[]){0x37}, 1, 0},
    {0xC6, (uint8_t[]){0xA9}, 1, 0},
    {0xC7, (uint8_t[]){0x41}, 1, 0},
    {0xC8, (uint8_t[]){0x01}, 1, 0},
    {0xC9, (uint8_t[]){0xA9}, 1, 0},
    {0xCA, (uint8_t[]){0x41}, 1, 0},
    {0xCB, (uint8_t[]){0x01}, 1, 0},
    {0xD0, (uint8_t[]){0x91}, 1, 0},
    {0xD1, (uint8_t[]){0x68}, 1, 0},
    {0xD2, (uint8_t[]){0x68}, 1, 0},
    {0xF5, (uint8_t[]){0x00, 0xA5}, 2, 0},
    {0xDD, (uint8_t[]){0x4F}, 1, 0},
    {0xDE, (uint8_t[]){0x4F}, 1, 0},
    {0xF1, (uint8_t[]){0x10}, 1, 0},
    {0xF0, (uint8_t[]){0x00}, 1, 0},
    {0xF0, (uint8_t[]){0x02}, 1, 0},
    {0xE0, (uint8_t[]){0xF0, 0x0A, 0x10, 0x09, 0x09, 0x36, 0x35, 0x33, 0x4A, 0x29, 0x15, 0x15, 0x2E, 0x34}, 14, 0},
    {0xE1, (uint8_t[]){0xF0, 0x0A, 0x0F, 0x08, 0x08, 0x05, 0x34, 0x33, 0x4A, 0x39, 0x15, 0x15, 0x2D, 0x33}, 14, 0},
    {0xF0, (uint8_t[]){0x10}, 1, 0},
    {0xF3, (uint8_t[]){0x10}, 1, 0},
    {0xE0, (uint8_t[]){0x07}, 1, 0},
    {0xE1, (uint8_t[]){0x00}, 1, 0},
    {0xE2, (uint8_t[]){0x00}, 1, 0},
    {0xE3, (uint8_t[]){0x00}, 1, 0},
    {0xE4, (uint8_t[]){0xE0}, 1, 0},
    {0xE5, (uint8_t[]){0x06}, 1, 0},
    {0xE6, (uint8_t[]){0x21}, 1, 0},
    {0xE7, (uint8_t[]){0x01}, 1, 0},
    {0xE8, (uint8_t[]){0x05}, 1, 0},
    {0xE9, (uint8_t[]){0x02}, 1, 0},
    {0xEA, (uint8_t[]){0xDA}, 1, 0},
    {0xEB, (uint8_t[]){0x00}, 1, 0},
    {0xEC, (uint8_t[]){0x00}, 1, 0},
    {0xED, (uint8_t[]){0x0F}, 1, 0},
    {0xEE, (uint8_t[]){0x00}, 1, 0},
    {0xEF, (uint8_t[]){0x00}, 1, 0},
    {0xF8, (uint8_t[]){0x00}, 1, 0},
    {0xF9, (uint8_t[]){0x00}, 1, 0},
    {0xFA, (uint8_t[]){0x00}, 1, 0},
    {0xFB, (uint8_t[]){0x00}, 1, 0},
    {0xFC, (uint8_t[]){0x00}, 1, 0},
    {0xFD, (uint8_t[]){0x00}, 1, 0},
    {0xFE, (uint8_t[]){0x00}, 1, 0},
    {0xFF, (uint8_t[]){0x00}, 1, 0},
    {0x60, (uint8_t[]){0x40}, 1, 0},
    {0x61, (uint8_t[]){0x04}, 1, 0},
    {0x62, (uint8_t[]){0x00}, 1, 0},
    {0x63, (uint8_t[]){0x42}, 1, 0},
    {0x64, (uint8_t[]){0xD9}, 1, 0},
    {0x65, (uint8_t[]){0x00}, 1, 0},
    {0x66, (uint8_t[]){0x00}, 1, 0},
    {0x67, (uint8_t[]){0x00}, 1, 0},
    {0x68, (uint8_t[]){0x00}, 1, 0},
    {0x69, (uint8_t[]){0x00}, 1, 0},
    {0x6A, (uint8_t[]){0x00}, 1, 0},
    {0x6B, (uint8_t[]){0x00}, 1, 0},
    {0x70, (uint8_t[]){0x40}, 1, 0},
    {0x71, (uint8_t[]){0x03}, 1, 0},
    {0x72, (uint8_t[]){0x00}, 1, 0},
    {0x73, (uint8_t[]){0x42}, 1, 0},
    {0x74, (uint8_t[]){0xD8}, 1, 0},
    {0x75, (uint8_t[]){0x00}, 1, 0},
    {0x76, (uint8_t[]){0x00}, 1, 0},
    {0x77, (uint8_t[]){0x00}, 1, 0},
    {0x78, (uint8_t[]){0x00}, 1, 0},
    {0x79, (uint8_t[]){0x00}, 1, 0},
    {0x7A, (uint8_t[]){0x00}, 1, 0},
    {0x7B, (uint8_t[]){0x00}, 1, 0},
    {0x80, (uint8_t[]){0x48}, 1, 0},
    {0x81, (uint8_t[]){0x00}, 1, 0},
    {0x82, (uint8_t[]){0x06}, 1, 0},
    {0x83, (uint8_t[]){0x02}, 1, 0},
    {0x84, (uint8_t[]){0xD6}, 1, 0},
    {0x85, (uint8_t[]){0x04}, 1, 0},
    {0x86, (uint8_t[]){0x00}, 1, 0},
    {0x87, (uint8_t[]){0x00}, 1, 0},
    {0x88, (uint8_t[]){0x48}, 1, 0},
    {0x89, (uint8_t[]){0x00}, 1, 0},
    {0x8A, (uint8_t[]){0x08}, 1, 0},
    {0x8B, (uint8_t[]){0x02}, 1, 0},
    {0x8C, (uint8_t[]){0xD8}, 1, 0},
    {0x8D, (uint8_t[]){0x04}, 1, 0},
    {0x8E, (uint8_t[]){0x00}, 1, 0},
    {0x8F, (uint8_t[]){0x00}, 1, 0},
    {0x90, (uint8_t[]){0x48}, 1, 0},
    {0x91, (uint8_t[]){0x00}, 1, 0},
    {0x92, (uint8_t[]){0x0A}, 1, 0},
    {0x93, (uint8_t[]){0x02}, 1, 0},
    {0x94, (uint8_t[]){0xDA}, 1, 0},
    {0x95, (uint8_t[]){0x04}, 1, 0},
    {0x96, (uint8_t[]){0x00}, 1, 0},
    {0x97, (uint8_t[]){0x00}, 1, 0},
    {0x98, (uint8_t[]){0x48}, 1, 0},
    {0x99, (uint8_t[]){0x00}, 1, 0},
    {0x9A, (uint8_t[]){0x0C}, 1, 0},
    {0x9B, (uint8_t[]){0x02}, 1, 0},
    {0x9C, (uint8_t[]){0xDC}, 1, 0},
    {0x9D, (uint8_t[]){0x04}, 1, 0},
    {0x9E, (uint8_t[]){0x00}, 1, 0},
    {0x9F, (uint8_t[]){0x00}, 1, 0},
    {0xA0, (uint8_t[]){0x48}, 1, 0},
    {0xA1, (uint8_t[]){0x00}, 1, 0},
    {0xA2, (uint8_t[]){0x05}, 1, 0},
    {0xA3, (uint8_t[]){0x02}, 1, 0},
    {0xA4, (uint8_t[]){0xD5}, 1, 0},
    {0xA5, (uint8_t[]){0x04}, 1, 0},
    {0xA6, (uint8_t[]){0x00}, 1, 0},
    {0xA7, (uint8_t[]){0x00}, 1, 0},
    {0xA8, (uint8_t[]){0x48}, 1, 0},
    {0xA9, (uint8_t[]){0x00}, 1, 0},
    {0xAA, (uint8_t[]){0x07}, 1, 0},
    {0xAB, (uint8_t[]){0x02}, 1, 0},
    {0xAC, (uint8_t[]){0xD7}, 1, 0},
    {0xAD, (uint8_t[]){0x04}, 1, 0},
    {0xAE, (uint8_t[]){0x00}, 1, 0},
    {0xAF, (uint8_t[]){0x00}, 1, 0},
    {0xB0, (uint8_t[]){0x48}, 1, 0},
    {0xB1, (uint8_t[]){0x00}, 1, 0},
    {0xB2, (uint8_t[]){0x09}, 1, 0},
    {0xB3, (uint8_t[]){0x02}, 1, 0},
    {0xB4, (uint8_t[]){0xD9}, 1, 0},
    {0xB5, (uint8_t[]){0x04}, 1, 0},
    {0xB6, (uint8_t[]){0x00}, 1, 0},
    {0xB7, (uint8_t[]){0x00}, 1, 0},
    {0xB8, (uint8_t[]){0x48}, 1, 0},
    {0xB9, (uint8_t[]){0x00}, 1, 0},
    {0xBA, (uint8_t[]){0x0B}, 1, 0},
    {0xBB, (uint8_t[]){0x02}, 1, 0},
    {0xBC, (uint8_t[]){0xDB}, 1, 0},
    {0xBD, (uint8_t[]){0x04}, 1, 0},
    {0xBE, (uint8_t[]){0x00}, 1, 0},
    {0xBF, (uint8_t[]){0x00}, 1, 0},
    {0xC0, (uint8_t[]){0x10}, 1, 0},
    {0xC1, (uint8_t[]){0x47}, 1, 0},
    {0xC2, (uint8_t[]){0x56}, 1, 0},
    {0xC3, (uint8_t[]){0x65}, 1, 0},
    {0xC4, (uint8_t[]){0x74}, 1, 0},
    {0xC5, (uint8_t[]){0x88}, 1, 0},
    {0xC6, (uint8_t[]){0x99}, 1, 0},
    {0xC7, (uint8_t[]){0x01}, 1, 0},
    {0xC8, (uint8_t[]){0xBB}, 1, 0},
    {0xC9, (uint8_t[]){0xAA}, 1, 0},
    {0xD0, (uint8_t[]){0x10}, 1, 0},
    {0xD1, (uint8_t[]){0x47}, 1, 0},
    {0xD2, (uint8_t[]){0x56}, 1, 0},
    {0xD3, (uint8_t[]){0x65}, 1, 0},
    {0xD4, (uint8_t[]){0x74}, 1, 0},
    {0xD5, (uint8_t[]){0x88}, 1, 0},
    {0xD6, (uint8_t[]){0x99}, 1, 0},
    {0xD7, (uint8_t[]){0x01}, 1, 0},
    {0xD8, (uint8_t[]){0xBB}, 1, 0},
    {0xD9, (uint8_t[]){0xAA}, 1, 0},
    {0xF3, (uint8_t[]){0x01}, 1, 0},
    {0xF0, (uint8_t[]){0x00}, 1, 0},
    {0x21, (uint8_t[]){0x00}, 1, 0},
    {0x11, (uint8_t[]){0x00}, 1, 120},
    {0x29, (uint8_t[]){0x00}, 1, 0},
};

static esp_lcd_panel_handle_t s_panel = NULL;
static bool s_ready = false;

static void byteswap_rgb565(uint16_t *buf, uint32_t px_count) {
    for (uint32_t i = 0; i < px_count; i++) {
        uint16_t v = buf[i];
        buf[i] = (uint16_t)(((v >> 8) & 0x00FFu) | ((v << 8) & 0xFF00u));
    }
}


#ifndef ST77916_ADC_ATTEN
#define ST77916_ADC_ATTEN ADC_ATTEN_DB_12
#endif

static int32_t st77916_adc_raw_to_mv_fallback(int raw) {
    if (raw <= 0) {
        return 0;
    }
    return (int32_t) (((int64_t) raw * 3300 + 2047) / 4095);
}


bool st77916_gpio_input_pullup(int32_t gpio_num) {
    if (gpio_num < 0 || gpio_num > 48) {
        return false;
    }

    gpio_config_t io_conf = {
        .pin_bit_mask = (1ULL << ((uint32_t) gpio_num)),
        .mode = GPIO_MODE_INPUT,
        .pull_up_en = GPIO_PULLUP_ENABLE,
        .pull_down_en = GPIO_PULLDOWN_DISABLE,
        .intr_type = GPIO_INTR_DISABLE,
    };

    return gpio_config(&io_conf) == ESP_OK;
}

int32_t st77916_gpio_get_level(int32_t gpio_num) {
    if (gpio_num < 0 || gpio_num > 48) {
        return -1;
    }

    return (int32_t) gpio_get_level((gpio_num_t) gpio_num);
}

bool st77916_adc1_gpio8_oneshot_probe(uint32_t sample_count, st77916_adc_probe_result_t *out_result) {
    if (out_result == NULL || sample_count == 0) {
        return false;
    }

    memset(out_result, 0, sizeof(*out_result));
    out_result->raw_min = -1;
    out_result->raw_max = -1;
    out_result->raw_avg = -1;
    out_result->mv_avg = -1;
    out_result->status = ESP_OK;

    adc_oneshot_unit_handle_t adc1_handle = NULL;
    adc_oneshot_unit_init_cfg_t init_config = {
        .unit_id = ADC_UNIT_1,
        .ulp_mode = ADC_ULP_MODE_DISABLE,
    };

    esp_err_t err = adc_oneshot_new_unit(&init_config, &adc1_handle);
    if (err != ESP_OK || adc1_handle == NULL) {
        out_result->status = (int32_t) err;
        out_result->error_count = sample_count;
        return false;
    }

    adc_oneshot_chan_cfg_t chan_config = {
        .bitwidth = ADC_BITWIDTH_DEFAULT,
        .atten = ST77916_ADC_ATTEN,
    };

    err = adc_oneshot_config_channel(adc1_handle, ADC_CHANNEL_7, &chan_config);
    if (err != ESP_OK) {
        out_result->status = (int32_t) err;
        out_result->error_count = sample_count;
        adc_oneshot_del_unit(adc1_handle);
        return false;
    }

    adc_cali_handle_t cali_handle = NULL;
    bool cali_available = false;

#if ADC_CALI_SCHEME_CURVE_FITTING_SUPPORTED
    adc_cali_curve_fitting_config_t cali_config = {
        .unit_id = ADC_UNIT_1,
        .atten = ST77916_ADC_ATTEN,
        .bitwidth = ADC_BITWIDTH_DEFAULT,
    };
    if (adc_cali_create_scheme_curve_fitting(&cali_config, &cali_handle) == ESP_OK) {
        cali_available = true;
    }
#elif ADC_CALI_SCHEME_LINE_FITTING_SUPPORTED
    adc_cali_line_fitting_config_t cali_config = {
        .unit_id = ADC_UNIT_1,
        .atten = ST77916_ADC_ATTEN,
        .bitwidth = ADC_BITWIDTH_DEFAULT,
    };
    if (adc_cali_create_scheme_line_fitting(&cali_config, &cali_handle) == ESP_OK) {
        cali_available = true;
    }
#endif

    int raw_min = INT32_MAX;
    int raw_max = 0;
    int64_t raw_sum = 0;
    int64_t mv_sum = 0;
    uint32_t valid_count = 0;
    uint32_t zero_count = 0;
    uint32_t error_count = 0;

    for (uint32_t i = 0; i < sample_count; i++) {
        int raw = 0;
        err = adc_oneshot_read(adc1_handle, ADC_CHANNEL_7, &raw);
        if (err == ESP_OK) {
            if (raw > 0) {
                valid_count++;
                raw_sum += raw;
                if (raw < raw_min) {
                    raw_min = raw;
                }
                if (raw > raw_max) {
                    raw_max = raw;
                }

                int voltage_mv = 0;
                if (cali_available && adc_cali_raw_to_voltage(cali_handle, raw, &voltage_mv) == ESP_OK) {
                    mv_sum += voltage_mv;
                } else {
                    mv_sum += st77916_adc_raw_to_mv_fallback(raw);
                }
            } else {
                zero_count++;
            }
        } else {
            error_count++;
            out_result->status = (int32_t) err;
        }

        if (i + 1 < sample_count) {
            vTaskDelay(pdMS_TO_TICKS(2));
        }
    }

    if (valid_count > 0) {
        out_result->raw_min = raw_min;
        out_result->raw_max = raw_max;
        out_result->raw_avg = (int32_t) (raw_sum / valid_count);
        out_result->mv_avg = (int32_t) (mv_sum / valid_count);
    }

    out_result->valid_count = valid_count;
    out_result->zero_count = zero_count;
    out_result->error_count = error_count;
    out_result->calibrated = cali_available ? 1 : 0;

#if ADC_CALI_SCHEME_CURVE_FITTING_SUPPORTED
    if (cali_available) {
        adc_cali_delete_scheme_curve_fitting(cali_handle);
    }
#elif ADC_CALI_SCHEME_LINE_FITTING_SUPPORTED
    if (cali_available) {
        adc_cali_delete_scheme_line_fitting(cali_handle);
    }
#endif

    adc_oneshot_del_unit(adc1_handle);
    return valid_count > 0;
}


bool st77916_panel_init(void) {
    if (s_ready) {
        return true;
    }

    static const spi_bus_config_t host_config = {
        .data0_io_num = ESP_PANEL_LCD_SPI_IO_DATA0,
        .data1_io_num = ESP_PANEL_LCD_SPI_IO_DATA1,
        .sclk_io_num = ESP_PANEL_LCD_SPI_IO_SCK,
        .data2_io_num = ESP_PANEL_LCD_SPI_IO_DATA2,
        .data3_io_num = ESP_PANEL_LCD_SPI_IO_DATA3,
        .data4_io_num = -1,
        .data5_io_num = -1,
        .data6_io_num = -1,
        .data7_io_num = -1,
        .max_transfer_sz = ESP_PANEL_HOST_SPI_MAX_TRANSFER_SIZE,
        .flags = SPICOMMON_BUSFLAG_MASTER,
        .intr_flags = 0,
    };

    if (spi_bus_initialize(ESP_PANEL_HOST_SPI_ID_DEFAULT, &host_config, SPI_DMA_CH_AUTO) != ESP_OK) {
        return false;
    }

    esp_lcd_panel_io_spi_config_t io_config = {
        .cs_gpio_num = ESP_PANEL_LCD_SPI_IO_CS,
        .dc_gpio_num = -1,
        .spi_mode = ESP_PANEL_LCD_SPI_MODE,
        .pclk_hz = 5 * 1000 * 1000,
        .trans_queue_depth = ESP_PANEL_LCD_SPI_TRANS_QUEUE_SZ,
        .on_color_trans_done = NULL,
        .user_ctx = NULL,
        .lcd_cmd_bits = ESP_PANEL_LCD_SPI_CMD_BITS,
        .lcd_param_bits = ESP_PANEL_LCD_SPI_PARAM_BITS,
        .flags = {
            .dc_low_on_data = 0,
            .octal_mode = 0,
            .quad_mode = 1,
            .sio_mode = 0,
            .lsb_first = 0,
            .cs_high_active = 0,
        },
    };

    esp_lcd_panel_io_handle_t io_handle = NULL;
    if (esp_lcd_new_panel_io_spi((esp_lcd_spi_bus_handle_t)ESP_PANEL_HOST_SPI_ID_DEFAULT, &io_config, &io_handle) != ESP_OK) {
        return false;
    }

    st77916_vendor_config_t vendor_config = {
        .flags = {
            .use_qspi_interface = 1,
        },
    };

    int lcd_cmd = 0x04;
    uint8_t register_data[4] = {0};
    size_t param_size = sizeof(register_data);
    lcd_cmd &= 0xff;
    lcd_cmd <<= 8;
    lcd_cmd |= LCD_OPCODE_READ_CMD << 24;

    (void)esp_lcd_panel_io_rx_param(io_handle, lcd_cmd, register_data, param_size);

    io_config.pclk_hz = ESP_PANEL_LCD_SPI_CLK_HZ;
    if (esp_lcd_new_panel_io_spi((esp_lcd_spi_bus_handle_t)ESP_PANEL_HOST_SPI_ID_DEFAULT, &io_config, &io_handle) != ESP_OK) {
        return false;
    }

    if (register_data[0] == 0x00 &&
        register_data[1] == 0x02 &&
        register_data[2] == 0x7F &&
        register_data[3] == 0x7F) {
        vendor_config.init_cmds = vendor_specific_init_new;
        vendor_config.init_cmds_size =
            sizeof(vendor_specific_init_new) / sizeof(st77916_lcd_init_cmd_t);
    }

    esp_lcd_panel_dev_config_t panel_config = {
        .reset_gpio_num = EXAMPLE_LCD_PIN_NUM_RST,
        .rgb_ele_order = LCD_RGB_ELEMENT_ORDER_RGB,
        .data_endian = LCD_RGB_DATA_ENDIAN_BIG,
        .bits_per_pixel = LCD_COLOR_BITS,
        .flags = {
            .reset_active_high = 0,
        },
        .vendor_config = (void *)&vendor_config,
    };

    if (esp_lcd_new_panel_st77916(io_handle, &panel_config, &s_panel) != ESP_OK) {
        return false;
    }
    if (esp_lcd_panel_reset(s_panel) != ESP_OK) {
        return false;
    }
    if (esp_lcd_panel_init(s_panel) != ESP_OK) {
        return false;
    }
    if (esp_lcd_panel_disp_on_off(s_panel, true) != ESP_OK) {
        return false;
    }

    s_ready = true;
    return true;
}

bool st77916_panel_draw_rgb565(uint16_t x0, uint16_t y0, uint16_t x1, uint16_t y1, uint16_t *color) {
    if (!s_ready || s_panel == NULL || color == NULL) {
        return false;
    }

    uint32_t size = (uint32_t)(x1 - x0 + 1u) * (uint32_t)(y1 - y0 + 1u);
    byteswap_rgb565(color, size);

    x1 = x1 + 1;
    y1 = y1 + 1;
    if (x1 > LCD_WIDTH) {
        x1 = LCD_WIDTH;
    }
    if (y1 > LCD_HEIGHT) {
        y1 = LCD_HEIGHT;
    }

    return esp_lcd_panel_draw_bitmap(s_panel, x0, y0, x1, y1, color) == ESP_OK;
}
bool st77916_probe_sd_capacity_mb(bool *out_present, uint32_t *out_capacity_mb) {
    if (out_present) {
        *out_present = false;
    }
    if (out_capacity_mb) {
        *out_capacity_mb = 0;
    }

    const char *mount_point = "/sdcard";
    sdmmc_card_t *card = NULL;

    esp_vfs_fat_sdmmc_mount_config_t mount_config = {
        .format_if_mount_failed = false,
        .max_files = 4,
        .allocation_unit_size = 16 * 1024,
    };

    sdmmc_host_t host = SDMMC_HOST_DEFAULT();
    host.max_freq_khz = SDMMC_FREQ_DEFAULT;

    sdmmc_slot_config_t slot_config = SDMMC_SLOT_CONFIG_DEFAULT();
    slot_config.width = 1;
    slot_config.clk = GPIO_NUM_14;
    slot_config.cmd = GPIO_NUM_17;
    slot_config.d0 = GPIO_NUM_16;
    slot_config.flags |= SDMMC_SLOT_FLAG_INTERNAL_PULLUP;

    esp_err_t err = esp_vfs_fat_sdmmc_mount(
        mount_point,
        &host,
        &slot_config,
        &mount_config,
        &card
    );

    if (err != ESP_OK || card == NULL) {
        return false;
    }

    if (out_present) {
        *out_present = true;
    }

    if (out_capacity_mb) {
        uint64_t total_bytes =
            ((uint64_t) card->csd.capacity) *
            ((uint64_t) card->csd.sector_size);
        *out_capacity_mb = (uint32_t) (total_bytes / (1024ULL * 1024ULL));
    }

    esp_vfs_fat_sdcard_unmount(mount_point, card);
    return true;
}


bool st77916_probe_sd_space_mb(bool *out_present, uint32_t *out_total_mb, uint32_t *out_free_mb) {
    if (out_present) {
        *out_present = false;
    }
    if (out_total_mb) {
        *out_total_mb = 0;
    }
    if (out_free_mb) {
        *out_free_mb = 0;
    }

    const char *mount_point = "/sdcard";
    sdmmc_card_t *card = NULL;

    esp_vfs_fat_sdmmc_mount_config_t mount_config = {
        .format_if_mount_failed = false,
        .max_files = 4,
        .allocation_unit_size = 16 * 1024,
    };

    sdmmc_host_t host = SDMMC_HOST_DEFAULT();
    host.max_freq_khz = SDMMC_FREQ_DEFAULT;

    sdmmc_slot_config_t slot_config = SDMMC_SLOT_CONFIG_DEFAULT();
    slot_config.width = 1;
    slot_config.clk = GPIO_NUM_14;
    slot_config.cmd = GPIO_NUM_17;
    slot_config.d0 = GPIO_NUM_16;
    slot_config.flags |= SDMMC_SLOT_FLAG_INTERNAL_PULLUP;

    esp_err_t err = esp_vfs_fat_sdmmc_mount(
        mount_point,
        &host,
        &slot_config,
        &mount_config,
        &card
    );

    if (err != ESP_OK || card == NULL) {
        return false;
    }

    if (out_present) {
        *out_present = true;
    }

    uint32_t sector_size = card->csd.sector_size;
    if (sector_size == 0) {
        sector_size = 512;
    }

    uint64_t total_bytes =
        ((uint64_t) card->csd.capacity) *
        ((uint64_t) sector_size);

    uint64_t free_bytes = 0;

    FATFS *fs = NULL;
    DWORD free_clusters = 0;
    FRESULT fr = f_getfree("0:", &free_clusters, &fs);
    if (fr != FR_OK || fs == NULL) {
        fr = f_getfree(mount_point, &free_clusters, &fs);
    }

    if (fr == FR_OK && fs != NULL) {
        uint64_t cluster_size_sectors = (uint64_t) fs->csize;
        uint64_t total_clusters = 0;
        if (fs->n_fatent > 2) {
            total_clusters = (uint64_t) (fs->n_fatent - 2);
        }

        uint64_t fs_total_bytes =
            total_clusters *
            cluster_size_sectors *
            ((uint64_t) sector_size);
        free_bytes =
            ((uint64_t) free_clusters) *
            cluster_size_sectors *
            ((uint64_t) sector_size);

        if (fs_total_bytes > 0) {
            total_bytes = fs_total_bytes;
        }
    }

    if (out_total_mb) {
        *out_total_mb = (uint32_t) (total_bytes / (1024ULL * 1024ULL));
    }
    if (out_free_mb) {
        *out_free_mb = (uint32_t) (free_bytes / (1024ULL * 1024ULL));
    }

    esp_vfs_fat_sdcard_unmount(mount_point, card);
    return true;
}

int32_t st77916_read_sd_wifi_txt(uint8_t *out_buf, uint32_t out_len) {
    if (out_buf == NULL || out_len < 2) {
        return -1;
    }

    out_buf[0] = 0;

    const char *mount_point = "/sdcard";
    sdmmc_card_t *card = NULL;

    esp_vfs_fat_sdmmc_mount_config_t mount_config = {
        .format_if_mount_failed = false,
        .max_files = 4,
        .allocation_unit_size = 16 * 1024,
    };

    sdmmc_host_t host = SDMMC_HOST_DEFAULT();
    host.max_freq_khz = SDMMC_FREQ_DEFAULT;

    sdmmc_slot_config_t slot_config = SDMMC_SLOT_CONFIG_DEFAULT();
    slot_config.width = 1;
    slot_config.clk = GPIO_NUM_14;
    slot_config.cmd = GPIO_NUM_17;
    slot_config.d0 = GPIO_NUM_16;
    slot_config.flags |= SDMMC_SLOT_FLAG_INTERNAL_PULLUP;

    esp_err_t err = esp_vfs_fat_sdmmc_mount(
        mount_point,
        &host,
        &slot_config,
        &mount_config,
        &card
    );

    if (err != ESP_OK || card == NULL) {
        return -2;
    }

    const char *paths[] = {
        "/sdcard/WIFI.TXT",
        "/sdcard/wifi.txt",
        "/sdcard/WIFI.CFG",
        "/sdcard/wifi.cfg",
    };

    FILE *fp = NULL;
    for (size_t i = 0; i < sizeof(paths) / sizeof(paths[0]); ++i) {
        fp = fopen(paths[i], "rb");
        if (fp != NULL) {
            break;
        }
    }

    if (fp == NULL) {
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        return -3;
    }

    size_t n = fread(out_buf, 1, out_len - 1, fp);
    fclose(fp);
    out_buf[n] = 0;

    esp_vfs_fat_sdcard_unmount(mount_point, card);

    if (n == 0) {
        return -4;
    }

    return (int32_t) n;
}


static bool s_st77916_sntp_started = false;

void st77916_time_configure_eastern(void) {
    setenv("TZ", "EST5EDT,M3.2.0/2,M11.1.0/2", 1);
    tzset();
}

void st77916_sntp_start(void) {
    if (s_st77916_sntp_started) {
        return;
    }

    esp_sntp_setoperatingmode(SNTP_OPMODE_POLL);
    esp_sntp_setservername(0, "pool.ntp.org");
    esp_sntp_setservername(1, "time.google.com");
    esp_sntp_init();
    s_st77916_sntp_started = true;
}

bool st77916_sntp_is_synced(void) {
    if (!s_st77916_sntp_started) {
        return false;
    }

    return esp_sntp_get_sync_status() == SNTP_SYNC_STATUS_COMPLETED;
}

int64_t st77916_time_epoch(void) {
    time_t now = 0;
    time(&now);
    return (int64_t) now;
}

bool st77916_get_local_datetime(st77916_datetime_t *out_dt) {
    if (out_dt == NULL) {
        return false;
    }

    time_t now = 0;
    time(&now);
    if (now < 1700000000) {
        return false;
    }

    struct tm local_tm;
    if (localtime_r(&now, &local_tm) == NULL) {
        return false;
    }

    int year = local_tm.tm_year + 1900;
    if (year < 2000 || year > 2099) {
        return false;
    }

    out_dt->second = (uint8_t) local_tm.tm_sec;
    out_dt->minute = (uint8_t) local_tm.tm_min;
    out_dt->hour = (uint8_t) local_tm.tm_hour;
    out_dt->day = (uint8_t) local_tm.tm_mday;
    out_dt->month = (uint8_t) (local_tm.tm_mon + 1);
    out_dt->year = (uint8_t) (year - 2000);

    return true;
}


int32_t st77916_http_get(const char *url, uint8_t *out_buf, uint32_t out_len) {
    if (url == NULL || out_buf == NULL || out_len < 2) {
        return -1;
    }

    out_buf[0] = 0;

    esp_http_client_config_t config = {
        .url = url,
        .timeout_ms = 6000,
        .buffer_size = 1024,
        .buffer_size_tx = 512,
        .disable_auto_redirect = false,
    };

    esp_http_client_handle_t client = esp_http_client_init(&config);
    if (client == NULL) {
        return -2;
    }

    esp_err_t err = esp_http_client_open(client, 0);
    if (err != ESP_OK) {
        esp_http_client_cleanup(client);
        return -3;
    }

    int64_t content_length = esp_http_client_fetch_headers(client);
    (void) content_length;

    int read_len = esp_http_client_read_response(client, (char *) out_buf, (int) out_len - 1);
    int status_code = esp_http_client_get_status_code(client);

    esp_http_client_close(client);
    esp_http_client_cleanup(client);

    if (read_len < 0) {
        out_buf[0] = 0;
        return -4;
    }

    if (status_code < 200 || status_code >= 300) {
        out_buf[0] = 0;
        return -1000 - status_code;
    }

    if ((uint32_t) read_len >= out_len) {
        read_len = (int) out_len - 1;
    }

    out_buf[read_len] = 0;
    return read_len;
}


int32_t st77916_read_sd_weather_txt(uint8_t *out_buf, uint32_t out_len) {
    if (out_buf == NULL || out_len < 2) {
        return -1;
    }

    out_buf[0] = 0;

    const char *mount_point = "/sdcard";
    sdmmc_card_t *card = NULL;

    esp_vfs_fat_sdmmc_mount_config_t mount_config = {
        .format_if_mount_failed = false,
        .max_files = 4,
        .allocation_unit_size = 16 * 1024,
    };

    sdmmc_host_t host = SDMMC_HOST_DEFAULT();
    host.max_freq_khz = SDMMC_FREQ_DEFAULT;

    sdmmc_slot_config_t slot_config = SDMMC_SLOT_CONFIG_DEFAULT();
    slot_config.width = 1;
    slot_config.clk = GPIO_NUM_14;
    slot_config.cmd = GPIO_NUM_17;
    slot_config.d0 = GPIO_NUM_16;
    slot_config.flags |= SDMMC_SLOT_FLAG_INTERNAL_PULLUP;

    esp_err_t err = esp_vfs_fat_sdmmc_mount(
        mount_point,
        &host,
        &slot_config,
        &mount_config,
        &card
    );

    if (err != ESP_OK || card == NULL) {
        return -2;
    }

    const char *paths[] = {
        "/sdcard/WEATHER.TXT",
        "/sdcard/weather.txt",
        "/sdcard/WEATHER.CFG",
        "/sdcard/weather.cfg",
    };

    FILE *fp = NULL;
    for (size_t i = 0; i < sizeof(paths) / sizeof(paths[0]); ++i) {
        fp = fopen(paths[i], "rb");
        if (fp != NULL) {
            break;
        }
    }

    if (fp == NULL) {
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        return -3;
    }

    size_t n = fread(out_buf, 1, out_len - 1, fp);
    fclose(fp);
    out_buf[n] = 0;

    esp_vfs_fat_sdcard_unmount(mount_point, card);

    if (n == 0) {
        return -4;
    }

    return (int32_t) n;
}


int32_t st77916_read_sd_battery_txt(uint8_t *out_buf, uint32_t out_len) {
    if (out_buf == NULL || out_len < 2) {
        return -1;
    }

    out_buf[0] = 0;

    const char *mount_point = "/sdcard";
    sdmmc_card_t *card = NULL;

    esp_vfs_fat_sdmmc_mount_config_t mount_config = {
        .format_if_mount_failed = false,
        .max_files = 4,
        .allocation_unit_size = 16 * 1024,
    };

    sdmmc_host_t host = SDMMC_HOST_DEFAULT();
    host.max_freq_khz = SDMMC_FREQ_DEFAULT;

    sdmmc_slot_config_t slot_config = SDMMC_SLOT_CONFIG_DEFAULT();
    slot_config.width = 1;
    slot_config.clk = GPIO_NUM_14;
    slot_config.cmd = GPIO_NUM_17;
    slot_config.d0 = GPIO_NUM_16;
    slot_config.flags |= SDMMC_SLOT_FLAG_INTERNAL_PULLUP;

    esp_err_t err = esp_vfs_fat_sdmmc_mount(
        mount_point,
        &host,
        &slot_config,
        &mount_config,
        &card
    );

    if (err != ESP_OK || card == NULL) {
        return -2;
    }

    const char *paths[] = {
        "/sdcard/BATTERY.TXT",
        "/sdcard/battery.txt",
        "/sdcard/BATTERY.CFG",
        "/sdcard/battery.cfg",
    };

    FILE *fp = NULL;
    for (size_t i = 0; i < sizeof(paths) / sizeof(paths[0]); ++i) {
        fp = fopen(paths[i], "rb");
        if (fp != NULL) {
            break;
        }
    }

    if (fp == NULL) {
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        return -3;
    }

    size_t n = fread(out_buf, 1, out_len - 1, fp);
    fclose(fp);
    out_buf[n] = 0;

    esp_vfs_fat_sdcard_unmount(mount_point, card);

    if (n == 0) {
        return -4;
    }

    return (int32_t) n;
}



void st77916_configure_runtime_logs(bool debug_enabled) {
    s_runtime_log_debug_enabled = debug_enabled;
    esp_log_level_t normal_level = debug_enabled ? ESP_LOG_INFO : ESP_LOG_WARN;
    esp_log_level_set("gpio", normal_level);
    esp_log_level_set("wifi", normal_level);
    esp_log_level_set("wifi_init", normal_level);
    esp_log_level_set("phy_init", normal_level);
    esp_log_level_set("net80211", normal_level);
    esp_log_level_set("esp_netif_handlers", normal_level);
    esp_log_level_set("sdmmc_common", normal_level);
    esp_log_level_set("vfs_fat_sdmmc", normal_level);
}

int32_t st77916_read_sd_log_txt(uint8_t *out_buf, uint32_t out_len) {
    if (out_buf == NULL || out_len < 2) {
        return -1;
    }

    out_buf[0] = 0;

    const char *mount_point = "/sdcard";
    sdmmc_card_t *card = NULL;

    esp_vfs_fat_sdmmc_mount_config_t mount_config = {
        .format_if_mount_failed = false,
        .max_files = 4,
        .allocation_unit_size = 16 * 1024,
    };

    sdmmc_host_t host = SDMMC_HOST_DEFAULT();
    host.max_freq_khz = SDMMC_FREQ_DEFAULT;

    sdmmc_slot_config_t slot_config = SDMMC_SLOT_CONFIG_DEFAULT();
    slot_config.width = 1;
    slot_config.clk = GPIO_NUM_14;
    slot_config.cmd = GPIO_NUM_17;
    slot_config.d0 = GPIO_NUM_16;
    slot_config.flags |= SDMMC_SLOT_FLAG_INTERNAL_PULLUP;

    esp_err_t err = esp_vfs_fat_sdmmc_mount(
        mount_point,
        &host,
        &slot_config,
        &mount_config,
        &card
    );

    if (err != ESP_OK || card == NULL) {
        return -2;
    }

    const char *paths[] = {
        "/sdcard/LOG.TXT",
        "/sdcard/log.txt",
        "/sdcard/LOG.CFG",
        "/sdcard/log.cfg",
    };

    FILE *fp = NULL;
    for (size_t i = 0; i < sizeof(paths) / sizeof(paths[0]); ++i) {
        fp = fopen(paths[i], "rb");
        if (fp != NULL) {
            break;
        }
    }

    if (fp == NULL) {
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        return -3;
    }

    size_t n = fread(out_buf, 1, out_len - 1, fp);
    fclose(fp);
    out_buf[n] = 0;

    esp_vfs_fat_sdcard_unmount(mount_point, card);

    if (n == 0) {
        return -4;
    }

    return (int32_t) n;
}



int32_t st77916_read_sd_asset_rgb565(const char *asset_name, uint8_t *out_buf, uint32_t out_len) {
    if (asset_name == NULL || out_buf == NULL || out_len == 0) {
        return -1;
    }

    const char *mount_point = "/sdcard";
    sdmmc_card_t *card = NULL;

    esp_vfs_fat_sdmmc_mount_config_t mount_config = {
        .format_if_mount_failed = false,
        .max_files = 4,
        .allocation_unit_size = 16 * 1024,
    };

    sdmmc_host_t host = SDMMC_HOST_DEFAULT();
    host.max_freq_khz = SDMMC_FREQ_DEFAULT;

    sdmmc_slot_config_t slot_config = SDMMC_SLOT_CONFIG_DEFAULT();
    slot_config.width = 1;
    slot_config.clk = GPIO_NUM_14;
    slot_config.cmd = GPIO_NUM_17;
    slot_config.d0 = GPIO_NUM_16;
    slot_config.flags |= SDMMC_SLOT_FLAG_INTERNAL_PULLUP;

    esp_err_t err = esp_vfs_fat_sdmmc_mount(
        mount_point,
        &host,
        &slot_config,
        &mount_config,
        &card
    );

    if (err != ESP_OK || card == NULL) {
        return -2;
    }

    char path[96];
    int written = snprintf(path, sizeof(path), "/sdcard/ASSETS/%s", asset_name);
    if (written <= 0 || written >= (int) sizeof(path)) {
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        return -5;
    }

    FILE *fp = fopen(path, "rb");
    if (fp == NULL) {
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        return -3;
    }

    size_t n = fread(out_buf, 1, out_len, fp);
    fclose(fp);
    esp_vfs_fat_sdcard_unmount(mount_point, card);

    if (n != out_len) {
        return -4;
    }

    return (int32_t) n;
}



static bool st77916_has_mjpeg_extension(const char *name) {
    if (name == NULL) {
        return false;
    }

    const char *dot = strrchr(name, '.');
    if (dot == NULL) {
        return false;
    }

    char ext[8] = {0};
    size_t i = 0;
    for (const char *p = dot; *p != 0 && i < sizeof(ext) - 1; ++p) {
        char ch = *p;
        if (ch >= 'a' && ch <= 'z') {
            ch = (char) (ch - 'a' + 'A');
        }
        ext[i++] = ch;
    }

    return strcmp(ext, ".MJPG") == 0 ||
           strcmp(ext, ".MJPEG") == 0 ||
           strcmp(ext, ".MJP") == 0;
}

static uint32_t st77916_file_size(FILE *fp) {
    long current = ftell(fp);
    if (current < 0) {
        current = 0;
    }

    if (fseek(fp, 0, SEEK_END) != 0) {
        return 0;
    }

    long end = ftell(fp);
    (void) fseek(fp, current, SEEK_SET);

    if (end <= 0) {
        return 0;
    }

    return (uint32_t) end;
}

static bool st77916_find_first_jpeg_frame(FILE *fp, uint32_t *out_offset, uint32_t *out_size) {
    if (fp == NULL || out_offset == NULL || out_size == NULL) {
        return false;
    }

    if (fseek(fp, 0, SEEK_SET) != 0) {
        return false;
    }

    int previous = -1;
    int current = -1;
    uint32_t pos = 0;
    uint32_t soi = 0;
    bool in_frame = false;

    while ((current = fgetc(fp)) != EOF) {
        if (!in_frame && previous == 0xFF && current == 0xD8) {
            soi = pos - 1;
            in_frame = true;
        } else if (in_frame && previous == 0xFF && current == 0xD9) {
            uint32_t end = pos;
            *out_offset = soi;
            *out_size = end - soi + 1;
            return *out_size >= 4;
        }

        previous = current;
        pos++;
    }

    return false;
}

bool st77916_probe_sd_mjpeg_library(st77916_mjpeg_probe_result_t *out_result) {
    if (out_result == NULL) {
        return false;
    }

    memset(out_result, 0, sizeof(*out_result));
    out_result->status = -1;

    const char *mount_point = "/sdcard";
    sdmmc_card_t *card = NULL;

    esp_vfs_fat_sdmmc_mount_config_t mount_config = {
        .format_if_mount_failed = false,
        .max_files = 4,
        .allocation_unit_size = 16 * 1024,
    };

    sdmmc_host_t host = SDMMC_HOST_DEFAULT();
    host.max_freq_khz = SDMMC_FREQ_DEFAULT;

    sdmmc_slot_config_t slot_config = SDMMC_SLOT_CONFIG_DEFAULT();
    slot_config.width = 1;
    slot_config.clk = GPIO_NUM_14;
    slot_config.cmd = GPIO_NUM_17;
    slot_config.d0 = GPIO_NUM_16;
    slot_config.flags |= SDMMC_SLOT_FLAG_INTERNAL_PULLUP;

    esp_err_t err = esp_vfs_fat_sdmmc_mount(
        mount_point,
        &host,
        &slot_config,
        &mount_config,
        &card
    );

    if (err != ESP_OK || card == NULL) {
        out_result->status = -2;
        return false;
    }

    DIR *dir = opendir("/sdcard/VIDEO");
    if (dir == NULL) {
        out_result->status = -3;
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        return true;
    }

    struct dirent *entry = NULL;
    char first_name[32] = {0};
    char first_path[96] = {0};

    while ((entry = readdir(dir)) != NULL) {
        if (!st77916_has_mjpeg_extension(entry->d_name)) {
            continue;
        }

        out_result->file_count++;
        if (first_name[0] == 0) {
            strncpy(first_name, entry->d_name, sizeof(first_name) - 1);
            snprintf(first_path, sizeof(first_path), "/sdcard/VIDEO/%s", entry->d_name);
        }
    }

    closedir(dir);

    if (out_result->file_count == 0 || first_name[0] == 0) {
        out_result->status = -4;
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        return true;
    }

    strncpy(out_result->first_name, first_name, sizeof(out_result->first_name) - 1);

    FILE *fp = fopen(first_path, "rb");
    if (fp == NULL) {
        out_result->status = -5;
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        return true;
    }

    out_result->first_file_size = st77916_file_size(fp);

    uint32_t frame_offset = 0;
    uint32_t frame_size = 0;
    if (st77916_find_first_jpeg_frame(fp, &frame_offset, &frame_size)) {
        out_result->first_frame_offset = frame_offset;
        out_result->first_frame_size = frame_size;
        out_result->status = 0;
    } else {
        out_result->status = -6;
    }

    fclose(fp);
    esp_vfs_fat_sdcard_unmount(mount_point, card);
    return true;
}



static bool st77916_parse_jpeg_dimensions(const uint8_t *jpg, uint32_t size, uint16_t *out_w, uint16_t *out_h) {
    if (jpg == NULL || size < 8 || out_w == NULL || out_h == NULL) {
        return false;
    }
    if (jpg[0] != 0xFF || jpg[1] != 0xD8) {
        return false;
    }
    uint32_t pos = 2;
    while (pos + 4 < size) {
        if (jpg[pos] != 0xFF) {
            pos++;
            continue;
        }
        while (pos < size && jpg[pos] == 0xFF) {
            pos++;
        }
        if (pos >= size) break;
        uint8_t marker = jpg[pos++];
        if (marker == 0xD9 || marker == 0xDA) break;
        if (pos + 2 > size) break;
        uint16_t seg_len = ((uint16_t)jpg[pos] << 8) | jpg[pos + 1];
        if (seg_len < 2 || pos + seg_len > size) break;
        bool is_sof = marker == 0xC0 || marker == 0xC1 || marker == 0xC2 || marker == 0xC3 ||
                      marker == 0xC5 || marker == 0xC6 || marker == 0xC7 || marker == 0xC9 ||
                      marker == 0xCA || marker == 0xCB || marker == 0xCD || marker == 0xCE || marker == 0xCF;
        if (is_sof && seg_len >= 7) {
            *out_h = ((uint16_t)jpg[pos + 3] << 8) | jpg[pos + 4];
            *out_w = ((uint16_t)jpg[pos + 5] << 8) | jpg[pos + 6];
            return *out_w > 0 && *out_h > 0;
        }
        pos += seg_len;
    }
    return false;
}

static esp_jpeg_image_scale_t st77916_jpeg_scale_from_div(uint8_t div) {
    switch (div) {
        case 2: return JPEG_IMAGE_SCALE_1_2;
        case 4: return JPEG_IMAGE_SCALE_1_4;
        case 8: return JPEG_IMAGE_SCALE_1_8;
        case 1:
        default: return JPEG_IMAGE_SCALE_0;
    }
}


static bool st77916_video_swap_rgb565_enabled(void) {
    FILE *fp = fopen("/sdcard/VIDEO/SWAP.TXT", "rb");
    if (fp == NULL) {
        return false;
    }

    char buf[64] = {0};
    size_t n = fread(buf, 1, sizeof(buf) - 1, fp);
    fclose(fp);
    if (n == 0) {
        return false;
    }

    for (size_t i = 0; i < n; i++) {
        if (buf[i] >= 'a' && buf[i] <= 'z') {
            buf[i] = (char)(buf[i] - 'a' + 'A');
        }
    }

    return strstr(buf, "1") != NULL ||
           strstr(buf, "YES") != NULL ||
           strstr(buf, "TRUE") != NULL ||
           strstr(buf, "SWAP") != NULL;
}

static uint8_t st77916_scale_div_for_fit(uint16_t width, uint16_t height) {
    uint8_t div = 1;
    while (((width / div) > VIDEO_PREVIEW_MAX_WIDTH || (height / div) > VIDEO_PREVIEW_MAX_HEIGHT) && div < 8) {
        div = (uint8_t)(div * 2);
    }
    return div;
}


static bool st77916_find_jpeg_frame_at_index(FILE *fp, uint32_t target_index, uint32_t *out_offset, uint32_t *out_size, uint32_t *out_actual_index) {
    if (fp == NULL || out_offset == NULL || out_size == NULL || out_actual_index == NULL) {
        return false;
    }

    if (fseek(fp, 0, SEEK_SET) != 0) {
        return false;
    }

    *out_offset = 0;
    *out_size = 0;
    *out_actual_index = 0;

    int previous = -1;
    int current = -1;
    uint32_t pos = 0;
    uint32_t soi = 0;
    uint32_t frame_index = 0;
    bool in_frame = false;

    while ((current = fgetc(fp)) != EOF) {
        if (!in_frame && previous == 0xFF && current == 0xD8) {
            soi = pos - 1;
            in_frame = true;
        } else if (in_frame && previous == 0xFF && current == 0xD9) {
            uint32_t end = pos;
            uint32_t frame_size = end - soi + 1;
            if (frame_size >= 4) {
                if (frame_index == target_index) {
                    *out_offset = soi;
                    *out_size = frame_size;
                    *out_actual_index = frame_index;
                    return true;
                }
                frame_index++;
            }
            in_frame = false;
        }

        previous = current;
        pos++;
        if ((pos & 0x3FFFU) == 0) { /* v0.1.30-r2 yield while scanning long MJPEG files */ vTaskDelay(pdMS_TO_TICKS(1)); }
    }

    *out_actual_index = frame_index;
    return false;
}

bool st77916_decode_mjpeg_frame_rgb565(st77916_mjpeg_decode_result_t *out_result, uint16_t *out_rgb565, uint32_t out_pixels, uint32_t frame_index) {
    if (out_result == NULL || out_rgb565 == NULL || out_pixels < (LCD_WIDTH * LCD_HEIGHT)) {
        return false;
    }

    memset(out_result, 0, sizeof(*out_result));
    out_result->status = -1;

    st77916_mjpeg_probe_result_t probe = {0};
    bool probe_ok = st77916_probe_sd_mjpeg_library(&probe);
    out_result->status = probe.status;
    out_result->file_count = probe.file_count;
    out_result->first_file_size = probe.first_file_size;
    out_result->first_frame_offset = probe.first_frame_offset;
    out_result->first_frame_size = probe.first_frame_size;
    strncpy(out_result->first_name, probe.first_name, sizeof(out_result->first_name) - 1);

    if (!probe_ok || probe.status != 0 || probe.first_frame_size == 0) {
        return true;
    }

    const char *mount_point = "/sdcard";
    sdmmc_card_t *card = NULL;
    esp_vfs_fat_sdmmc_mount_config_t mount_config = {
        .format_if_mount_failed = false,
        .max_files = 4,
        .allocation_unit_size = 16 * 1024,
    };
    sdmmc_host_t host = SDMMC_HOST_DEFAULT();
    host.max_freq_khz = SDMMC_FREQ_DEFAULT;
    sdmmc_slot_config_t slot_config = SDMMC_SLOT_CONFIG_DEFAULT();
    slot_config.width = 1;
    slot_config.clk = GPIO_NUM_14;
    slot_config.cmd = GPIO_NUM_17;
    slot_config.d0 = GPIO_NUM_16;
    slot_config.flags |= SDMMC_SLOT_FLAG_INTERNAL_PULLUP;

    esp_err_t err = esp_vfs_fat_sdmmc_mount(mount_point, &host, &slot_config, &mount_config, &card);
    if (err != ESP_OK || card == NULL) {
        out_result->status = -2;
        return true;
    }

    char path[96];
    int path_len = snprintf(path, sizeof(path), "/sdcard/VIDEO/%s", out_result->first_name);
    if (path_len <= 0 || path_len >= (int) sizeof(path)) {
        out_result->status = -7;
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        return true;
    }

    FILE *fp = fopen(path, "rb");
    if (fp == NULL) {
        out_result->status = -5;
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        return true;
    }

    uint32_t target_offset = 0;
    uint32_t target_size = 0;
    uint32_t actual_index = 0;
    bool found_index = st77916_find_jpeg_frame_at_index(fp, frame_index, &target_offset, &target_size, &actual_index);
    bool looped_to_zero = false;
    if (!found_index && frame_index != 0) {
        looped_to_zero = true;
        actual_index = 0;
        found_index = st77916_find_jpeg_frame_at_index(fp, 0, &target_offset, &target_size, &actual_index);
    }

    if (!found_index) {
        fclose(fp);
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        out_result->status = -12;
        out_result->decoded_frame_index = actual_index;
        return true;
    }

    out_result->first_frame_offset = target_offset;
    out_result->first_frame_size = target_size;
    out_result->decoded_frame_index = actual_index;
    if (looped_to_zero) {
        out_result->decoded_frame_index = actual_index;
    }

    uint8_t *jpg = (uint8_t *)heap_caps_malloc(target_size, MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT);
    if (jpg == NULL) jpg = (uint8_t *)malloc(target_size);
    if (jpg == NULL) {
        fclose(fp);
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        out_result->status = -8;
        return true;
    }

    if (fseek(fp, (long)target_offset, SEEK_SET) != 0) {
        free(jpg);
        fclose(fp);
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        out_result->status = -9;
        return true;
    }

    size_t read_len = fread(jpg, 1, target_size, fp);
    bool swap_color_bytes = st77916_video_swap_rgb565_enabled();
    out_result->color_swap = swap_color_bytes ? 1 : 0;
    fclose(fp);
    esp_vfs_fat_sdcard_unmount(mount_point, card);

    if (read_len != target_size) {
        free(jpg);
        out_result->status = -4;
        return true;
    }

    uint16_t jpeg_w = 0, jpeg_h = 0;
    if (!st77916_parse_jpeg_dimensions(jpg, target_size, &jpeg_w, &jpeg_h)) {
        free(jpg);
        out_result->status = -10;
        return true;
    }

    uint8_t scale_div = st77916_scale_div_for_fit(jpeg_w, jpeg_h);
    uint32_t out_w = jpeg_w / scale_div;
    uint32_t out_h = jpeg_h / scale_div;
    uint32_t decoded_bytes = out_w * out_h * 2;
    uint8_t *decoded = (uint8_t *)heap_caps_malloc(decoded_bytes, MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT);
    if (decoded == NULL) decoded = (uint8_t *)malloc(decoded_bytes);
    if (decoded == NULL) {
        free(jpg);
        out_result->status = -8;
        return true;
    }

    esp_jpeg_image_cfg_t jpeg_cfg = {
        .indata = jpg,
        .indata_size = target_size,
        .outbuf = decoded,
        .outbuf_size = decoded_bytes,
        .out_format = JPEG_IMAGE_FORMAT_RGB565,
        .out_scale = st77916_jpeg_scale_from_div(scale_div),
        .flags = { .swap_color_bytes = swap_color_bytes ? 1 : 0 },
    };

    esp_jpeg_image_output_t out_img = {0};
    esp_err_t decode_err = esp_jpeg_decode(&jpeg_cfg, &out_img);
    free(jpg);

    if (decode_err != ESP_OK || out_img.width == 0 || out_img.height == 0) {
        free(decoded);
        out_result->status = -11;
        return true;
    }

    for (uint32_t i = 0; i < LCD_WIDTH * LCD_HEIGHT; i++) out_rgb565[i] = 0x0000;
    uint32_t copy_w = out_img.width > VIDEO_PREVIEW_MAX_WIDTH ? VIDEO_PREVIEW_MAX_WIDTH : out_img.width;
    uint32_t copy_h = out_img.height > VIDEO_PREVIEW_MAX_HEIGHT ? VIDEO_PREVIEW_MAX_HEIGHT : out_img.height;
    uint32_t dst_x = (LCD_WIDTH - copy_w) / 2;
    uint32_t dst_y = VIDEO_PREVIEW_TOP_Y;
    for (uint32_t y = 0; y < copy_h; y++) {
        memcpy(&out_rgb565[(dst_y + y) * LCD_WIDTH + dst_x], &decoded[y * out_img.width * 2], copy_w * 2);
    }

    out_result->jpeg_width = jpeg_w;
    out_result->jpeg_height = jpeg_h;
    out_result->output_width = (uint16_t) out_img.width;
    out_result->output_height = (uint16_t) out_img.height;
    out_result->preview_x = (uint16_t) dst_x;
    out_result->preview_y = (uint16_t) dst_y;
    out_result->decoded_frame_index = actual_index;
    out_result->scale_div = scale_div;
    out_result->status = 0;
    free(decoded);
    return true;
}


bool st77916_decode_first_mjpeg_frame_rgb565(st77916_mjpeg_decode_result_t *out_result, uint16_t *out_rgb565, uint32_t out_pixels) {
    if (out_result == NULL || out_rgb565 == NULL || out_pixels < (LCD_WIDTH * LCD_HEIGHT)) {
        return false;
    }
    memset(out_result, 0, sizeof(*out_result));
    out_result->status = -1;

    st77916_mjpeg_probe_result_t probe = {0};
    bool probe_ok = st77916_probe_sd_mjpeg_library(&probe);
    out_result->status = probe.status;
    out_result->file_count = probe.file_count;
    out_result->first_file_size = probe.first_file_size;
    out_result->first_frame_offset = probe.first_frame_offset;
    out_result->first_frame_size = probe.first_frame_size;
    strncpy(out_result->first_name, probe.first_name, sizeof(out_result->first_name) - 1);
    if (!probe_ok || probe.status != 0 || probe.first_frame_size == 0) {
        return true;
    }

    const char *mount_point = "/sdcard";
    sdmmc_card_t *card = NULL;
    esp_vfs_fat_sdmmc_mount_config_t mount_config = {
        .format_if_mount_failed = false,
        .max_files = 4,
        .allocation_unit_size = 16 * 1024,
    };
    sdmmc_host_t host = SDMMC_HOST_DEFAULT();
    host.max_freq_khz = SDMMC_FREQ_DEFAULT;
    sdmmc_slot_config_t slot_config = SDMMC_SLOT_CONFIG_DEFAULT();
    slot_config.width = 1;
    slot_config.clk = GPIO_NUM_14;
    slot_config.cmd = GPIO_NUM_17;
    slot_config.d0 = GPIO_NUM_16;
    slot_config.flags |= SDMMC_SLOT_FLAG_INTERNAL_PULLUP;
    esp_err_t err = esp_vfs_fat_sdmmc_mount(mount_point, &host, &slot_config, &mount_config, &card);
    if (err != ESP_OK || card == NULL) {
        out_result->status = -2;
        return true;
    }

    char path[96];
    int path_len = snprintf(path, sizeof(path), "/sdcard/VIDEO/%s", out_result->first_name);
    if (path_len <= 0 || path_len >= (int) sizeof(path)) {
        out_result->status = -7;
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        return true;
    }
    FILE *fp = fopen(path, "rb");
    if (fp == NULL) {
        out_result->status = -5;
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        return true;
    }
    uint8_t *jpg = (uint8_t *)heap_caps_malloc(probe.first_frame_size, MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT);
    if (jpg == NULL) jpg = (uint8_t *)malloc(probe.first_frame_size);
    if (jpg == NULL) {
        fclose(fp);
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        out_result->status = -8;
        return true;
    }
    if (fseek(fp, (long)probe.first_frame_offset, SEEK_SET) != 0) {
        free(jpg);
        fclose(fp);
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        out_result->status = -9;
        return true;
    }
    size_t read_len = fread(jpg, 1, probe.first_frame_size, fp);
    bool swap_color_bytes = st77916_video_swap_rgb565_enabled();
    out_result->color_swap = swap_color_bytes ? 1 : 0;
    fclose(fp);
    esp_vfs_fat_sdcard_unmount(mount_point, card);
    if (read_len != probe.first_frame_size) {
        free(jpg);
        out_result->status = -4;
        return true;
    }

    uint16_t jpeg_w = 0, jpeg_h = 0;
    if (!st77916_parse_jpeg_dimensions(jpg, probe.first_frame_size, &jpeg_w, &jpeg_h)) {
        free(jpg);
        out_result->status = -10;
        return true;
    }
    uint8_t scale_div = st77916_scale_div_for_fit(jpeg_w, jpeg_h);
    uint32_t out_w = jpeg_w / scale_div;
    uint32_t out_h = jpeg_h / scale_div;
    uint32_t decoded_bytes = out_w * out_h * 2;
    uint8_t *decoded = (uint8_t *)heap_caps_malloc(decoded_bytes, MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT);
    if (decoded == NULL) decoded = (uint8_t *)malloc(decoded_bytes);
    if (decoded == NULL) {
        free(jpg);
        out_result->status = -8;
        return true;
    }

    esp_jpeg_image_cfg_t jpeg_cfg = {
        .indata = jpg,
        .indata_size = probe.first_frame_size,
        .outbuf = decoded,
        .outbuf_size = decoded_bytes,
        .out_format = JPEG_IMAGE_FORMAT_RGB565,
        .out_scale = st77916_jpeg_scale_from_div(scale_div),
        .flags = { .swap_color_bytes = swap_color_bytes ? 1 : 0 },
    };
    esp_jpeg_image_output_t out_img = {0};
    esp_err_t decode_err = esp_jpeg_decode(&jpeg_cfg, &out_img);
    free(jpg);
    if (decode_err != ESP_OK || out_img.width == 0 || out_img.height == 0) {
        free(decoded);
        out_result->status = -11;
        return true;
    }

    for (uint32_t i = 0; i < LCD_WIDTH * LCD_HEIGHT; i++) out_rgb565[i] = 0x0000;
    uint32_t copy_w = out_img.width > VIDEO_PREVIEW_MAX_WIDTH ? VIDEO_PREVIEW_MAX_WIDTH : out_img.width;
    uint32_t copy_h = out_img.height > VIDEO_PREVIEW_MAX_HEIGHT ? VIDEO_PREVIEW_MAX_HEIGHT : out_img.height;
    uint32_t dst_x = (LCD_WIDTH - copy_w) / 2;
    uint32_t dst_y = VIDEO_PREVIEW_TOP_Y;
    for (uint32_t y = 0; y < copy_h; y++) {
        memcpy(&out_rgb565[(dst_y + y) * LCD_WIDTH + dst_x], &decoded[y * out_img.width * 2], copy_w * 2);
    }

    out_result->jpeg_width = jpeg_w;
    out_result->jpeg_height = jpeg_h;
    out_result->output_width = (uint16_t) out_img.width;
    out_result->output_height = (uint16_t) out_img.height;
    out_result->preview_x = (uint16_t) dst_x;
    out_result->preview_y = (uint16_t) dst_y;
    out_result->scale_div = scale_div;
    out_result->status = 0;
    free(decoded);
    return true;
}


#define ST77916_VIDEO_WORKER_STOPPED 0
#define ST77916_VIDEO_WORKER_PLAYING 1
#define ST77916_VIDEO_WORKER_BUSY 2
#define ST77916_VIDEO_WORKER_TOUCH_STOP 3
#define ST77916_VIDEO_WORKER_SD_OWNER 4
#define ST77916_VIDEO_WORKER_SD_BUSY 5

#define ST77916_MJPEG_STATUS_WORKER_SD_OWNER (-13)
#define ST77916_MJPEG_STATUS_WORKER_SD_BUSY  (-14)
#define ST77916_MJPEG_STATUS_WORKER_DEFERRED (-15)
#define ST77916_MJPEG_STATUS_STREAM_OPEN_FAIL (-16)
#define ST77916_MJPEG_STATUS_STREAM_FRAME_TOO_LARGE (-17)
#define ST77916_MJPEG_STATUS_STREAM_READ_FAIL (-18)
#define VIDEO_STREAM_BUFFER_BYTES (64U * 1024U)
#define VIDEO_STREAM_READ_CHUNK_BYTES 4096U
#define VIDEO_STREAM_DEFAULT_DISPLAY_FPS 5U
#define VIDEO_STREAM_DEFAULT_SOURCE_SKIP 6U
#define VIDEO_STREAM_SOURCE_FPS 30U
#define VIDEO_STREAM_MIN_DISPLAY_FPS 1U
#define VIDEO_STREAM_MAX_DISPLAY_FPS 12U
#define VIDEO_STREAM_MIN_SOURCE_SKIP 1U
#define VIDEO_STREAM_MAX_SOURCE_SKIP 30U
#define VIDEO_STREAM_NORMAL_LOG_EVERY 30U

static TaskHandle_t s_video_worker_task = NULL;
static SemaphoreHandle_t s_video_worker_lock = NULL;
static uint16_t *s_video_worker_decode_frame = NULL;
static uint16_t *s_video_worker_latest_frame = NULL;
static st77916_mjpeg_decode_result_t s_video_worker_latest_meta = {0};
static volatile uint32_t s_video_worker_state = ST77916_VIDEO_WORKER_STOPPED;
static volatile uint32_t s_video_worker_request_counter = 0;
static volatile uint32_t s_video_worker_completed_counter = 0;
static volatile uint32_t s_video_worker_frame_index = 0;
static volatile uint32_t s_video_worker_frame_step = 5;
static volatile uint32_t s_video_worker_frame_ms = 500;
static volatile bool s_video_worker_has_frame = false;
static volatile bool s_video_worker_next_latched = false;
static FILE *s_video_stream_fp = NULL;
static uint8_t *s_video_stream_buf = NULL;
static uint32_t s_video_stream_len = 0;
static uint32_t s_video_stream_file_pos = 0;
static uint32_t s_video_stream_frame_index = 0;
static char s_video_stream_name[32] = {0};
static volatile uint32_t s_video_stream_display_fps = VIDEO_STREAM_DEFAULT_DISPLAY_FPS;
static volatile uint32_t s_video_stream_source_skip = VIDEO_STREAM_DEFAULT_SOURCE_SKIP;
static volatile uint32_t s_video_stream_decode_log_counter = 0;
static volatile bool s_video_stream_wrapped = false;
static volatile uint32_t s_video_stream_read_us = 0;
static volatile uint32_t s_video_stream_skip_us = 0;
static volatile uint32_t s_video_stream_decode_us = 0;
static volatile uint32_t s_video_stream_publish_us = 0;


static bool st77916_decode_jpeg_bytes_to_rgb565(
    st77916_mjpeg_decode_result_t *out_result,
    uint16_t *out_rgb565,
    uint32_t out_pixels,
    const uint8_t *jpg,
    uint32_t jpg_size,
    uint32_t actual_frame_index,
    uint32_t frame_offset,
    const char *first_name
) {
    if (out_result == NULL || out_rgb565 == NULL || out_pixels < (LCD_WIDTH * LCD_HEIGHT) ||
        jpg == NULL || jpg_size < 4) {
        return false;
    }

    memset(out_result, 0, sizeof(*out_result));
    out_result->status = -1;
    out_result->first_frame_offset = frame_offset;
    out_result->first_frame_size = jpg_size;
    out_result->decoded_frame_index = actual_frame_index;
    if (first_name != NULL) {
        strncpy(out_result->first_name, first_name, sizeof(out_result->first_name) - 1);
    }

    uint16_t jpeg_w = 0, jpeg_h = 0;
    if (!st77916_parse_jpeg_dimensions(jpg, jpg_size, &jpeg_w, &jpeg_h)) {
        out_result->status = -10;
        return true;
    }

    uint8_t scale_div = st77916_scale_div_for_fit(jpeg_w, jpeg_h);
    uint32_t out_w = jpeg_w / scale_div;
    uint32_t out_h = jpeg_h / scale_div;
    uint32_t decoded_bytes = out_w * out_h * 2;
    uint8_t *decoded = (uint8_t *)heap_caps_malloc(decoded_bytes, MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT);
    if (decoded == NULL) decoded = (uint8_t *)malloc(decoded_bytes);
    if (decoded == NULL) {
        out_result->status = -8;
        return true;
    }

    bool swap_color_bytes = st77916_video_swap_rgb565_enabled();
    out_result->color_swap = swap_color_bytes ? 1 : 0;

    esp_jpeg_image_cfg_t jpeg_cfg = {
        .indata = (uint8_t *)jpg,
        .indata_size = jpg_size,
        .outbuf = decoded,
        .outbuf_size = decoded_bytes,
        .out_format = JPEG_IMAGE_FORMAT_RGB565,
        .out_scale = st77916_jpeg_scale_from_div(scale_div),
        .flags = { .swap_color_bytes = swap_color_bytes ? 1 : 0 },
    };

    esp_jpeg_image_output_t out_img = {0};
    esp_err_t decode_err = esp_jpeg_decode(&jpeg_cfg, &out_img);

    if (decode_err != ESP_OK || out_img.width == 0 || out_img.height == 0) {
        free(decoded);
        out_result->status = -11;
        return true;
    }

    for (uint32_t i = 0; i < LCD_WIDTH * LCD_HEIGHT; i++) out_rgb565[i] = 0x0000;
    uint32_t copy_w = out_img.width > VIDEO_PREVIEW_MAX_WIDTH ? VIDEO_PREVIEW_MAX_WIDTH : out_img.width;
    uint32_t copy_h = out_img.height > VIDEO_PREVIEW_MAX_HEIGHT ? VIDEO_PREVIEW_MAX_HEIGHT : out_img.height;
    uint32_t dst_x = (LCD_WIDTH - copy_w) / 2;
    uint32_t dst_y = VIDEO_PREVIEW_TOP_Y;
    for (uint32_t y = 0; y < copy_h; y++) {
        memcpy(&out_rgb565[(dst_y + y) * LCD_WIDTH + dst_x], &decoded[y * out_img.width * 2], copy_w * 2);
    }

    out_result->jpeg_width = jpeg_w;
    out_result->jpeg_height = jpeg_h;
    out_result->output_width = (uint16_t)out_img.width;
    out_result->output_height = (uint16_t)out_img.height;
    out_result->preview_x = (uint16_t)dst_x;
    out_result->preview_y = (uint16_t)dst_y;
    out_result->scale_div = scale_div;
    out_result->status = 0;

    free(decoded);
    return true;
}

static bool st77916_video_stream_ensure_buffer(void) {
    if (s_video_stream_buf == NULL) {
        s_video_stream_buf = (uint8_t *)heap_caps_malloc(VIDEO_STREAM_BUFFER_BYTES, MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT);
        if (s_video_stream_buf == NULL) {
            s_video_stream_buf = (uint8_t *)malloc(VIDEO_STREAM_BUFFER_BYTES);
        }
    }
    return s_video_stream_buf != NULL;
}

static void st77916_video_stream_close(void) {
    if (s_video_stream_fp != NULL) {
        fclose(s_video_stream_fp);
        s_video_stream_fp = NULL;
    }
    s_video_stream_len = 0;
    s_video_stream_file_pos = 0;
    s_video_stream_frame_index = 0;
    s_video_stream_name[0] = 0;
    s_video_stream_wrapped = false;
    s_video_stream_read_us = 0;
    s_video_stream_skip_us = 0;
    s_video_stream_decode_us = 0;
    s_video_stream_publish_us = 0;
}

static uint32_t st77916_video_stream_parse_u32(const char *buf, size_t len, uint32_t fallback, uint32_t min_value, uint32_t max_value) {
    uint32_t value = 0;
    bool saw_digit = false;
    for (size_t i = 0; i < len; i++) {
        char ch = buf[i];
        if (ch >= '0' && ch <= '9') {
            saw_digit = true;
            value = (value * 10U) + (uint32_t)(ch - '0');
            if (value > max_value) {
                value = max_value;
                break;
            }
        } else if (saw_digit) {
            break;
        }
    }

    if (!saw_digit) {
        return fallback;
    }
    if (value < min_value) return min_value;
    if (value > max_value) return max_value;
    return value;
}

static uint32_t st77916_video_stream_read_u32_config(const char *path, uint32_t fallback, uint32_t min_value, uint32_t max_value) {
    if (path == NULL) {
        return fallback;
    }

    if (!st77916_sd_owner_try_acquire("VIDEO_CFG_READ", pdMS_TO_TICKS(300))) {
        return fallback;
    }

    FILE *fp = fopen(path, "rb");
    if (fp == NULL) {
        st77916_sd_owner_release("VIDEO_CFG_READ");
        return fallback;
    }

    char buf[32] = {0};
    size_t n = fread(buf, 1, sizeof(buf) - 1, fp);
    fclose(fp);
    st77916_sd_owner_release("VIDEO_CFG_READ");

    if (n == 0) {
        return fallback;
    }

    return st77916_video_stream_parse_u32(buf, n, fallback, min_value, max_value);
}

static void st77916_video_stream_load_runtime_config(uint32_t requested_frame_ms) {
    uint32_t display_fps = st77916_video_stream_read_u32_config(
        "/sdcard/VIDEO/FPS.TXT",
        VIDEO_STREAM_DEFAULT_DISPLAY_FPS,
        VIDEO_STREAM_MIN_DISPLAY_FPS,
        VIDEO_STREAM_MAX_DISPLAY_FPS
    );
    uint32_t source_skip = st77916_video_stream_read_u32_config(
        "/sdcard/VIDEO/SKIP.TXT",
        VIDEO_STREAM_DEFAULT_SOURCE_SKIP,
        VIDEO_STREAM_MIN_SOURCE_SKIP,
        VIDEO_STREAM_MAX_SOURCE_SKIP
    );

    s_video_stream_display_fps = display_fps;
    s_video_stream_source_skip = source_skip;

    if (display_fps > 0) {
        s_video_worker_frame_ms = 1000U / display_fps;
    } else if (requested_frame_ms > 0) {
        s_video_worker_frame_ms = requested_frame_ms;
    } else {
        s_video_worker_frame_ms = 1000U / VIDEO_STREAM_DEFAULT_DISPLAY_FPS;
    }

    printf(
        "video-stream-config: source_fps=%lu display_fps=%lu skip=%lu effective_fps=%lu frame_ms=%lu source=/VIDEO/FPS.TXT,/VIDEO/SKIP.TXT audio=DEFERRED\n",
        (unsigned long)VIDEO_STREAM_SOURCE_FPS,
        (unsigned long)s_video_stream_display_fps,
        (unsigned long)s_video_stream_source_skip,
        (unsigned long)(s_video_stream_display_fps * s_video_stream_source_skip),
        (unsigned long)s_video_worker_frame_ms
    );
}

uint32_t st77916_video_worker_frame_ms(void) {
    uint32_t frame_ms = s_video_worker_frame_ms;
    if (frame_ms == 0) {
        frame_ms = 1000U / VIDEO_STREAM_DEFAULT_DISPLAY_FPS;
    }
    return frame_ms;
}

uint32_t st77916_video_worker_display_fps(void) {
    return s_video_stream_display_fps == 0 ? VIDEO_STREAM_DEFAULT_DISPLAY_FPS : s_video_stream_display_fps;
}

uint32_t st77916_video_worker_source_skip(void) {
    return s_video_stream_source_skip == 0 ? VIDEO_STREAM_DEFAULT_SOURCE_SKIP : s_video_stream_source_skip;
}


static bool st77916_video_stream_open_first(st77916_mjpeg_decode_result_t *out_meta) {
    if (s_video_stream_fp != NULL) {
        return true;
    }

    if (!st77916_video_stream_ensure_buffer()) {
        if (out_meta != NULL) out_meta->status = -8;
        return false;
    }

    st77916_mjpeg_probe_result_t probe = {0};
    bool probe_ok = st77916_probe_sd_mjpeg_library(&probe);
    if (out_meta != NULL) {
        memset(out_meta, 0, sizeof(*out_meta));
        out_meta->status = probe.status;
        out_meta->file_count = probe.file_count;
        out_meta->first_file_size = probe.first_file_size;
        out_meta->first_frame_offset = probe.first_frame_offset;
        out_meta->first_frame_size = probe.first_frame_size;
        strncpy(out_meta->first_name, probe.first_name, sizeof(out_meta->first_name) - 1);
    }

    if (!probe_ok || probe.status != 0 || probe.first_name[0] == 0) {
        return false;
    }

    char path[96];
    int path_len = snprintf(path, sizeof(path), "/sdcard/VIDEO/%s", probe.first_name);
    if (path_len <= 0 || path_len >= (int)sizeof(path)) {
        if (out_meta != NULL) out_meta->status = -7;
        return false;
    }

    if (!st77916_sd_owner_try_acquire("VIDEO_STREAM_OPEN", pdMS_TO_TICKS(1000))) {
        if (out_meta != NULL) out_meta->status = ST77916_MJPEG_STATUS_WORKER_SD_BUSY;
        return false;
    }

    FILE *fp = fopen(path, "rb");
    st77916_sd_owner_release("VIDEO_STREAM_OPEN");

    if (fp == NULL) {
        if (out_meta != NULL) out_meta->status = ST77916_MJPEG_STATUS_STREAM_OPEN_FAIL;
        return false;
    }

    s_video_stream_fp = fp;
    s_video_stream_len = 0;
    s_video_stream_file_pos = 0;
    s_video_stream_frame_index = 0;
    strncpy(s_video_stream_name, probe.first_name, sizeof(s_video_stream_name) - 1);
    return true;
}

static bool st77916_video_stream_locate_frame(uint32_t *out_offset_in_buf, uint32_t *out_size, uint32_t *out_file_offset) {
    if (s_video_stream_buf == NULL || out_offset_in_buf == NULL || out_size == NULL || out_file_offset == NULL) {
        return false;
    }

    int32_t soi = -1;
    for (uint32_t i = 0; i + 1 < s_video_stream_len; i++) {
        if (s_video_stream_buf[i] == 0xFF && s_video_stream_buf[i + 1] == 0xD8) {
            soi = (int32_t)i;
            break;
        }
    }

    if (soi < 0) {
        if (s_video_stream_len > 1) {
            uint8_t last = s_video_stream_buf[s_video_stream_len - 1];
            s_video_stream_buf[0] = last;
            s_video_stream_len = 1;
        }
        return false;
    }

    if (soi > 0) {
        uint32_t drop = (uint32_t)soi;
        memmove(s_video_stream_buf, s_video_stream_buf + drop, s_video_stream_len - drop);
        s_video_stream_len -= drop;
    }

    for (uint32_t i = 2; i + 1 < s_video_stream_len; i++) {
        if (s_video_stream_buf[i] == 0xFF && s_video_stream_buf[i + 1] == 0xD9) {
            uint32_t frame_size = i + 2;
            *out_offset_in_buf = 0;
            *out_size = frame_size;
            *out_file_offset = s_video_stream_file_pos >= s_video_stream_len
                ? (s_video_stream_file_pos - s_video_stream_len)
                : 0;
            return frame_size >= 4;
        }
    }

    return false;
}

static bool st77916_video_stream_read_more(st77916_mjpeg_decode_result_t *out_meta) {
    if (s_video_stream_fp == NULL || s_video_stream_buf == NULL) {
        if (out_meta != NULL) out_meta->status = ST77916_MJPEG_STATUS_STREAM_OPEN_FAIL;
        return false;
    }

    if (s_video_stream_len >= VIDEO_STREAM_BUFFER_BYTES) {
        if (out_meta != NULL) out_meta->status = ST77916_MJPEG_STATUS_STREAM_FRAME_TOO_LARGE;
        s_video_stream_len = 0;
        return false;
    }

    uint32_t space = VIDEO_STREAM_BUFFER_BYTES - s_video_stream_len;
    uint32_t want = space > VIDEO_STREAM_READ_CHUNK_BYTES ? VIDEO_STREAM_READ_CHUNK_BYTES : space;

    int64_t read_t0 = esp_timer_get_time();
    if (!st77916_sd_owner_try_acquire("VIDEO_STREAM_READ", pdMS_TO_TICKS(1000))) {
        if (out_meta != NULL) out_meta->status = ST77916_MJPEG_STATUS_WORKER_SD_BUSY;
        return false;
    }

    size_t n = fread(s_video_stream_buf + s_video_stream_len, 1, want, s_video_stream_fp);
    bool at_eof = feof(s_video_stream_fp) != 0;
    bool had_error = ferror(s_video_stream_fp) != 0;
    st77916_sd_owner_release("VIDEO_STREAM_READ");
    int64_t read_t1 = esp_timer_get_time();
    if (read_t1 > read_t0) {
        s_video_stream_read_us += (uint32_t)(read_t1 - read_t0);
    }

    if (n > 0) {
        s_video_stream_len += (uint32_t)n;
        s_video_stream_file_pos += (uint32_t)n;
        return true;
    }

    if (had_error) {
        clearerr(s_video_stream_fp);
        if (out_meta != NULL) out_meta->status = ST77916_MJPEG_STATUS_STREAM_READ_FAIL;
        return false;
    }

    if (at_eof) {
        if (st77916_sd_owner_try_acquire("VIDEO_STREAM_REWIND", pdMS_TO_TICKS(1000))) {
            fseek(s_video_stream_fp, 0, SEEK_SET);
            clearerr(s_video_stream_fp);
            st77916_sd_owner_release("VIDEO_STREAM_REWIND");
        }
        s_video_stream_len = 0;
        s_video_stream_file_pos = 0;
        s_video_stream_frame_index = 0;
        s_video_stream_wrapped = true;
        return true;
    }

    return false;
}

static bool st77916_video_stream_drop_one_frame(st77916_mjpeg_decode_result_t *out_meta) {
    uint32_t offset_in_buf = 0;
    uint32_t frame_size = 0;
    uint32_t file_offset = 0;
    uint32_t guard = 0;

    while (!st77916_video_stream_locate_frame(&offset_in_buf, &frame_size, &file_offset)) {
        if (!st77916_video_stream_read_more(out_meta)) {
            return false;
        }
        if (s_video_stream_wrapped) {
            return true;
        }
        guard++;
        if (guard > 128) {
            if (out_meta != NULL) out_meta->status = ST77916_MJPEG_STATUS_STREAM_READ_FAIL;
            return false;
        }
        vTaskDelay(pdMS_TO_TICKS(1));
    }

    uint32_t remaining = s_video_stream_len - (offset_in_buf + frame_size);
    if (remaining > 0) {
        memmove(s_video_stream_buf, s_video_stream_buf + offset_in_buf + frame_size, remaining);
    }
    s_video_stream_len = remaining;
    s_video_stream_frame_index++;
    return true;
}

/* v0.1.31-r2 eof_wrap decodes frame 0 before skip resumes. */
/* v0.1.31-r2 decode_selected_only: skipped source frames are dropped without JPEG decode; EOF wrap decodes frame 0 before resuming skips. */
static bool st77916_video_stream_decode_next(st77916_mjpeg_decode_result_t *out_meta, uint16_t *out_rgb565, uint32_t out_pixels) {
    if (out_meta == NULL || out_rgb565 == NULL || out_pixels < (LCD_WIDTH * LCD_HEIGHT)) {
        return false;
    }

    memset(out_meta, 0, sizeof(*out_meta));
    out_meta->status = -1;
    s_video_stream_read_us = 0;
    s_video_stream_skip_us = 0;
    s_video_stream_decode_us = 0;
    s_video_stream_publish_us = 0;
    s_video_stream_wrapped = false;

    if (!st77916_video_stream_open_first(out_meta)) {
        return true;
    }

    uint32_t source_skip = s_video_stream_source_skip;
    if (source_skip == 0) {
        source_skip = VIDEO_STREAM_DEFAULT_SOURCE_SKIP;
    }

    uint32_t dropped = 0;
    if (s_video_stream_frame_index > 0 && source_skip > 1) {
        int64_t skip_t0 = esp_timer_get_time();
        for (uint32_t i = 1; i < source_skip; i++) {
            if (!st77916_video_stream_drop_one_frame(out_meta)) {
                return true;
            }
            if (s_video_stream_wrapped) {
                break;
            }
            dropped++;
        }
        int64_t skip_t1 = esp_timer_get_time();
        if (skip_t1 > skip_t0) {
            s_video_stream_skip_us += (uint32_t)(skip_t1 - skip_t0);
        }
    }

    uint32_t offset_in_buf = 0;
    uint32_t frame_size = 0;
    uint32_t file_offset = 0;
    uint32_t guard = 0;

    while (!st77916_video_stream_locate_frame(&offset_in_buf, &frame_size, &file_offset)) {
        if (!st77916_video_stream_read_more(out_meta)) {
            return true;
        }
        guard++;
        if (guard > 128) {
            out_meta->status = ST77916_MJPEG_STATUS_STREAM_READ_FAIL;
            return true;
        }
        vTaskDelay(pdMS_TO_TICKS(1));
    }

    int64_t decode_t0 = esp_timer_get_time();
    bool ok = st77916_decode_jpeg_bytes_to_rgb565(
        out_meta,
        out_rgb565,
        out_pixels,
        s_video_stream_buf + offset_in_buf,
        frame_size,
        s_video_stream_frame_index,
        file_offset,
        s_video_stream_name
    );
    int64_t decode_t1 = esp_timer_get_time();
    if (decode_t1 > decode_t0) {
        s_video_stream_decode_us += (uint32_t)(decode_t1 - decode_t0);
    }

    uint32_t remaining = s_video_stream_len - (offset_in_buf + frame_size);
    if (remaining > 0) {
        memmove(s_video_stream_buf, s_video_stream_buf + offset_in_buf + frame_size, remaining);
    }
    s_video_stream_len = remaining;

    if (ok && out_meta->status == 0) {
        s_video_stream_frame_index++;
    }

    out_meta->file_count = dropped;
    return ok;
}

static bool st77916_video_worker_ensure_allocated(void) {
    if (s_video_worker_lock == NULL) {
        s_video_worker_lock = xSemaphoreCreateMutex();
        if (s_video_worker_lock == NULL) {
            return false;
        }
    }

    if (s_video_worker_decode_frame == NULL) {
        s_video_worker_decode_frame = (uint16_t *)heap_caps_malloc(LCD_WIDTH * LCD_HEIGHT * sizeof(uint16_t), MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT);
        if (s_video_worker_decode_frame == NULL) {
            s_video_worker_decode_frame = (uint16_t *)malloc(LCD_WIDTH * LCD_HEIGHT * sizeof(uint16_t));
        }
    }

    if (s_video_worker_latest_frame == NULL) {
        s_video_worker_latest_frame = (uint16_t *)heap_caps_malloc(LCD_WIDTH * LCD_HEIGHT * sizeof(uint16_t), MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT);
        if (s_video_worker_latest_frame == NULL) {
            s_video_worker_latest_frame = (uint16_t *)malloc(LCD_WIDTH * LCD_HEIGHT * sizeof(uint16_t));
        }
    }

    return s_video_worker_decode_frame != NULL && s_video_worker_latest_frame != NULL;
}

static void st77916_video_worker_task(void *arg) {
    (void)arg;

    for (;;) {
        if (s_video_worker_state == ST77916_VIDEO_WORKER_PLAYING &&
            s_video_worker_request_counter != s_video_worker_completed_counter) {
            uint32_t requested_counter = s_video_worker_request_counter;

            if (!st77916_sd_persistent_is_ready()) {
                st77916_mjpeg_decode_result_t meta = {0};
                meta.status = ST77916_MJPEG_STATUS_WORKER_SD_OWNER;
                meta.decoded_frame_index = s_video_stream_frame_index;
                strncpy(meta.first_name, "SD-OWNER", sizeof(meta.first_name) - 1);
                if (xSemaphoreTake(s_video_worker_lock, pdMS_TO_TICKS(5)) == pdTRUE) {
                    s_video_worker_completed_counter = requested_counter;
                    s_video_worker_latest_meta = meta;
                    s_video_worker_has_frame = false;
                    if (s_video_worker_next_latched) {
                        s_video_worker_next_latched = false;
                        s_video_worker_request_counter = s_video_worker_completed_counter + 1;
                    }
                    xSemaphoreGive(s_video_worker_lock);
                }
                s_video_worker_state = ST77916_VIDEO_WORKER_PLAYING;
                vTaskDelay(pdMS_TO_TICKS(20));
                continue;
            }

            s_video_worker_state = ST77916_VIDEO_WORKER_BUSY;

            uint32_t requested_frame_index = s_video_stream_frame_index;
            st77916_mjpeg_decode_result_t meta = {0};
            bool ok = st77916_video_stream_decode_next(
                &meta,
                s_video_worker_decode_frame,
                LCD_WIDTH * LCD_HEIGHT
            );

            if (s_video_worker_state == ST77916_VIDEO_WORKER_BUSY) {
                uint32_t decoded_frame_index = meta.decoded_frame_index;
                uint32_t next_frame_index = s_video_stream_frame_index;

                int64_t publish_t0 = esp_timer_get_time();
                if (xSemaphoreTake(s_video_worker_lock, pdMS_TO_TICKS(20)) == pdTRUE) {
                    s_video_worker_completed_counter = requested_counter;
                    if (ok && meta.status == 0) {
                        memcpy(
                            s_video_worker_latest_frame,
                            s_video_worker_decode_frame,
                            LCD_WIDTH * LCD_HEIGHT * sizeof(uint16_t)
                        );
                        s_video_worker_latest_meta = meta;
                        s_video_worker_has_frame = true;
                    } else {
                        s_video_worker_latest_meta = meta;
                        s_video_worker_has_frame = false;
                    }

                    if (s_video_worker_next_latched) {
                        s_video_worker_next_latched = false;
                        s_video_worker_request_counter = s_video_worker_completed_counter + 1;
                    }

                    xSemaphoreGive(s_video_worker_lock);
                }
                int64_t publish_t1 = esp_timer_get_time();
                if (publish_t1 > publish_t0) {
                    s_video_stream_publish_us = (uint32_t)(publish_t1 - publish_t0);
                }

                uint32_t log_counter = s_video_stream_decode_log_counter++;
                bool log_this = s_runtime_log_debug_enabled ||
                    (log_counter == 0) ||
                    ((log_counter % VIDEO_STREAM_NORMAL_LOG_EVERY) == 0) ||
                    (meta.status != 0);
                if (log_this) {
                    printf(
                        "video-stream-decode: requested_frame=%lu actual_frame=%lu decoded_frame=%lu frame_offset=%lu frame_size=%lu dropped=%lu status=%ld next_frame=%lu buffer_len=%lu source_fps=%lu display_fps=%lu skip=%lu effective_fps=%lu read_ms=%lu skip_ms=%lu decode_ms=%lu publish_ms=%lu log_mode=%s log_every=%lu sd_persistent=%s audio=DEFERRED\n",
                        (unsigned long)requested_frame_index,
                        (unsigned long)decoded_frame_index,
                        (unsigned long)decoded_frame_index,
                        (unsigned long)meta.first_frame_offset,
                        (unsigned long)meta.first_frame_size,
                        (unsigned long)meta.file_count,
                        (long)meta.status,
                        (unsigned long)next_frame_index,
                        (unsigned long)s_video_stream_len,
                        (unsigned long)VIDEO_STREAM_SOURCE_FPS,
                        (unsigned long)s_video_stream_display_fps,
                        (unsigned long)s_video_stream_source_skip,
                        (unsigned long)(s_video_stream_display_fps * s_video_stream_source_skip),
                        (unsigned long)(s_video_stream_read_us / 1000U),
                        (unsigned long)(s_video_stream_skip_us / 1000U),
                        (unsigned long)(s_video_stream_decode_us / 1000U),
                        (unsigned long)(s_video_stream_publish_us / 1000U),
                        s_runtime_log_debug_enabled ? "DEBUG" : "NORMAL",
                        (unsigned long)VIDEO_STREAM_NORMAL_LOG_EVERY,
                        st77916_sd_persistent_is_ready() ? "READY" : "NOT_READY"
                    );
                }

                s_video_worker_state = ST77916_VIDEO_WORKER_PLAYING;
            }

            vTaskDelay(pdMS_TO_TICKS(1));
        } else {
            vTaskDelay(pdMS_TO_TICKS(20));
        }
    }
}
bool st77916_video_worker_start(uint32_t frame_step, uint32_t frame_ms) {
    if (!st77916_video_worker_ensure_allocated()) {
        return false;
    }

    if (s_video_worker_task == NULL) {
        BaseType_t created = xTaskCreatePinnedToCore(
            st77916_video_worker_task,
            "mjpeg-worker",
            8192,
            NULL,
            4,
            &s_video_worker_task,
            1
        );
        if (created != pdPASS) {
            s_video_worker_task = NULL;
            return false;
        }
    }

    if (xSemaphoreTake(s_video_worker_lock, portMAX_DELAY) == pdTRUE) {
        (void)frame_step;
        st77916_video_stream_close();
        st77916_video_stream_load_runtime_config(frame_ms);
        s_video_worker_frame_step = s_video_stream_source_skip == 0 ? VIDEO_STREAM_DEFAULT_SOURCE_SKIP : s_video_stream_source_skip;
        s_video_worker_frame_index = 0;
        s_video_worker_request_counter = 0;
        s_video_worker_completed_counter = 0;
        s_video_stream_decode_log_counter = 0;
        s_video_worker_has_frame = false;
        s_video_worker_next_latched = false;
        memset(&s_video_worker_latest_meta, 0, sizeof(s_video_worker_latest_meta));
        s_video_worker_latest_meta.status = ST77916_MJPEG_STATUS_WORKER_SD_OWNER;
        strncpy(s_video_worker_latest_meta.first_name, "SD-OWNER", sizeof(s_video_worker_latest_meta.first_name) - 1);
        s_video_worker_state = ST77916_VIDEO_WORKER_PLAYING;
        xSemaphoreGive(s_video_worker_lock);
    }

    return true;
}

void st77916_video_worker_stop(void) {
    if (s_video_worker_lock != NULL && xSemaphoreTake(s_video_worker_lock, portMAX_DELAY) == pdTRUE) {
        s_video_worker_state = ST77916_VIDEO_WORKER_STOPPED;
        s_video_worker_frame_index = 0;
        s_video_worker_request_counter = 0;
        s_video_worker_completed_counter = 0;
        s_video_worker_has_frame = false;
        s_video_worker_next_latched = false;
        st77916_video_stream_close();
        xSemaphoreGive(s_video_worker_lock);
    } else {
        s_video_worker_state = ST77916_VIDEO_WORKER_STOPPED;
    }
}

void st77916_video_worker_request_next(void) {
    uint32_t state = s_video_worker_state;
    if (state == ST77916_VIDEO_WORKER_STOPPED || state == ST77916_VIDEO_WORKER_TOUCH_STOP) {
        return;
    }

    if (state == ST77916_VIDEO_WORKER_BUSY ||
        state == ST77916_VIDEO_WORKER_SD_OWNER ||
        state == ST77916_VIDEO_WORKER_SD_BUSY ||
        s_video_worker_request_counter != s_video_worker_completed_counter) {
        s_video_worker_next_latched = true;
        return;
    }

    s_video_worker_request_counter++;
}

bool st77916_video_worker_copy_latest(uint16_t *out_rgb565, uint32_t out_pixels, st77916_mjpeg_decode_result_t *out_result) {
    if (out_rgb565 == NULL || out_result == NULL || out_pixels < (LCD_WIDTH * LCD_HEIGHT)) {
        return false;
    }

    bool copied = false;
    if (s_video_worker_lock != NULL && xSemaphoreTake(s_video_worker_lock, pdMS_TO_TICKS(2)) == pdTRUE) {
        if (s_video_worker_has_frame && s_video_worker_latest_frame != NULL) {
            memcpy(out_rgb565, s_video_worker_latest_frame, LCD_WIDTH * LCD_HEIGHT * sizeof(uint16_t));
            *out_result = s_video_worker_latest_meta;
            copied = true;
        } else {
            for (uint32_t i = 0; i < LCD_WIDTH * LCD_HEIGHT; i++) {
                out_rgb565[i] = 0x0000;
            }
            *out_result = s_video_worker_latest_meta;
            copied = true;
        }
        xSemaphoreGive(s_video_worker_lock);
    }

    return copied;
}

uint32_t st77916_video_worker_state(void) {
    return s_video_worker_state;
}

const char *st77916_sd_owner_status(void) {
    return s_sd_owner_label;
}


int32_t st77916_write_sd_weather_txt(const uint8_t *data, uint32_t data_len) {
    if (data == NULL || data_len == 0) {
        return -1;
    }

    const char *mount_point = "/sdcard";
    sdmmc_card_t *card = NULL;

    esp_vfs_fat_sdmmc_mount_config_t mount_config = {
        .format_if_mount_failed = false,
        .max_files = 4,
        .allocation_unit_size = 16 * 1024,
    };

    sdmmc_host_t host = SDMMC_HOST_DEFAULT();
    host.max_freq_khz = SDMMC_FREQ_DEFAULT;

    sdmmc_slot_config_t slot_config = SDMMC_SLOT_CONFIG_DEFAULT();
    slot_config.width = 1;
    slot_config.clk = GPIO_NUM_14;
    slot_config.cmd = GPIO_NUM_17;
    slot_config.d0 = GPIO_NUM_16;
    slot_config.flags |= SDMMC_SLOT_FLAG_INTERNAL_PULLUP;

    esp_err_t err = esp_vfs_fat_sdmmc_mount(
        mount_point,
        &host,
        &slot_config,
        &mount_config,
        &card
    );

    if (err != ESP_OK || card == NULL) {
        return -2;
    }

    FILE *fp = fopen("/sdcard/WEATHER.TXT", "wb");
    if (fp == NULL) {
        esp_vfs_fat_sdcard_unmount(mount_point, card);
        return -3;
    }

    size_t n = fwrite(data, 1, data_len, fp);
    fclose(fp);

    esp_vfs_fat_sdcard_unmount(mount_point, card);

    if (n != data_len) {
        return -4;
    }

    return (int32_t) n;
}
