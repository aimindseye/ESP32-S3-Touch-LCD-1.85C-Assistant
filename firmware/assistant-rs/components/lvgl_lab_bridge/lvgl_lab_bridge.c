#include "lvgl_lab_bridge.h"

// LVGL_LAB_STUB_RENDERER: default safe bridge path; real esp_lvgl_port is staged but disabled.

#include <math.h>
#include <stddef.h>
#include <stdint.h>

#define RGB565(r, g, b) (uint16_t)((((r) & 0xF8) << 8) | (((g) & 0xFC) << 3) | ((b) >> 3))

static void put_px(uint16_t *frame, int32_t width, int32_t height, int32_t x, int32_t y, uint16_t color) {
    if (!frame || x < 0 || y < 0 || x >= width || y >= height) {
        return;
    }
    frame[(size_t)y * (size_t)width + (size_t)x] = color;
}

static void fill_rect(uint16_t *frame, int32_t width, int32_t height, int32_t x, int32_t y, int32_t w, int32_t h, uint16_t color) {
    for (int32_t yy = y; yy < y + h; ++yy) {
        for (int32_t xx = x; xx < x + w; ++xx) {
            put_px(frame, width, height, xx, yy, color);
        }
    }
}

static void fill_circle(uint16_t *frame, int32_t width, int32_t height, int32_t cx, int32_t cy, int32_t r, uint16_t color) {
    int32_t rr = r * r;
    for (int32_t y = cy - r; y <= cy + r; ++y) {
        for (int32_t x = cx - r; x <= cx + r; ++x) {
            int32_t dx = x - cx;
            int32_t dy = y - cy;
            if (dx * dx + dy * dy <= rr) {
                put_px(frame, width, height, x, y, color);
            }
        }
    }
}

static void stroke_circle(uint16_t *frame, int32_t width, int32_t height, int32_t cx, int32_t cy, int32_t r, uint16_t color) {
    for (int32_t deg = 0; deg < 360; ++deg) {
        float a = ((float)deg) * 0.01745329252f;
        int32_t x = cx + (int32_t)lroundf(cosf(a) * (float)r);
        int32_t y = cy + (int32_t)lroundf(sinf(a) * (float)r);
        put_px(frame, width, height, x, y, color);
    }
}

static void draw_arc(uint16_t *frame, int32_t width, int32_t height, int32_t cx, int32_t cy, int32_t r, int32_t start, int32_t end, uint16_t color) {
    int32_t deg = start;
    int32_t guard = 0;
    while (guard <= 360) {
        float a = ((float)deg) * 0.01745329252f;
        int32_t x = cx + (int32_t)lroundf(cosf(a) * (float)r);
        int32_t y = cy + (int32_t)lroundf(sinf(a) * (float)r);
        put_px(frame, width, height, x, y, color);
        put_px(frame, width, height, x + 1, y, color);
        put_px(frame, width, height, x, y + 1, color);
        if (((deg % 360) + 360) % 360 == ((end % 360) + 360) % 360) {
            break;
        }
        deg = (deg + 1) % 360;
        ++guard;
    }
}

static void draw_demo_letter_l(uint16_t *frame, int32_t width, int32_t height, int32_t x, int32_t y, uint16_t color) {
    fill_rect(frame, width, height, x, y, 8, 52, color);
    fill_rect(frame, width, height, x, y + 44, 34, 8, color);
}

static void draw_demo_letter_v(uint16_t *frame, int32_t width, int32_t height, int32_t x, int32_t y, uint16_t color) {
    for (int32_t i = 0; i < 26; ++i) {
        fill_rect(frame, width, height, x + i / 2, y + i * 2, 6, 4, color);
        fill_rect(frame, width, height, x + 34 - i / 2, y + i * 2, 6, 4, color);
    }
}

static void draw_demo_letter_g(uint16_t *frame, int32_t width, int32_t height, int32_t x, int32_t y, uint16_t color) {
    fill_rect(frame, width, height, x + 8, y, 30, 8, color);
    fill_rect(frame, width, height, x, y + 8, 8, 36, color);
    fill_rect(frame, width, height, x + 8, y + 44, 30, 8, color);
    fill_rect(frame, width, height, x + 34, y + 28, 8, 20, color);
    fill_rect(frame, width, height, x + 22, y + 28, 18, 8, color);
}

static void draw_text_lvgl(uint16_t *frame, int32_t width, int32_t height, int32_t x, int32_t y, uint16_t color) {
    draw_demo_letter_l(frame, width, height, x, y, color);
    draw_demo_letter_v(frame, width, height, x + 44, y, color);
    draw_demo_letter_g(frame, width, height, x + 92, y, color);
    draw_demo_letter_l(frame, width, height, x + 144, y, color);
}

bool lvgl_lab_bridge_available(void) {
    return true;
}

bool lvgl_lab_bridge_real_lvgl_enabled(void) {
    return false;
}

bool lvgl_lab_render_test_rgb565(uint16_t *frame, int32_t width, int32_t height) {
    if (!frame || width != 360 || height != 360) {
        return false;
    }

    const int32_t cx = 180;
    const int32_t cy = 180;

    uint16_t bg_top = RGB565(2, 8, 18);
    uint16_t bg_bottom = RGB565(4, 42, 90);
    for (int32_t y = 0; y < height; ++y) {
        uint8_t r = (uint8_t)(2 + (2 * y) / height);
        uint8_t g = (uint8_t)(8 + (34 * y) / height);
        uint8_t b = (uint8_t)(18 + (72 * y) / height);
        uint16_t c = RGB565(r, g, b);
        for (int32_t x = 0; x < width; ++x) {
            int32_t dx = x - cx;
            int32_t dy = y - cy;
            if (dx * dx + dy * dy <= 176 * 176) {
                frame[(size_t)y * (size_t)width + (size_t)x] = c;
            } else {
                frame[(size_t)y * (size_t)width + (size_t)x] = 0;
            }
        }
    }

    (void)bg_top;
    (void)bg_bottom;

    stroke_circle(frame, width, height, cx, cy, 176, RGB565(20, 34, 46));
    draw_arc(frame, width, height, cx, cy, 166, 210, 330, RGB565(25, 205, 245));
    draw_arc(frame, width, height, cx, cy, 128, 135, 45, RGB565(120, 80, 245));
    draw_arc(frame, width, height, cx, cy, 116, 135, 315, RGB565(60, 230, 245));

    fill_circle(frame, width, height, cx, 168, 58, RGB565(8, 20, 42));
    stroke_circle(frame, width, height, cx, 168, 58, RGB565(80, 230, 245));
    fill_circle(frame, width, height, cx, 168, 42, RGB565(18, 55, 120));

    draw_text_lvgl(frame, width, height, 88, 122, RGB565(245, 250, 255));

    fill_circle(frame, width, height, cx, 276, 28, RGB565(25, 165, 245));
    fill_rect(frame, width, height, cx - 6, 263, 12, 22, RGB565(255, 255, 255));
    fill_rect(frame, width, height, cx - 1, 285, 2, 12, RGB565(255, 255, 255));

    return true;
}
