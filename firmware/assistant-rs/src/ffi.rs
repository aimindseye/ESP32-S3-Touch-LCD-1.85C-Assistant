#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct St77916AdcProbeResult {
    pub status: i32,
    pub valid_count: u32,
    pub zero_count: u32,
    pub error_count: u32,
    pub raw_min: i32,
    pub raw_max: i32,
    pub raw_avg: i32,
    pub mv_avg: i32,
    pub calibrated: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct St77916DateTime {
    pub second: u8,
    pub minute: u8,
    pub hour: u8,
    pub day: u8,
    pub month: u8,
    pub year: u8,
}

unsafe extern "C" {
    pub fn st77916_panel_init() -> bool;
    pub fn st77916_panel_draw_rgb565(x0: u16, y0: u16, x1: u16, y1: u16, color: *mut u16) -> bool;
    pub fn st77916_probe_sd_capacity_mb(out_present: *mut bool, out_capacity_mb: *mut u32) -> bool;
    pub fn st77916_probe_sd_space_mb(
        out_present: *mut bool,
        out_total_mb: *mut u32,
        out_free_mb: *mut u32,
    ) -> bool;
    pub fn st77916_read_sd_wifi_txt(out_buf: *mut u8, out_len: u32) -> i32;
    pub fn st77916_time_configure_eastern();
    pub fn st77916_sntp_start();
    pub fn st77916_sntp_is_synced() -> bool;
    pub fn st77916_time_epoch() -> i64;
    pub fn st77916_get_local_datetime(out_dt: *mut St77916DateTime) -> bool;
    pub fn st77916_http_get(url: *const core::ffi::c_char, out_buf: *mut u8, out_len: u32) -> i32;
    pub fn st77916_read_sd_weather_txt(out_buf: *mut u8, out_len: u32) -> i32;
    pub fn st77916_read_sd_battery_txt(out_buf: *mut u8, out_len: u32) -> i32;
    pub fn st77916_write_sd_weather_txt(data: *const u8, data_len: u32) -> i32;
    pub fn st77916_adc1_gpio8_oneshot_probe(
        sample_count: u32,
        out_result: *mut St77916AdcProbeResult,
    ) -> bool;
    pub fn st77916_gpio_input_pullup(gpio_num: i32) -> bool;
    pub fn st77916_gpio_get_level(gpio_num: i32) -> i32;
    pub fn st77916_configure_runtime_logs(debug_enabled: bool);
    pub fn st77916_read_sd_log_txt(out_buf: *mut u8, out_len: u32) -> i32;
    pub fn st77916_read_sd_asset_rgb565(
        asset_name: *const core::ffi::c_char,
        out_buf: *mut u8,
        out_len: u32,
    ) -> i32;
    pub fn st77916_sd_owner_status() -> *const ::core::ffi::c_char;
    pub fn st77916_sd_persistent_mount_session() -> bool;
    pub fn st77916_sd_persistent_is_ready() -> bool;
    pub fn st77916_sd_persistent_mount_count() -> u32;
    pub fn st77916_sd_owner_busy_count() -> u32;
    pub fn st77916_audio_pcm_init(sample_rate: u32, bits_per_sample: u16, channels: u16) -> bool;
    pub fn st77916_audio_pcm_write(data: *const u8, len: u32, timeout_ms: u32) -> i32;
    pub fn st77916_audio_pcm_stop();
    pub fn st77916_audio_pcm_is_ready() -> bool;
    pub fn st77916_audio_mp3_helix_play_file(
        path: *const ::core::ffi::c_char,
        volume_percent: u32,
    ) -> i32;
    pub fn st77916_audio_mp3_helix_stop_request();
    pub fn st77916_audio_mp3_helix_set_volume(volume_percent: u32);
    pub fn st77916_audio_mp3_helix_progress_percent() -> u32;
    pub fn st77916_audio_mp3_helix_elapsed_seconds() -> u32;
    pub fn st77916_audio_mp3_helix_duration_seconds() -> u32;
    pub fn st77916_radio_http_mp3_play(
        url: *const ::core::ffi::c_char,
        station_name: *const ::core::ffi::c_char,
        volume_percent: u32,
    ) -> i32;
    pub fn st77916_radio_http_mp3_stop_request();
    pub fn st77916_radio_http_mp3_set_volume(volume_percent: u32);
    pub fn st77916_radio_http_mp3_elapsed_seconds() -> u32;
    pub fn st77916_radio_http_mp3_buffered_bytes() -> u32;
    pub fn st77916_radio_http_mp3_status_code() -> u32;
}

// RAW-R42-VIDEO-RUST-FFI-REMOVED
