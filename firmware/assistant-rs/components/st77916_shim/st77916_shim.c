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
#include "esp_heap_caps.h"


#define LCD_OPCODE_READ_CMD                 (0x0BULL)

#define LCD_WIDTH                           360
#define LCD_HEIGHT                          360
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





/*
 * Video/MJPEG support was removed from the accepted product path.
 * The active firmware keeps SD assets, weather cache, music, and radio only.
 * RAW-V1-0-1-R14-VIDEO-MJPEG-C-SHIM-REMOVED
 */

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
