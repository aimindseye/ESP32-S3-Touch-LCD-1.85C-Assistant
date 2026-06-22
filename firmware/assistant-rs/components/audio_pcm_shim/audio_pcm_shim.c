#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "driver/gpio.h"
#include "driver/i2s_std.h"
#include "esp_err.h"
#include "freertos/FreeRTOS.h"

#define AUDIO_I2S_PORT I2S_NUM_0
#define AUDIO_BCLK_GPIO GPIO_NUM_48
#define AUDIO_LRCLK_GPIO GPIO_NUM_38
#define AUDIO_DOUT_GPIO GPIO_NUM_47

static i2s_chan_handle_t s_tx_chan = NULL;
static bool s_ready = false;

static i2s_data_bit_width_t bit_width_from_u16(uint16_t bits) {
    switch (bits) {
    case 8:
        return I2S_DATA_BIT_WIDTH_8BIT;
    case 16:
        return I2S_DATA_BIT_WIDTH_16BIT;
    case 24:
        return I2S_DATA_BIT_WIDTH_24BIT;
    case 32:
        return I2S_DATA_BIT_WIDTH_32BIT;
    default:
        return I2S_DATA_BIT_WIDTH_16BIT;
    }
}

void st77916_audio_pcm_stop(void) {
    if (s_tx_chan != NULL) {
        if (s_ready) {
            (void)i2s_channel_disable(s_tx_chan);
        }
        (void)i2s_del_channel(s_tx_chan);
        s_tx_chan = NULL;
    }
    s_ready = false;
}

bool st77916_audio_pcm_init(uint32_t sample_rate, uint16_t bits_per_sample, uint16_t channels) {
    if (sample_rate < 8000 || sample_rate > 96000) {
        return false;
    }
    if (!(bits_per_sample == 8 || bits_per_sample == 16 || bits_per_sample == 24 || bits_per_sample == 32)) {
        return false;
    }
    if (!(channels == 1 || channels == 2)) {
        return false;
    }

    st77916_audio_pcm_stop();

    i2s_chan_config_t chan_cfg = I2S_CHANNEL_DEFAULT_CONFIG(AUDIO_I2S_PORT, I2S_ROLE_MASTER);
    esp_err_t err = i2s_new_channel(&chan_cfg, &s_tx_chan, NULL);
    if (err != ESP_OK || s_tx_chan == NULL) {
        s_tx_chan = NULL;
        return false;
    }

    i2s_slot_mode_t slot_mode = channels == 1 ? I2S_SLOT_MODE_MONO : I2S_SLOT_MODE_STEREO;
    i2s_data_bit_width_t bit_width = bit_width_from_u16(bits_per_sample);

    i2s_std_config_t std_cfg = {
        .clk_cfg = I2S_STD_CLK_DEFAULT_CONFIG(sample_rate),
        .slot_cfg = I2S_STD_PHILIPS_SLOT_DEFAULT_CONFIG(bit_width, slot_mode),
        .gpio_cfg = {
            .mclk = I2S_GPIO_UNUSED,
            .bclk = AUDIO_BCLK_GPIO,
            .ws = AUDIO_LRCLK_GPIO,
            .dout = AUDIO_DOUT_GPIO,
            .din = I2S_GPIO_UNUSED,
            .invert_flags = {
                .mclk_inv = false,
                .bclk_inv = false,
                .ws_inv = false,
            },
        },
    };

    err = i2s_channel_init_std_mode(s_tx_chan, &std_cfg);
    if (err != ESP_OK) {
        st77916_audio_pcm_stop();
        return false;
    }

    err = i2s_channel_enable(s_tx_chan);
    if (err != ESP_OK) {
        st77916_audio_pcm_stop();
        return false;
    }

    s_ready = true;
    return true;
}

int32_t st77916_audio_pcm_write(const uint8_t *data, uint32_t len, uint32_t timeout_ms) {
    if (!s_ready || s_tx_chan == NULL || data == NULL || len == 0) {
        return -1;
    }
    size_t bytes_written = 0;
    esp_err_t err = i2s_channel_write(s_tx_chan, data, (size_t)len, &bytes_written, pdMS_TO_TICKS(timeout_ms));
    if (err != ESP_OK) {
        return -2;
    }
    return (int32_t)bytes_written;
}

bool st77916_audio_pcm_is_ready(void) {
    return s_ready && s_tx_chan != NULL;
}
