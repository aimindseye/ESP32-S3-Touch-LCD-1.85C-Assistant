#pragma once

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

bool lvgl_lab_bridge_available(void);
bool lvgl_lab_bridge_real_lvgl_enabled(void);
bool lvgl_lab_render_test_rgb565(uint16_t *frame, int32_t width, int32_t height);

#ifdef __cplusplus
}
#endif
