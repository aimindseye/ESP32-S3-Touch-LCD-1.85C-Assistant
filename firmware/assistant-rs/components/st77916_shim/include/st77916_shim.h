#pragma once

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

bool st77916_panel_init(void);
bool st77916_panel_draw_rgb565(uint16_t x0, uint16_t y0, uint16_t x1, uint16_t y1, uint16_t *color);
bool st77916_probe_sd_capacity_mb(bool *out_present, uint32_t *out_capacity_mb);

#ifdef __cplusplus
}
#endif