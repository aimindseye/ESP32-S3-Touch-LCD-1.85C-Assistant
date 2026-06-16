#pragma once

#include <Arduino.h>
#include <lvgl.h>

#include "Display_ST77916.h"
#include "Touch_CST816.h"

void Lvgl_Init(void);
void Lvgl_Loop(void);

bool lvgl_port_lock(uint32_t timeout_ms = 0xFFFFFFFFUL);
void lvgl_port_unlock(void);

lv_display_t *lvgl_port_get_display(void);
lv_indev_t *lvgl_port_get_touch(void);