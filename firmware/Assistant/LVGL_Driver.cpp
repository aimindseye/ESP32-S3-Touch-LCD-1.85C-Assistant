#include "LVGL_Driver.h"

#include <cstring>

#include "esp_heap_caps.h"
#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"

namespace {
constexpr uint32_t kLoopDelayMs = 5;
constexpr uint32_t kPaletteReserveBytes = 8;
constexpr uint32_t kDrawBufferLines = 40;

static SemaphoreHandle_t s_lvgl_mutex = nullptr;
static lv_display_t *s_display = nullptr;
static lv_indev_t *s_touch = nullptr;
static uint8_t *s_buf1 = nullptr;
static uint8_t *s_buf2 = nullptr;

static uint32_t lvgl_tick_cb(void) {
  return millis();
}

static void lvgl_flush_cb(lv_display_t *disp, const lv_area_t *area, uint8_t *px_map) {
  if (disp == nullptr || area == nullptr || px_map == nullptr) {
    if (disp) lv_display_flush_ready(disp);
    return;
  }

  int32_t x1 = area->x1;
  int32_t y1 = area->y1;
  int32_t x2 = area->x2;
  int32_t y2 = area->y2;

  if (x1 < 0) x1 = 0;
  if (y1 < 0) y1 = 0;
  if (x2 >= EXAMPLE_LCD_WIDTH)  x2 = EXAMPLE_LCD_WIDTH - 1;
  if (y2 >= EXAMPLE_LCD_HEIGHT) y2 = EXAMPLE_LCD_HEIGHT - 1;

  if (x2 < x1 || y2 < y1) {
    lv_display_flush_ready(disp);
    return;
  }

  px_map += kPaletteReserveBytes;

  LCD_addWindow(
    (uint16_t)x1,
    (uint16_t)y1,
    (uint16_t)x2,
    (uint16_t)y2,
    reinterpret_cast<uint16_t *>(px_map)
  );

  lv_display_flush_ready(disp);
}

static void lvgl_touch_read_cb(lv_indev_t *indev, lv_indev_data_t *data) {
  LV_UNUSED(indev);

  Touch_Read_Data();

  if (touch_data.points > 0) {
    uint16_t x = touch_data.x;
    uint16_t y = touch_data.y;

    if (x >= EXAMPLE_LCD_WIDTH)  x = EXAMPLE_LCD_WIDTH - 1;
    if (y >= EXAMPLE_LCD_HEIGHT) y = EXAMPLE_LCD_HEIGHT - 1;

    data->state = LV_INDEV_STATE_PRESSED;
    data->point.x = x;
    data->point.y = y;
  } else {
    data->state = LV_INDEV_STATE_RELEASED;
  }

  touch_data.points = 0;
  touch_data.gesture = NONE;
  touch_data.x = 0;
  touch_data.y = 0;
}
}  // namespace

bool lvgl_port_lock(uint32_t timeout_ms) {
  if (s_lvgl_mutex == nullptr) return true;

  const TickType_t ticks =
    (timeout_ms == 0xFFFFFFFFUL) ? portMAX_DELAY : pdMS_TO_TICKS(timeout_ms);

  return xSemaphoreTakeRecursive(s_lvgl_mutex, ticks) == pdTRUE;
}

void lvgl_port_unlock(void) {
  if (s_lvgl_mutex) {
    xSemaphoreGiveRecursive(s_lvgl_mutex);
  }
}

lv_display_t *lvgl_port_get_display(void) {
  return s_display;
}

lv_indev_t *lvgl_port_get_touch(void) {
  return s_touch;
}

void Lvgl_Init(void) {
  if (s_display != nullptr) return;

  lv_init();
  lv_tick_set_cb(lvgl_tick_cb);

  s_lvgl_mutex = xSemaphoreCreateRecursiveMutex();
  if (s_lvgl_mutex == nullptr) {
    Serial.println("Failed to create LVGL mutex");
    while (true) delay(1000);
  }

  const size_t color_bytes =
    EXAMPLE_LCD_WIDTH * kDrawBufferLines * sizeof(uint16_t);
  const size_t buffer_bytes = color_bytes + kPaletteReserveBytes;

  s_buf1 = static_cast<uint8_t *>(
    heap_caps_malloc(buffer_bytes, MALLOC_CAP_INTERNAL | MALLOC_CAP_DMA)
  );
  s_buf2 = static_cast<uint8_t *>(
    heap_caps_malloc(buffer_bytes, MALLOC_CAP_INTERNAL | MALLOC_CAP_DMA)
  );

  if (s_buf1 == nullptr || s_buf2 == nullptr) {
    Serial.println("Failed to allocate LVGL draw buffers");
    while (true) delay(1000);
  }

  memset(s_buf1, 0, buffer_bytes);
  memset(s_buf2, 0, buffer_bytes);

  s_display = lv_display_create(EXAMPLE_LCD_WIDTH, EXAMPLE_LCD_HEIGHT);
  lv_display_set_color_format(s_display, LV_COLOR_FORMAT_RGB565);
  lv_display_set_buffers(
    s_display,
    s_buf1,
    s_buf2,
    buffer_bytes,
    LV_DISPLAY_RENDER_MODE_PARTIAL
  );
  lv_display_set_flush_cb(s_display, lvgl_flush_cb);

  s_touch = lv_indev_create();
  lv_indev_set_type(s_touch, LV_INDEV_TYPE_POINTER);
  lv_indev_set_read_cb(s_touch, lvgl_touch_read_cb);
  lv_indev_set_display(s_touch, s_display);
}

void Lvgl_Loop(void) {
  if (s_display == nullptr) {
    delay(kLoopDelayMs);
    return;
  }

  if (lvgl_port_lock(50)) {
    lv_timer_handler();
    lvgl_port_unlock();
  }

  delay(kLoopDelayMs);
}