#pragma once

#include <stdbool.h>
#include <stdint.h>

typedef struct {
    int32_t status;
    uint32_t valid_count;
    uint32_t zero_count;
    uint32_t error_count;
    int32_t raw_min;
    int32_t raw_max;
    int32_t raw_avg;
    int32_t mv_avg;
    uint8_t calibrated;
} st77916_adc_probe_result_t;

typedef struct {
    uint8_t second;
    uint8_t minute;
    uint8_t hour;
    uint8_t day;
    uint8_t month;
    uint8_t year;
} st77916_datetime_t;

typedef struct {
    int32_t status;
    uint32_t file_count;
    uint32_t first_file_size;
    uint32_t first_frame_offset;
    uint32_t first_frame_size;
    char first_name[32];
} st77916_mjpeg_probe_result_t;

typedef struct {
    int32_t status;
    uint32_t file_count;
    uint32_t first_file_size;
    uint32_t first_frame_offset;
    uint32_t first_frame_size;
    uint16_t jpeg_width;
    uint16_t jpeg_height;
    uint16_t output_width;
    uint16_t output_height;
    uint16_t preview_x;
    uint16_t preview_y;
    uint8_t scale_div;
    uint8_t color_swap;
    uint32_t decoded_frame_index;
    char first_name[32];
} st77916_mjpeg_decode_result_t;

#ifdef __cplusplus
extern "C" {
#endif

bool st77916_panel_init(void);
bool st77916_panel_draw_rgb565(uint16_t x0, uint16_t y0, uint16_t x1, uint16_t y1, uint16_t *color);
bool st77916_probe_sd_capacity_mb(bool *out_present, uint32_t *out_capacity_mb);
bool st77916_probe_sd_space_mb(bool *out_present, uint32_t *out_total_mb, uint32_t *out_free_mb);
int32_t st77916_read_sd_wifi_txt(uint8_t *out_buf, uint32_t out_len);
void st77916_time_configure_eastern(void);
void st77916_sntp_start(void);
bool st77916_sntp_is_synced(void);
int64_t st77916_time_epoch(void);
bool st77916_get_local_datetime(st77916_datetime_t *out_dt);
int32_t st77916_http_get(const char *url, uint8_t *out_buf, uint32_t out_len);
int32_t st77916_read_sd_weather_txt(uint8_t *out_buf, uint32_t out_len);
int32_t st77916_read_sd_battery_txt(uint8_t *out_buf, uint32_t out_len);
int32_t st77916_write_sd_weather_txt(const uint8_t *data, uint32_t data_len);
bool st77916_adc1_gpio8_oneshot_probe(uint32_t sample_count, st77916_adc_probe_result_t *out_result);
bool st77916_gpio_input_pullup(int32_t gpio_num);
int32_t st77916_gpio_get_level(int32_t gpio_num);
void st77916_configure_runtime_logs(bool debug_enabled);
int32_t st77916_read_sd_log_txt(uint8_t *out_buf, uint32_t out_len);
int32_t st77916_read_sd_asset_rgb565(const char *asset_name, uint8_t *out_buf, uint32_t out_len);
bool st77916_probe_sd_mjpeg_library(st77916_mjpeg_probe_result_t *out_result);
bool st77916_decode_first_mjpeg_frame_rgb565(st77916_mjpeg_decode_result_t *out_result, uint16_t *out_rgb565, uint32_t out_pixels);
bool st77916_decode_mjpeg_frame_rgb565(st77916_mjpeg_decode_result_t *out_result, uint16_t *out_rgb565, uint32_t out_pixels, uint32_t frame_index);
bool st77916_video_worker_start(uint32_t frame_step, uint32_t frame_ms);
void st77916_video_worker_stop(void);
void st77916_video_worker_request_next(void);
bool st77916_video_worker_copy_latest(uint16_t *out_rgb565, uint32_t out_pixels, st77916_mjpeg_decode_result_t *out_result);
uint32_t st77916_video_worker_state(void);
uint32_t st77916_video_worker_frame_ms(void);
uint32_t st77916_video_worker_display_fps(void);
uint32_t st77916_video_worker_source_skip(void);
const char *st77916_sd_owner_status(void);
bool st77916_sd_persistent_mount_session(void);
bool st77916_sd_persistent_is_ready(void);
uint32_t st77916_sd_persistent_mount_count(void);
uint32_t st77916_sd_owner_busy_count(void);

#ifdef __cplusplus
}
#endif