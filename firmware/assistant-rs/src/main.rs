#![allow(dead_code)]
mod screens;

// RAW-R50-SCREEN-MODULES-NO-INCLUDES
mod app;
mod audio_foundation;
mod board;
mod boot_report;
mod drivers;
mod ffi;
mod internet_radio;
mod internet_radio_screen;
mod media_controls;
mod music_screen;

use anyhow::{bail, Result};
use app::{
    actions::{handle_select_action, AppAction},
    intents::UiIntent,
    model::{ButtonName, ButtonPressKind},
    pages::{AssistantPage, ALL_PAGES},
    providers::LocalProviders,
    settings::SettingsPanel,
    state::AppState,
    time::TimeSyncPhase,
    weather::{condition_from_weather_code, WeatherHourSlot, WeatherSample},
    wifi::{WifiCredentials, WifiProvisioningStep},
};
use core::{ffi::c_void, mem::size_of, ptr::NonNull, slice};
use drivers::{
    cst816::Cst816,
    pcf85063::{DateTime, Pcf85063},
    tca9554::Tca9554,
};
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration, Wifi};
use esp_idf_hal::{
    adc::{
        attenuation,
        oneshot::{config::AdcChannelConfig, AdcChannelDriver, AdcDriver},
    },
    gpio::{PinDriver, Pull},
    i2c::{I2cConfig, I2cDriver},
    peripherals::Peripherals,
    prelude::*,
    sys::adc_atten_t,
};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition, wifi::EspWifi};
use esp_idf_sys::{
    self, heap_caps_free, heap_caps_get_free_size, heap_caps_malloc, MALLOC_CAP_8BIT,
    MALLOC_CAP_SPIRAM,
};
use std::{
    ffi::CString,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

const W: usize = 360;
const H: usize = 360;
const PIXELS: usize = W * H;
const RGB565_ASSET_BYTES: usize = PIXELS * 2;
const UI_ASSET_COUNT: usize = 5;
const UI_ASSET_EMBEDDED_BYTES_REMOVED: usize = RGB565_ASSET_BYTES * UI_ASSET_COUNT;
const APP_BINARY_BEFORE_R7_BYTES: usize = 2_616_176;
const APP_PARTITION_BYTES: usize = 3_145_728;
const CX: i32 = 180;
const CY: i32 = 180;
const R_OUTER: i32 = 178;

const BLACK: u16 = 0x0000;
const BG: u16 = 0x0841;
const BG_DARK: u16 = 0x0008;
const BG_BLUE: u16 = 0x0296;
const RING: u16 = 0x18C3;
const RING_DIM: u16 = 0x1082;
const WHITE: u16 = 0xFFFF;
const MUTED: u16 = 0x9CF3;
const SOFT: u16 = 0x5AEB;

const ACCENT_HOME: u16 = 0x05DF;
const ACCENT_HOME_GREEN: u16 = 0x05E0;
const ACCENT_HOME_BLUE: u16 = 0x02DF;
const ACCENT_WEATHER: u16 = 0xFDE0;
const ACCENT_WEATHER_BLUE: u16 = 0x449F;
const ACCENT_MUSIC: u16 = 0x7818;
const ACCENT_MUSIC_BLUE: u16 = 0x22DF;
const ACCENT_ASSISTANT: u16 = 0x05FF;
const ACCENT_ASSISTANT_BLUE: u16 = 0x035F;
const ACCENT_SETTINGS: u16 = 0x44BF;
const STATUS_OK: u16 = 0x07E0;
const STATUS_BAD: u16 = 0xF800;

const TOP_STATUS_Y: i32 = 18;
const TITLE_Y: i32 = 70;
const SUBTITLE_Y: i32 = 112;
const FOOTER_DOTS_Y: i32 = 324;

const TOUCH_POLL_MS: u64 = 8;
const BUTTON_POLL_MS: u64 = 25;
const POWER_LONG_PRESS_MS: u64 = 850;
const BUTTON_DEBOUNCE_MS: u64 = 40;
const BATTERY_REFRESH_MS: u64 = 5_000;
const BATTERY_ADC_SAMPLE_COUNT: u8 = 8;
const BATTERY_ADC_SAMPLE_DELAY_MS: u64 = 2;
const BATTERY_ADC_PROBE_SAMPLE_COUNT: u8 = 8;
const BATTERY_ADC_PROBE_DELAY_MS: u64 = 2;
const WIFI_REFRESH_MS: u64 = 30_000;
const WIFI_CONNECT_POLL_MS: u64 = 1_000;
const WIFI_CONNECT_TIMEOUT_MS: u64 = 15_000;
const WIFI_TXT_MAX_BYTES: usize = 512;
const BATTERY_TXT_MAX_BYTES: usize = 512;
const WEATHER_HTTP_MAX_BYTES: usize = 4096;
const WEATHER_CACHE_MAX_BYTES: usize = 1024;
const LOG_TXT_MAX_BYTES: usize = 64;
const LOG_PROFILE_COMPILE_DEBUG: bool = false;
const RTC_REFRESH_MS: u64 = 1_000;
const TIME_SYNC_POLL_MS: u64 = 1_000;
const TIME_SYNC_TIMEOUT_MS: u64 = 45_000;
const TOUCH_TAP_MAX_MS: u64 = 450;
const TAP_MOVEMENT_MAX_PX: i16 = 25;
const TOUCH_NAV_COOLDOWN_MS: u64 = 300;
const TOUCH_ACTIVE_POLL_WINDOW_MS: u64 = 180;
const TOUCH_NO_TOUCH_FINISH_COUNT: u8 = 3;
const TOUCH_GESTURE_SPAN_PREFER_PX: i16 = 35;
const UNIVERSAL_SWIPE_MIN_DX: i16 = 20;
const SETTINGS_DETAIL_VERTICAL_SWIPE_MIN_DY: i16 = 20;
const SETTINGS_DETAIL_HEADER_TAP_Y_MAX: u16 = 92;
const CENTER_TAP_X_MIN: u16 = 95;
const CENTER_TAP_X_MAX: u16 = 265;
const CENTER_TAP_Y_MIN: u16 = 95;
const CENTER_TAP_Y_MAX: u16 = 285;
const CENTER_TAP_MAX_MOVE_PX: i16 = 12;
const CST816_GESTURE_LEFT: u8 = 0x03;
const CST816_GESTURE_RIGHT: u8 = 0x04;
const RENDER_MIN_INTERVAL_MS: u64 = 80;
const SCREEN_SLEEP_IDLE_MS: u64 = 120_000;
const SCREEN_WAKE_GUARD_MS: u64 = 700;
const SOFTWARE_SLEEP_CORNER_HOLD_MS: u64 = 900;
const SOFTWARE_SLEEP_CORNER_X_MAX: u16 = 58;
const SOFTWARE_SLEEP_CORNER_Y_MAX: u16 = 58;
const SOFTWARE_SLEEP_CORNER_MOVE_MAX: i16 = 22;
const POWER_GPIO_DIAG_MS: u64 = 250;
const BUTTON_DISCOVERY_HOLD_REPORT_MS: u64 = 850;
const EXIO_INPUT_POLL_MS: u64 = 250;

// Rust esp-idf-hal path remains diagnostic-only; production battery source is vendor C-shim DB_12.
const BAT_ATTEN: adc_atten_t = attenuation::DB_11;

static LOG_DEBUG_ENABLED: AtomicBool = AtomicBool::new(LOG_PROFILE_COMPILE_DEBUG);

macro_rules! debug_println {
    ($($arg:tt)*) => {{
        if log_debug_enabled() {
            println!($($arg)*);
        }
    }};
}

mod settings_action_router;
mod weather_action_router;

mod media_action_router;

mod page_assets;

mod page_orchestration;
mod touch_router;
fn log_debug_enabled() -> bool {
    LOG_DEBUG_ENABLED.load(Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeLogProfile {
    Normal,
    Debug,
}

impl RuntimeLogProfile {
    const fn from_debug(debug: bool) -> Self {
        if debug {
            Self::Debug
        } else {
            Self::Normal
        }
    }

    const fn is_debug(self) -> bool {
        matches!(self, Self::Debug)
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Debug => "DEBUG",
        }
    }
}

fn set_runtime_log_profile(profile: RuntimeLogProfile, source: &'static str) {
    LOG_DEBUG_ENABLED.store(profile.is_debug(), Ordering::Relaxed);
    unsafe {
        ffi::st77916_configure_runtime_logs(profile.is_debug());
    }
    println!(
        "log-profile: profile={} source={} debug={} normal_suppresses=touch-samples,battery-diagnostics,gpio-exio-discovery,render-ok",
        profile.label(),
        source,
        profile.is_debug()
    );
}

fn parse_runtime_log_profile_text(text: &str) -> Option<RuntimeLogProfile> {
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let value = if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_ascii_uppercase();
            if key != "LOG" && key != "PROFILE" && key != "VERBOSITY" {
                continue;
            }
            value.trim()
        } else {
            line
        };

        match value.trim().to_ascii_uppercase().as_str() {
            "DEBUG" | "VERBOSE" | "TRACE" => return Some(RuntimeLogProfile::Debug),
            "NORMAL" | "INFO" | "QUIET" => return Some(RuntimeLogProfile::Normal),
            _ => {}
        }
    }

    None
}

fn load_runtime_log_profile_from_sd() -> Option<RuntimeLogProfile> {
    let mut buf = [0_u8; LOG_TXT_MAX_BYTES];
    let read_len = unsafe { ffi::st77916_read_sd_log_txt(buf.as_mut_ptr(), buf.len() as u32) };
    if read_len <= 0 {
        return None;
    }

    let len = (read_len as usize).min(buf.len());
    let Ok(text) = core::str::from_utf8(&buf[..len]) else {
        return None;
    };

    parse_runtime_log_profile_text(text)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonEvent {
    Short,
    Long,
}

struct ButtonTracker {
    down: bool,
    pressed_at: Option<Instant>,
    long_sent: bool,
    long_press: Duration,
}

impl ButtonTracker {
    fn new(long_press: Duration) -> Self {
        Self {
            down: false,
            pressed_at: None,
            long_sent: false,
            long_press,
        }
    }

    fn update(&mut self, now: Instant, is_down: bool) -> Option<ButtonEvent> {
        if is_down {
            if !self.down {
                self.down = true;
                self.pressed_at = Some(now);
                self.long_sent = false;
                return None;
            }

            if !self.long_sent {
                if let Some(start) = self.pressed_at {
                    if now.duration_since(start) >= self.long_press {
                        self.long_sent = true;
                        return Some(ButtonEvent::Long);
                    }
                }
            }

            None
        } else if self.down {
            self.down = false;
            let elapsed = self
                .pressed_at
                .map(|start| now.duration_since(start))
                .unwrap_or_default();
            self.pressed_at = None;

            if !self.long_sent && elapsed >= Duration::from_millis(BUTTON_DEBOUNCE_MS) {
                Some(ButtonEvent::Short)
            } else {
                None
            }
        } else {
            None
        }
    }
}
fn power_gpio_level() -> i32 {
    unsafe { ffi::st77916_gpio_get_level(board::POWER_BUTTON_GPIO as i32) }
}

fn power_gpio_down_from_level(level: i32) -> bool {
    level == 0
}

fn power_gpio_duration_ms(now: Instant, pressed_since: Option<Instant>) -> u128 {
    pressed_since
        .map(|started| now.duration_since(started).as_millis())
        .unwrap_or(0)
}

#[derive(Clone, Copy)]
struct ButtonPinCandidate {
    gpio: i32,
    label: &'static str,
    origin: &'static str,
    active_low_expected: bool,
    sleep_bind_allowed: bool,
    guard: &'static str,
}

#[derive(Clone, Copy)]
struct ButtonPinState {
    boot_level: i32,
    last_level: i32,
    active_since: Option<Instant>,
    last_diag: Instant,
    long_reported: bool,
    noisy_at_boot: bool,
}

const BUTTON_PIN_CANDIDATES: [ButtonPinCandidate; 4] = [
    ButtonPinCandidate {
        gpio: 0,
        label: "BOOT",
        origin: "ESP32-S3_BOOT",
        active_low_expected: true,
        sleep_bind_allowed: false,
        guard: "USB_FLASHING_GUARD",
    },
    ButtonPinCandidate {
        gpio: 6,
        label: "POWER_ASSUMED",
        origin: "ASSISTANT_R1_ASSUMPTION",
        active_low_expected: true,
        sleep_bind_allowed: false,
        guard: "UNCONFIRMED_REPORT_ONLY",
    },
    ButtonPinCandidate {
        gpio: 7,
        label: "GPIO7_FREE_CANDIDATE",
        origin: "V1_ADC_PROBE_FREE_PIN",
        active_low_expected: true,
        sleep_bind_allowed: false,
        guard: "UNCONFIRMED_REPORT_ONLY",
    },
    ButtonPinCandidate {
        gpio: 9,
        label: "GPIO9_FREE_CANDIDATE",
        origin: "V1_ADC_PROBE_FREE_PIN",
        active_low_expected: true,
        sleep_bind_allowed: false,
        guard: "UNCONFIRMED_REPORT_ONLY",
    },
];

fn button_candidate_level(candidate: ButtonPinCandidate) -> i32 {
    unsafe { ffi::st77916_gpio_get_level(candidate.gpio) }
}

fn button_candidate_expected_down(candidate: ButtonPinCandidate, level: i32) -> bool {
    if candidate.active_low_expected {
        level == 0
    } else {
        level == 1
    }
}

fn button_candidate_duration_ms(now: Instant, pressed_since: Option<Instant>) -> u128 {
    pressed_since
        .map(|started| now.duration_since(started).as_millis())
        .unwrap_or(0)
}

fn button_candidate_noisy_at_boot(candidate: ButtonPinCandidate, boot_level: i32) -> bool {
    (candidate.gpio == 7 || candidate.gpio == 9)
        && button_candidate_expected_down(candidate, boot_level)
}

fn button_candidate_transition_active(state: ButtonPinState, level: i32) -> bool {
    level != state.boot_level
}

fn exio_changed_bits_text(changed: u8) -> &'static str {
    match changed {
        0x00 => "none",
        0x01 => "bit0",
        0x02 => "bit1",
        0x04 => "bit2",
        0x08 => "bit3",
        0x10 => "bit4",
        0x20 => "bit5",
        0x40 => "bit6",
        0x80 => "bit7",
        _ => "multiple",
    }
}

#[derive(Debug, Clone, Copy)]
struct TouchSummary {
    touch_id: u32,
    sample_count: u16,
    finish_reason: &'static str,
    start_x: u16,
    start_y: u16,
    end_x: u16,
    end_y: u16,
    min_x: u16,
    max_x: u16,
    min_y: u16,
    max_y: u16,
    dx: i16,
    dy: i16,
    span_x: i16,
    span_y: i16,
    duration_ms: u128,
    gesture: u8,
}

#[derive(Debug, Default)]
struct TouchTracker {
    active: bool,
    touch_id: u32,
    next_touch_id: u32,
    sample_count: u16,
    no_touch_count: u8,
    start_x: u16,
    start_y: u16,
    last_x: u16,
    last_y: u16,
    min_x: u16,
    max_x: u16,
    min_y: u16,
    max_y: u16,
    start_at: Option<Instant>,
    last_seen: Option<Instant>,
    gesture: u8,
}

impl TouchTracker {
    fn reset_fields(&mut self) {
        self.active = false;
        self.sample_count = 0;
        self.no_touch_count = 0;
        self.start_x = 0;
        self.start_y = 0;
        self.last_x = 0;
        self.last_y = 0;
        self.min_x = 0;
        self.max_x = 0;
        self.min_y = 0;
        self.max_y = 0;
        self.start_at = None;
        self.last_seen = None;
        self.gesture = 0;
    }

    fn begin(&mut self, now: Instant, x: u16, y: u16, gesture: u8) {
        self.reset_fields();
        self.next_touch_id = self.next_touch_id.wrapping_add(1);
        if self.next_touch_id == 0 {
            self.next_touch_id = 1;
        }

        self.active = true;
        self.touch_id = self.next_touch_id;
        self.sample_count = 0;
        self.no_touch_count = 0;
        self.start_x = x;
        self.start_y = y;
        self.last_x = x;
        self.last_y = y;
        self.min_x = x;
        self.max_x = x;
        self.min_y = y;
        self.max_y = y;
        self.start_at = Some(now);
        self.last_seen = Some(now);
        self.gesture = gesture;

        debug_println!("touch-track: begin id={}", self.touch_id);
        self.record_sample(now, "int", x, y, gesture);
    }

    fn record_sample(&mut self, now: Instant, source: &'static str, x: u16, y: u16, gesture: u8) {
        self.last_x = x;
        self.last_y = y;
        self.min_x = self.min_x.min(x);
        self.max_x = self.max_x.max(x);
        self.min_y = self.min_y.min(y);
        self.max_y = self.max_y.max(y);
        self.last_seen = Some(now);
        self.no_touch_count = 0;
        self.sample_count = self.sample_count.saturating_add(1);

        if gesture != 0 {
            self.gesture = gesture;
        }

        debug_println!(
            "touch-track: sample id={} source={} x={} y={} gesture=0x{:02X}",
            self.touch_id,
            source,
            x,
            y,
            gesture
        );
    }

    fn update_down(&mut self, now: Instant, source: &'static str, x: u16, y: u16, gesture: u8) {
        if !self.active {
            self.begin(now, x, y, gesture);
        } else {
            self.record_sample(now, source, x, y, gesture);
        }
    }

    fn note_no_touch(&mut self) {
        if !self.active {
            return;
        }

        self.no_touch_count = self.no_touch_count.saturating_add(1);
        debug_println!(
            "touch-track: no-touch id={} count={}",
            self.touch_id,
            self.no_touch_count
        );
    }

    fn active_elapsed(&self, now: Instant) -> Duration {
        self.start_at
            .map(|start| now.duration_since(start))
            .unwrap_or_default()
    }

    fn software_sleep_corner_candidate(&self) -> bool {
        self.active
            && self.start_x <= SOFTWARE_SLEEP_CORNER_X_MAX
            && self.start_y <= SOFTWARE_SLEEP_CORNER_Y_MAX
            && self.span_x().abs() <= SOFTWARE_SLEEP_CORNER_MOVE_MAX
            && self.span_y().abs() <= SOFTWARE_SLEEP_CORNER_MOVE_MAX
    }

    fn software_sleep_corner_ready(&self, now: Instant) -> bool {
        self.software_sleep_corner_candidate()
            && self.active_elapsed(now) >= Duration::from_millis(SOFTWARE_SLEEP_CORNER_HOLD_MS)
    }

    fn span_x(&self) -> i16 {
        self.max_x as i16 - self.min_x as i16
    }

    fn span_y(&self) -> i16 {
        self.max_y as i16 - self.min_y as i16
    }

    fn finish_reason(&self, now: Instant) -> Option<&'static str> {
        if !self.active {
            return None;
        }

        if self.no_touch_count >= TOUCH_NO_TOUCH_FINISH_COUNT {
            return Some("no-touch");
        }

        if self.software_sleep_corner_candidate()
            && self.active_elapsed(now) < Duration::from_millis(SOFTWARE_SLEEP_CORNER_HOLD_MS)
        {
            return None;
        }

        if self.active_elapsed(now) >= Duration::from_millis(TOUCH_ACTIVE_POLL_WINDOW_MS) {
            return Some("window");
        }

        None
    }

    fn finish(&mut self, now: Instant, reason: &'static str) -> Option<TouchSummary> {
        if !self.active {
            return None;
        }

        let start = self.start_at.unwrap_or(now);
        let duration_ms = now.duration_since(start).as_millis();
        let summary = TouchSummary {
            touch_id: self.touch_id,
            sample_count: self.sample_count,
            finish_reason: reason,
            start_x: self.start_x,
            start_y: self.start_y,
            end_x: self.last_x,
            end_y: self.last_y,
            min_x: self.min_x,
            max_x: self.max_x,
            min_y: self.min_y,
            max_y: self.max_y,
            dx: self.last_x as i16 - self.start_x as i16,
            dy: self.last_y as i16 - self.start_y as i16,
            span_x: self.max_x as i16 - self.min_x as i16,
            span_y: self.max_y as i16 - self.min_y as i16,
            duration_ms,
            gesture: self.gesture,
        };

        debug_println!("touch-track: reset id={}", self.touch_id);
        self.reset_fields();

        Some(summary)
    }
}

struct FrameBuffer {
    ptr: NonNull<u16>,
    len_words: usize,
}

impl FrameBuffer {
    fn new_rgb565(len_words: usize) -> Result<Self> {
        let bytes = len_words
            .checked_mul(size_of::<u16>())
            .ok_or_else(|| anyhow::anyhow!("framebuffer size overflow"))?;

        let raw =
            unsafe { heap_caps_malloc(bytes, MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT) as *mut u16 };

        let ptr =
            NonNull::new(raw).ok_or_else(|| anyhow::anyhow!("PSRAM framebuffer alloc failed"))?;

        unsafe {
            core::ptr::write_bytes(ptr.as_ptr(), 0, len_words);
        }

        Ok(Self { ptr, len_words })
    }

    fn as_mut_slice(&mut self) -> &mut [u16] {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len_words) }
    }

    fn as_slice(&self) -> &[u16] {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len_words) }
    }

    fn as_mut_bytes(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr() as *mut u8, self.len_words * 2) }
    }
}

impl Drop for FrameBuffer {
    fn drop(&mut self) {
        unsafe {
            heap_caps_free(self.ptr.as_ptr() as *mut c_void);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UiAssetSource {
    Sd,
    Fallback,
}

struct UiAssetCache {
    page: Option<AssistantPage>,
    source: UiAssetSource,
    buffer: FrameBuffer,
}

impl UiAssetCache {
    fn new() -> Result<Self> {
        Ok(Self {
            page: None,
            source: UiAssetSource::Fallback,
            buffer: FrameBuffer::new_rgb565(W * H)?,
        })
    }

    fn ensure_page(&mut self, page: AssistantPage) -> UiAssetSource {
        if self.page == Some(page) {
            return self.source;
        }

        self.page = Some(page);
        let asset_name = ui_asset_name(page);
        let c_name = match CString::new(asset_name) {
            Ok(name) => name,
            Err(_) => {
                self.source = UiAssetSource::Fallback;
                return self.source;
            }
        };

        let read = unsafe {
            ffi::st77916_read_sd_asset_rgb565(
                c_name.as_ptr(),
                self.buffer.as_mut_bytes().as_mut_ptr(),
                RGB565_ASSET_BYTES as u32,
            )
        };

        if read == RGB565_ASSET_BYTES as i32 {
            self.source = UiAssetSource::Sd;
            debug_println!(
                "asset-cache: page={:?} source=SD path=/ASSETS/{} bytes={}",
                page,
                asset_name,
                read
            );
        } else {
            self.source = UiAssetSource::Fallback;
            debug_println!(
                "asset-cache: page={:?} source=FALLBACK asset={} reason={} expected_bytes={}",
                page,
                asset_name,
                asset_sd_error_label(read),
                RGB565_ASSET_BYTES
            );
        }

        self.source
    }

    fn copy_to(&self, frame: &mut [u16]) {
        if self.source == UiAssetSource::Sd {
            frame.copy_from_slice(self.buffer.as_slice());
        }
    }
}

fn ui_asset_name(page: AssistantPage) -> &'static str {
    match page {
        AssistantPage::Home => "HOME.RGB",
        AssistantPage::Weather => "WEATHER.RGB",
        AssistantPage::Music => "MUSIC.RGB",
        AssistantPage::InternetRadio => "MUSIC.RGB",
        AssistantPage::Assistant => "AI.RGB",
        AssistantPage::Settings => "SETTINGS.RGB",
    }
}

fn asset_sd_error_label(code: i32) -> &'static str {
    match code {
        -1 => "BAD_ARG",
        -2 => "SD_MOUNT",
        -3 => "NO_FILE",
        -4 => "SHORT_READ",
        -5 => "BAD_PATH",
        _ => "SD_FAIL",
    }
}

fn main() -> Result<()> {
    esp_idf_sys::link_patches();

    thread::Builder::new()
        .name("waveshare-ui".to_string())
        .stack_size(32 * 1024)
        .spawn(|| {
            if let Err(err) = run_app() {
                eprintln!("fatal app error: {err:?}");
            }

            loop {
                thread::sleep(Duration::from_secs(60));
            }
        })?;

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct BatteryCalibration {
    adc_multiplier: f32,
    empty_mv: u16,
    full_mv: u16,
}

impl BatteryCalibration {
    fn defaults() -> Self {
        Self {
            adc_multiplier: board::BATTERY_DIVIDER_SCALE / board::BATTERY_MEASUREMENT_OFFSET,
            empty_mv: 3000,
            full_mv: 4200,
        }
    }
}

fn battery_sample_from_raw(raw: u16, adc_multiplier: f32) -> (u16, u16) {
    let adc_mv = ((raw as f32 / 4095.0) * 3300.0).round() as u16;
    let battery_mv = ((adc_mv as f32) * adc_multiplier).round() as u16;
    (adc_mv, battery_mv)
}

fn note_battery_adc_sample(model: &mut AppState, raw: u16, source: &str) {
    if raw == 0 {
        let zero_count = model.note_battery_zero_sample();
        if model.battery_has_valid_sample() {
            println!(
                "battery-adc: source={} raw=0 rejected=YES zero_count={} action=KEEP_LAST_VALID last_raw={} last_adc_mv={} last_battery_mv={} pct={} estimate={} power={} cal={} mult={:.3} empty_mv={} full_mv={}",
                source,
                zero_count,
                model.battery_adc_raw_label(),
                model.battery_adc_mv_label(),
                model.battery_voltage_text(),
                model.battery_home_text(),
                model.battery_estimate_text(),
                model.battery_power_text(),
                model.battery_cal_status_text(),
                model.battery_adc_multiplier(),
                model.battery_empty_mv,
                model.battery_full_mv
            );
        } else {
            let marker = if zero_count >= 2 {
                "ADC_ZERO_REPEATED"
            } else {
                "ADC_ZERO_NO_VALID_SAMPLE"
            };
            println!(
                "battery-adc: source={} raw=0 rejected=YES zero_count={} marker={} action=KEEP_UNAVAILABLE pct=-- estimate={} power={} cal={} mult={:.3} empty_mv={} full_mv={}",
                source,
                zero_count,
                marker,
                model.battery_estimate_text(),
                model.battery_power_text(),
                model.battery_cal_status_text(),
                model.battery_adc_multiplier(),
                model.battery_empty_mv,
                model.battery_full_mv
            );
        }
        return;
    }

    let (adc_mv, battery_mv) = battery_sample_from_raw(raw, model.battery_adc_multiplier());
    model.note_battery_sample(raw, adc_mv, battery_mv);

    println!(
        "battery-adc: source={} raw={} adc_mv={} battery_mv={} pct={} estimate={} power={} cal={} mult={:.3} empty_mv={} full_mv={}",
        source,
        raw,
        adc_mv,
        battery_mv,
        model.battery_home_text(),
        model.battery_estimate_text(),
        model.battery_power_text(),
        model.battery_cal_status_text(),
        model.battery_adc_multiplier(),
        model.battery_empty_mv,
        model.battery_full_mv
    );
}

fn note_vendor_c_shim_battery_sample(
    model: &mut AppState,
    probe: &ffi::St77916AdcProbeResult,
    source: &str,
) {
    if probe.valid_count == 0 || probe.raw_avg <= 0 || probe.mv_avg <= 0 {
        println!(
            "battery-adc-source: source=c-shim-gpio8-vendor phase={} action=NO_PROMOTE reason=NO_VALID_C_SHIM_SAMPLE status={} valid={} zero={} error={} ui_source=NO",
            source,
            probe.status,
            probe.valid_count,
            probe.zero_count,
            probe.error_count
        );
        return;
    }

    let raw = probe.raw_avg.max(0).min(u16::MAX as i32) as u16;
    let adc_mv = probe.mv_avg.max(0).min(u16::MAX as i32) as u16;
    let battery_mv = ((adc_mv as f32) * model.battery_adc_multiplier()).round() as u16;
    model.note_battery_sample_with_source(
        raw,
        adc_mv,
        battery_mv,
        board::BATTERY_SOURCE_C_SHIM_VENDOR,
    );

    debug_println!(
        "battery-adc-source: source=c-shim-gpio8-vendor phase={} action=PROMOTE_TO_UI raw={} adc_mv={} battery_mv={} pct={} estimate={} cal={} mult={:.3} vendor_atten={} measurement_offset={:.6} ui_source=YES",
        source,
        raw,
        adc_mv,
        battery_mv,
        model.battery_home_text(),
        model.battery_estimate_text(),
        model.battery_cal_status_text(),
        model.battery_adc_multiplier(),
        board::BATTERY_VENDOR_ATTEN_TEXT,
        board::BATTERY_MEASUREMENT_OFFSET
    );
}

fn note_rust_adc_diagnostic_sample(model: &AppState, raw: u16, source: &str) {
    if raw == 0 {
        debug_println!(
            "battery-adc-rust-diagnostic: source={} raw=0 action=NO_UI_SOURCE current_source={} current_pct={} current_voltage={} cal={} mult={:.3}",
            source,
            model.battery_adc_source_text(),
            model.battery_home_text(),
            model.battery_voltage_text(),
            model.battery_cal_status_text(),
            model.battery_adc_multiplier()
        );
        return;
    }

    let (adc_mv, battery_mv) = battery_sample_from_raw(raw, model.battery_adc_multiplier());
    debug_println!(
        "battery-adc-rust-diagnostic: source={} raw={} adc_mv={} battery_mv={} action=NO_UI_SOURCE reason=VENDOR_C_SHIM_IS_PRODUCTION current_source={} current_pct={} current_voltage={} cal={} mult={:.3}",
        source,
        raw,
        adc_mv,
        battery_mv,
        model.battery_adc_source_text(),
        model.battery_home_text(),
        model.battery_voltage_text(),
        model.battery_cal_status_text(),
        model.battery_adc_multiplier()
    );
}

macro_rules! sample_battery_adc_batch {
    ($adc:expr, $pin:expr, $model:expr, $source:expr) => {{
        let mut valid_count: u16 = 0;
        let mut zero_count: u16 = 0;
        let mut error_count: u16 = 0;
        let mut min_raw: u16 = u16::MAX;
        let mut max_raw: u16 = 0;
        let mut sum_raw: u32 = 0;

        for sample_index in 0..BATTERY_ADC_SAMPLE_COUNT {
            match $adc.read(&mut $pin) {
                Ok(raw) if raw > 0 => {
                    valid_count = valid_count.saturating_add(1);
                    min_raw = min_raw.min(raw);
                    max_raw = max_raw.max(raw);
                    sum_raw = sum_raw.saturating_add(raw as u32);
                }
                Ok(_) => {
                    zero_count = zero_count.saturating_add(1);
                }
                Err(err) => {
                    error_count = error_count.saturating_add(1);
                    debug_println!(
                        "battery-adc-error: source={} sample={} marker=ADC_READ_ERROR err={:?}",
                        $source,
                        sample_index,
                        err
                    );
                }
            }

            if sample_index + 1 < BATTERY_ADC_SAMPLE_COUNT {
                thread::sleep(Duration::from_millis(BATTERY_ADC_SAMPLE_DELAY_MS));
            }
        }

        let rust_avg_raw: u16 = if valid_count > 0 {
            (sum_raw / valid_count as u32) as u16
        } else {
            0
        };

        if $source == "pre-i2c" {
            debug_println!(
                "battery-adc-parity: source={} action=SKIP_AFTER_RUST_ADC_OWNED reason=C_FIRST_ALREADY_LOGGED rust_unit=ADC1 rust_gpio=GPIO{} rust_atten=DB_11 rust_valid={} rust_zero={} rust_error={} rust_min={} rust_max={} rust_avg={} c_unit=ADC1 c_channel=7 c_gpio=GPIO8 c_status={} c_ok=false c_valid=0 c_zero=0 c_error=0 c_min=-1 c_max=-1 c_avg=-1 c_mv_avg={} c_calibrated={} ui_source=NO",
                $source,
                board::BATTERY_ADC_GPIO,
                valid_count,
                zero_count,
                error_count,
                if valid_count > 0 { min_raw as i32 } else { -1 },
                if valid_count > 0 { max_raw as i32 } else { -1 },
                rust_avg_raw,
                "SKIPPED",
                -1,
                0
            );
        }

        if valid_count > 0 {
            let avg_raw = rust_avg_raw;
            debug_println!(
                "battery-adc-batch: source={} unit=ADC1 gpio=GPIO{} attenuation=DB_11 samples={} valid={} zero={} error={} min={} max={} avg={}",
                $source,
                board::BATTERY_ADC_GPIO,
                BATTERY_ADC_SAMPLE_COUNT,
                valid_count,
                zero_count,
                error_count,
                min_raw,
                max_raw,
                avg_raw
            );
            note_rust_adc_diagnostic_sample(&$model, avg_raw, $source);
        } else {
            debug_println!(
                "battery-adc-batch: source={} unit=ADC1 gpio=GPIO{} attenuation=DB_11 samples={} valid=0 zero={} error={} marker=ADC_BATCH_NO_VALID",
                $source,
                board::BATTERY_ADC_GPIO,
                BATTERY_ADC_SAMPLE_COUNT,
                zero_count,
                error_count
            );
            note_rust_adc_diagnostic_sample(&$model, 0, $source);
        }
    }};
}

macro_rules! probe_battery_adc_existing_channel {
    ($adc:expr, $pin:expr, $gpio:expr, $enable:expr, $label:expr) => {{
        let mut valid_count: u16 = 0;
        let mut zero_count: u16 = 0;
        let mut error_count: u16 = 0;
        let mut min_raw: u16 = u16::MAX;
        let mut max_raw: u16 = 0;
        let mut sum_raw: u32 = 0;

        for sample_index in 0..BATTERY_ADC_PROBE_SAMPLE_COUNT {
            match $adc.read(&mut $pin) {
                Ok(raw) if raw > 0 => {
                    valid_count = valid_count.saturating_add(1);
                    min_raw = min_raw.min(raw);
                    max_raw = max_raw.max(raw);
                    sum_raw = sum_raw.saturating_add(raw as u32);
                }
                Ok(_) => {
                    zero_count = zero_count.saturating_add(1);
                }
                Err(err) => {
                    error_count = error_count.saturating_add(1);
                    debug_println!(
                        "battery-adc-probe-error: label={} gpio=GPIO{} enable={} sample={} marker=ADC_PROBE_READ_ERROR err={:?}",
                        $label,
                        $gpio,
                        $enable,
                        sample_index,
                        err
                    );
                }
            }

            if sample_index + 1 < BATTERY_ADC_PROBE_SAMPLE_COUNT {
                thread::sleep(Duration::from_millis(BATTERY_ADC_PROBE_DELAY_MS));
            }
        }

        if valid_count > 0 {
            let avg_raw = (sum_raw / valid_count as u32) as u16;
            debug_println!(
                "battery-adc-probe: label={} gpio=GPIO{} unit=ADC1 attenuation=DB_11 enable={} samples={} valid={} zero={} error={} min={} max={} avg={} ui_source=NO",
                $label,
                $gpio,
                $enable,
                BATTERY_ADC_PROBE_SAMPLE_COUNT,
                valid_count,
                zero_count,
                error_count,
                min_raw,
                max_raw,
                avg_raw
            );
        } else {
            debug_println!(
                "battery-adc-probe: label={} gpio=GPIO{} unit=ADC1 attenuation=DB_11 enable={} samples={} valid=0 zero={} error={} marker=ADC_PROBE_NO_VALID ui_source=NO",
                $label,
                $gpio,
                $enable,
                BATTERY_ADC_PROBE_SAMPLE_COUNT,
                zero_count,
                error_count
            );
        }
    }};
}

macro_rules! probe_battery_adc_pin {
    ($adc:expr, $pin:expr, $gpio:expr, $enable:expr, $label:expr) => {{
        let probe_config = AdcChannelConfig {
            attenuation: BAT_ATTEN,
            ..Default::default()
        };

        match AdcChannelDriver::new(&$adc, $pin, &probe_config) {
            Ok(mut probe_pin) => {
                probe_battery_adc_existing_channel!($adc, probe_pin, $gpio, $enable, $label);
            }
            Err(err) => {
                debug_println!(
                    "battery-adc-probe: label={} gpio=GPIO{} unit=ADC1 attenuation=DB_11 enable={} samples=0 valid=0 zero=0 error=1 marker=ADC_PROBE_INIT_ERROR err={:?} ui_source=NO",
                    $label,
                    $gpio,
                    $enable,
                    err
                );
            }
        }
    }};
}

macro_rules! probe_battery_adc_matrix {
    ($adc:expr, $bat_pin:expr, $pins:ident) => {{
        debug_println!(
            "battery-adc-probe-matrix: start candidates=GPIO1,GPIO3,GPIO7,GPIO8,GPIO9 enable=NONE ui_source=NO"
        );
        debug_println!(
            "battery-adc-probe-enable: candidate=NONE_CONFIRMED action=SKIP_WITH_ENABLE_TEST"
        );

        probe_battery_adc_pin!($adc, $pins.gpio1, 1_u8, "NONE", "probe-gpio1-adc1-ch0");
        probe_battery_adc_pin!($adc, $pins.gpio3, 3_u8, "NONE", "probe-gpio3-adc1-ch2");
        probe_battery_adc_pin!($adc, $pins.gpio7, 7_u8, "NONE", "probe-gpio7-adc1-ch6");
        probe_battery_adc_existing_channel!($adc, $bat_pin, board::BATTERY_ADC_GPIO, "NONE", "current-gpio8-adc1-ch7");
        probe_battery_adc_pin!($adc, $pins.gpio9, 9_u8, "NONE", "probe-gpio9-adc1-ch8");

        debug_println!("battery-adc-probe-matrix: end ui_source=NO");
    }};
}

fn load_battery_config_from_sd(model: &mut AppState) {
    let defaults = BatteryCalibration::defaults();

    match read_battery_txt_from_sources() {
        Ok((text, source)) => match parse_battery_calibration_text(&text, defaults) {
            Ok(cfg) => {
                model.set_battery_calibration(cfg.adc_multiplier, cfg.empty_mv, cfg.full_mv, "SD");
                debug_println!(
                    "battery-cal: applied source={} adc_multiplier={:.3} empty_mv={} full_mv={} status=SD",
                    source,
                    cfg.adc_multiplier,
                    cfg.empty_mv,
                    cfg.full_mv
                );
            }
            Err(reason) => {
                model.set_battery_calibration(
                    defaults.adc_multiplier,
                    defaults.empty_mv,
                    defaults.full_mv,
                    "DEFAULT",
                );
                debug_println!(
                    "battery-cal: invalid reason={} using defaults adc_multiplier={:.3} empty_mv={} full_mv={} status=DEFAULT",
                    reason,
                    defaults.adc_multiplier,
                    defaults.empty_mv,
                    defaults.full_mv
                );
            }
        },
        Err(reason) => {
            model.set_battery_calibration(
                defaults.adc_multiplier,
                defaults.empty_mv,
                defaults.full_mv,
                "DEFAULT",
            );
            debug_println!(
                "battery-cal: not loaded reason={} using defaults adc_multiplier={:.3} empty_mv={} full_mv={} status=DEFAULT",
                reason,
                defaults.adc_multiplier,
                defaults.empty_mv,
                defaults.full_mv
            );
        }
    }
}

fn read_battery_txt_from_sources() -> Result<(String, &'static str), &'static str> {
    let mut sd_buf = [0_u8; BATTERY_TXT_MAX_BYTES];
    let sd_read =
        unsafe { ffi::st77916_read_sd_battery_txt(sd_buf.as_mut_ptr(), sd_buf.len() as u32) };

    if sd_read > 0 {
        let len = (sd_read as usize).min(sd_buf.len());
        let text = core::str::from_utf8(&sd_buf[..len]).map_err(|_| "UTF8")?;
        debug_println!("battery-cal: read source=/sdcard/BATTERY.TXT bytes={}", len);
        return Ok((text.to_string(), "/sdcard/BATTERY.TXT"));
    }

    for path in BATTERY_TXT_FALLBACK_PATHS {
        match std::fs::read_to_string(path) {
            Ok(text) if !text.trim().is_empty() => {
                debug_println!("battery-cal: read source={} bytes={}", path, text.len());
                return Ok((text, *path));
            }
            _ => {}
        }
    }

    Err("NO BATTERY.TXT")
}

fn parse_battery_calibration_text(
    text: &str,
    defaults: BatteryCalibration,
) -> Result<BatteryCalibration, &'static str> {
    let mut cfg = defaults;
    let mut any = false;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();

        match key.as_str() {
            "adc_multiplier" | "multiplier" => {
                let parsed = value.parse::<f32>().map_err(|_| "BAD_MULTIPLIER")?;
                if !(1.0..=5.0).contains(&parsed) {
                    return Err("MULTIPLIER_RANGE");
                }
                cfg.adc_multiplier = parsed;
                any = true;
            }
            "empty_mv" => {
                cfg.empty_mv = value.parse::<u16>().map_err(|_| "BAD_EMPTY")?;
                any = true;
            }
            "full_mv" => {
                cfg.full_mv = value.parse::<u16>().map_err(|_| "BAD_FULL")?;
                any = true;
            }
            _ => {}
        }
    }

    if !any {
        return Err("NO_KEYS");
    }
    if cfg.full_mv <= cfg.empty_mv {
        return Err("BAD_THRESHOLDS");
    }
    if cfg.empty_mv < 2500 || cfg.full_mv > 4600 {
        return Err("THRESHOLD_RANGE");
    }

    Ok(cfg)
}

fn log_battery_enable_probe(
    phase: &str,
    enable_source: &str,
    enable_state: &str,
    action: &str,
    ok: bool,
    result: &ffi::St77916AdcProbeResult,
) {
    debug_println!(
        "battery-enable-probe: phase={} enable_source={} enable_state={} action={} unit=ADC1 channel=7 gpio=GPIO8 status={} ok={} valid={} zero={} error={} min={} max={} avg={} mv_avg={} calibrated={} ui_source=NO",
        phase,
        enable_source,
        enable_state,
        action,
        result.status,
        ok,
        result.valid_count,
        result.zero_count,
        result.error_count,
        result.raw_min,
        result.raw_max,
        result.raw_avg,
        result.mv_avg,
        result.calibrated
    );
}

fn log_battery_enable_probe_skipped(
    phase: &str,
    enable_source: &str,
    enable_state: &str,
    action: &str,
) {
    debug_println!(
        "battery-enable-probe: phase={} enable_source={} enable_state={} action={} unit=ADC1 channel=7 gpio=GPIO8 status=SKIPPED ok=false valid=0 zero=0 error=0 min=-1 max=-1 avg=-1 mv_avg=-1 calibrated=0 ui_source=NO",
        phase,
        enable_source,
        enable_state,
        action
    );
}

// RAW-R42-VIDEO-DEAD-SOURCE-CLEANUP
fn run_app() -> Result<()> {
    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;
    let modem = peripherals.modem;

    let mut model = AppState::new();
    let providers = LocalProviders::new();

    unsafe {
        ffi::st77916_configure_runtime_logs(LOG_PROFILE_COMPILE_DEBUG);
    }
    let sd_persistent_boot = unsafe { ffi::st77916_sd_persistent_mount_session() };

    let mut runtime_log_profile = RuntimeLogProfile::from_debug(LOG_PROFILE_COMPILE_DEBUG);
    let mut runtime_log_profile_source = "COMPILE_DEFAULT";
    if let Some(sd_profile) = load_runtime_log_profile_from_sd() {
        runtime_log_profile = sd_profile;
        runtime_log_profile_source = "/LOG.TXT";
    }
    set_runtime_log_profile(runtime_log_profile, runtime_log_profile_source);

    let mut c_first_probe = ffi::St77916AdcProbeResult::default();
    let c_first_ok = unsafe {
        ffi::st77916_adc1_gpio8_oneshot_probe(
            BATTERY_ADC_SAMPLE_COUNT as u32,
            &mut c_first_probe as *mut ffi::St77916AdcProbeResult,
        )
    };
    debug_println!(
        "battery-adc-cfirst: phase=before-rust-adc-driver unit=ADC1 channel=7 gpio=GPIO8 status={} ok={} valid={} zero={} error={} min={} max={} avg={} mv_avg={} calibrated={} ui_source=NO",
        c_first_probe.status,
        c_first_ok,
        c_first_probe.valid_count,
        c_first_probe.zero_count,
        c_first_probe.error_count,
        c_first_probe.raw_min,
        c_first_probe.raw_max,
        c_first_probe.raw_avg,
        c_first_probe.mv_avg,
        c_first_probe.calibrated
    );
    log_battery_enable_probe(
        "before-rust-adc-driver",
        "NONE",
        "BASELINE",
        "C_FIRST_ADC1_CH7_GPIO8",
        c_first_ok,
        &c_first_probe,
    );
    log_battery_enable_probe_skipped(
        "before-rust-adc-driver",
        "TCA9554",
        "NO_CONFIRMED_ENABLE",
        "SKIP_NO_VENDOR_BAT_ENABLE_IN_RUST_DIAGNOSTICS",
    );
    note_vendor_c_shim_battery_sample(&mut model, &c_first_probe, "before-rust-adc-driver");

    let adc = AdcDriver::new(peripherals.adc1)?;
    let bat_config = AdcChannelConfig {
        attenuation: BAT_ATTEN,
        ..Default::default()
    };
    let mut bat_pin = AdcChannelDriver::new(&adc, pins.gpio8, &bat_config)?;
    debug_println!(
        "battery-adc-path: reference=rust-full-port unit=ADC1 gpio=GPIO{} attenuation=DB_11 samples={} delay_ms={}",
        board::BATTERY_ADC_GPIO,
        BATTERY_ADC_SAMPLE_COUNT,
        BATTERY_ADC_SAMPLE_DELAY_MS
    );
    debug_println!("battery-adc-path: production=c-shim-gpio8-vendor unit=ADC1 channel=7 gpio=GPIO8 attenuation=ADC_ATTEN_DB_12 measurement_offset=0.994500");
    debug_println!(
        "battery-adc-path: reference=rust-diagnostics init_order=BAT_Init gpio=GPIO{} unit=ADC1 attenuation=DB_11 diagnostic_only=YES",
        board::BATTERY_ADC_GPIO
    );
    debug_println!("battery-init-phase: pre-i2c order=after-peripherals-before-i2c");
    sample_battery_adc_batch!(adc, bat_pin, model, "pre-i2c");
    debug_println!(
        "battery-adc-ownership-compare: c_first_ok={} c_first_valid={} c_first_avg={} c_first_mv_avg={} rust_after_source=pre-i2c rust_after_state_raw={} rust_after_voltage={} ui_source=NO",
        c_first_ok,
        c_first_probe.valid_count,
        c_first_probe.raw_avg,
        c_first_probe.mv_avg,
        model.battery_adc_raw_label(),
        model.battery_voltage_text()
    );

    let power_gpio_ok = unsafe { ffi::st77916_gpio_input_pullup(board::POWER_BUTTON_GPIO as i32) };
    let power_gpio_initial_level = power_gpio_level();
    debug_println!(
        "power-gpio-config: gpio=GPIO{} via=c-shim gpio_config input=ENABLE pullup=ENABLE ok={} initial_level={} active_low_down={}",
        board::POWER_BUTTON_GPIO,
        power_gpio_ok,
        power_gpio_initial_level,
        power_gpio_down_from_level(power_gpio_initial_level)
    );

    debug_println!(
        "button-discovery-matrix: start candidates={} mode=REPORT_ONLY sleep_binding=DISABLED",
        BUTTON_PIN_CANDIDATES.len()
    );
    for candidate in BUTTON_PIN_CANDIDATES {
        let ok = unsafe { ffi::st77916_gpio_input_pullup(candidate.gpio) };
        let level = button_candidate_level(candidate);
        let noisy_at_boot = button_candidate_noisy_at_boot(candidate, level);
        debug_println!(
            "button-pin-config: label={} gpio=GPIO{} origin={} input_pullup_ok={} baseline_level={} active_low_down={} active_high_down={} noisy_at_boot={} sleep_bind_allowed={} guard={}",
            candidate.label,
            candidate.gpio,
            candidate.origin,
            ok,
            level,
            level == 0,
            level == 1,
            noisy_at_boot,
            candidate.sleep_bind_allowed,
            candidate.guard
        );
        if noisy_at_boot {
            debug_println!(
                "button-pin-noise: label={} gpio=GPIO{} baseline_level={} classification=STUCK_LOW_OR_BOOT_ACTIVE action=HOLD_SPAM_SUPPRESSED",
                candidate.label,
                candidate.gpio,
                level
            );
        }
    }

    let mut backlight = PinDriver::output(pins.gpio5)?;
    backlight.set_low()?;

    let mut touch_int = PinDriver::input(pins.gpio4)?;
    touch_int.set_pull(Pull::Up)?;

    let i2c_cfg = I2cConfig::new().baudrate(400.kHz().into());
    let mut i2c = I2cDriver::new(peripherals.i2c0, pins.gpio11, pins.gpio10, &i2c_cfg)?;

    debug_println!("battery-init-phase: pre-wifi order=after-i2c-before-wifi");
    sample_battery_adc_batch!(adc, bat_pin, model, "pre-wifi");

    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let mut wifi = EspWifi::new(modem, sys_loop, Some(nvs))?;
    let saved_wifi_config = saved_wifi_client_config_present(&wifi);
    if !saved_wifi_config {
        let _ = wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()));
    }
    let _ = wifi.start();
    if saved_wifi_config {
        debug_println!("wifi-boot: saved client config found; connecting");
        let _ = wifi.connect();
    } else {
        debug_println!("wifi-boot: no saved client config; use /WIFI.TXT import");
    }

    unsafe {
        ffi::st77916_time_configure_eastern();
    }
    debug_println!("time-zone: configured America/New_York");

    debug_println!("battery-init-phase: pre-exio order=after-wifi-before-exio");
    log_battery_enable_probe_skipped(
        "pre-exio-before-tca9554-init",
        "TCA9554",
        "NO_CONFIRMED_ENABLE",
        "SKIP_BEFORE_EXIO_INIT",
    );
    sample_battery_adc_batch!(adc, bat_pin, model, "pre-exio");

    let mut exio = Tca9554::new();
    let touch = Cst816::new();
    let rtc = Pcf85063::new();

    let exio_ok = exio.ping(&mut i2c, board::TCA9554_ADDR).is_ok();
    let touch_ok = touch.ping(&mut i2c, board::CST816_ADDR).is_ok();
    let rtc_ok = rtc.ping(&mut i2c, board::PCF85063_ADDR).is_ok();

    model.set_probe_status(exio_ok && touch_ok && rtc_ok, touch_ok, rtc_ok);
    model.backlight_percent = 100;
    if saved_wifi_config {
        model
            .settings
            .set_wifi_stage(WifiProvisioningStep::Connecting, "NVS");
    }
    debug_println!("\n=== {} ===", board::BOARD_NAME);
    debug_println!("Hybrid Rust + ESP-IDF display backend");
    debug_println!("v0.1.29-r3 Video Worker Status Quieting + Version Marker Repair");
    debug_println!("v0.1.29-r2 Video Worker SD Ownership Repair");
    debug_println!("v0.1.29-r1 Video Page Compile Repair");
    debug_println!("v0.1.29 Dedicated Video Player Screen + Worker Task Foundation");
    debug_println!("v0.1.28-r3 MJPEG Playback Responsiveness Repair");
    debug_println!("v0.1.28-r2 MJPEG Playback Frame Advance Visibility Repair");
    debug_println!("v0.1.28-r1 Playback Frame Index Compile Repair");
    debug_println!("v0.1.28 MJPEG Playback Loop, No Audio");
    debug_println!("v0.1.27-r2 MJPEG Preview Visibility Repair");
    debug_println!("v0.1.27-r1 MJPEG JPEG Format Enum Compile Repair");
    debug_println!("v0.1.27 MJPEG First Frame Decode + Display");
    debug_println!("v0.1.26 MJPEG Video Foundation");
    debug_println!("v0.1.25-r1 SD Asset Cache Scope Compile Repair");
    debug_println!("v0.1.25 SD-Backed UI Asset Loader + App Partition Relief");
    debug_println!("v0.1.24-r7 Wi-Fi Scan Quieting + Touch Finish Normalization");
    debug_println!("v0.1.24-r6-r1 Log Macro Scope Compile Repair");
    debug_println!("v0.1.24-r6 Monitor Log Cleanup + Runtime Verbosity Profiles");
    debug_println!("v0.1.24-r5-r1 Software Sleep Compile Repair");
    debug_println!("v0.1.24-r5 Software Sleep Control");
    debug_println!("v0.1.24-r4 EXIO Safe Input-Only Discovery for Unused Bits");
    debug_println!("v0.1.24-r3 Button Discovery Noise Gate + EXIO Input Matrix");
    debug_println!("v0.1.24-r2 Button Pin Discovery Matrix");
    debug_println!("v0.1.24-r1 Power Button GPIO Input Diagnostics + Sleep Trigger Repair");
    debug_println!("v0.1.24 Screen Sleep / Wake Guard");
    debug_println!("v0.1.23-r12 Vendor ESP-IDF Battery ADC Parity Alignment");
    debug_println!("v0.1.23-r11 V1 Schematic Battery ADC Confirmation + Physical Probe Guide");
    debug_println!("v0.1.23-r10 Battery Enable Path Probe");
    debug_println!("v0.1.23-r9-r1 C-Shim First ADC Ownership Probe");
    debug_println!("v0.1.23-r9 ESP-IDF ADC OneShot C-Shim Parity Probe");
    debug_println!("v0.1.23-r8 Battery Init Order Diagnostics Repo Alignment");
    debug_println!("v0.1.23-r7 Battery ADC Pin/Enable Probe Matrix");
    debug_println!("v0.1.23-r6 Vendor Battery ADC Path Alignment");
    debug_println!("v0.1.23-r5-r2 Battery First-Sample Before Any SD Access Repair");
    debug_println!("v0.1.23-r5-r1 Battery Isolation Compile Repair");
    debug_println!("v0.1.23-r5 Battery Calibration SD Read Isolation Repair");
    debug_println!("v0.1.23-r4 Battery Calibration Config");
    debug_println!("v0.1.23-r3 Battery ADC Diagnostics + Calibration");
    debug_println!("v0.1.23-r2 Home Battery + Settings Detail Repair");
    debug_println!("v0.1.23-r1 Battery Badge Placement Repair");
    debug_println!("v0.1.23 Battery Status Across Screens");
    debug_println!("v0.1.22-r5 Settings Detail Clean Base Repair");
    debug_println!("v0.1.22-r4 Settings Missing Icon Helper Compile Repair");
    debug_println!("v0.1.22-r3 Settings SD Space Compile Repair");
    debug_println!("v0.1.22-r2 Settings Detail Visual Alignment Repair");
    debug_println!("v0.1.22-r1 Settings Icon Helper Compile Repair");
    debug_println!("v0.1.22 Settings Details Hub + Home Simplification");
    debug_println!("Preserve v0.1.21-r2 Weather Center Location Auto-Fetch Repair");
    debug_println!("Screens frozen: Home r3 | Weather r8-r2 | Music v0.1.11 | Assistant v0.1.12 | Settings Option A");
    debug_println!(
        "Input: r12 gesture-first touch, poll={}ms, window={}ms, cooldown={}ms",
        TOUCH_POLL_MS,
        TOUCH_ACTIVE_POLL_WINDOW_MS,
        TOUCH_NAV_COOLDOWN_MS
    );
    debug_println!("Renderer: hybrid RGB565 five page assets + dynamic overlays");
    debug_println!("Integrations: Wi-Fi real station connect; SNTP time sync; weather provider foundation; periodic SD/GPIO refresh disabled");
    debug_println!("UI baseline: frozen five-screen layout with regression guards");
    debug_println!("Asset guard: stale non-frozen RGB565 files are cleaned before validation");
    debug_println!("Repository cleanup: legacy patch docs/scripts and LVGL lab leftovers removed");
    debug_println!("Architecture: AppState, UiIntent, and LocalProviders boundaries active");
    debug_println!("Settings subscreens: Wi-Fi Display Sound About local controls active");
    debug_println!("macOS build: scripts force cargo +esp for xtensa-esp32s3-espidf");
    debug_println!("Settings controls: tap row to open, tap detail to change local value");
    debug_println!("Settings detail navigation: swipe up previous, swipe down next");
    debug_println!("Settings detail order: WI-FI -> DISPLAY -> SOUND -> ABOUT -> WI-FI");
    debug_println!("Validator repair: shell guard checks page order in pages.rs");
    debug_println!("Compile repair: Settings detail intents covered in AppState note_intent");
    debug_println!("Settings detail back: tap title/header band to return to Settings main");
    debug_println!("Validator repair: intent boundary guard aligned with r4 header-back marker");
    debug_println!("Validator repair: provider boundary guard aligned with r4 header-back marker");
    debug_println!(
        "Wi-Fi status foundation: station scan, AP count, connected flag, SSID snapshot"
    );
    debug_println!("Wi-Fi credential import: read /WIFI.TXT from SD/storage paths");
    debug_println!("Wi-Fi connect foundation: save NVS config and attempt station connect");
    debug_println!("No captive portal, no weather, no NTP in v0.1.18");
    debug_println!("Compile repair: POWER button intents carry Wi-Fi connect context");
    debug_println!("Time sync foundation: RTC boot read, SNTP after Wi-Fi, PCF85063 writeback");
    debug_println!("Time zone: America/New_York via POSIX TZ EST5EDT rule");
    debug_println!("Weather provider foundation: Open-Meteo current weather for Jersey City");
    debug_println!("Weather display: city/place label replaces LOCAL placeholder");
    debug_println!("Weather policy: bounded fetch with last-known-good stale fallback");
    debug_println!("Weather config repair: center tap cycles location and auto-fetches");
    debug_println!("Home simplification: center tap refreshes weather without detail mode");
    debug_println!("Settings hub: Network Weather Time Display Sound Storage Device Diagnostics");
    debug_println!("Settings detail visual repair: shared template, aligned rows, clipped values");
    debug_println!(
        "Settings detail clean base: old baked row/card area cleared before detail draw"
    );
    debug_println!("Battery status: non-Home screens show compact battery badge");
    debug_println!("Battery badge repair: Home-only battery badge with percent text");
    debug_println!(
        "Battery settings: Device detail shows percent, USB/charging state, and voltage"
    );
    debug_println!(
        "Battery diagnostics: raw ADC, ADC mV, calculated battery mV, estimated percent"
    );
    debug_println!("Battery calibration: /BATTERY.TXT adc_multiplier empty_mv full_mv");
    debug_println!("Battery calibration: Settings Device shows CAL status");
    debug_println!(
        "Battery isolation: sample before SD calibration, then sample after calibration"
    );
    debug_println!("Battery isolation: raw=0 ignored once, last valid ADC sample retained");
    debug_println!("Compile repair: battery raw/mV label helpers restored");
    debug_println!(
        "Battery first-sample: ADC sampled before weather-cache and BATTERY.TXT SD reads"
    );
    debug_println!("Battery first-sample: raw=0 never stored as valid battery sample");
    debug_println!("Battery ADC path: vendor-aligned ADC1 GPIO8 DB_11 multi-sample probe");
    debug_println!("Battery ADC batch: min/max/avg with ADC read error markers");
    debug_println!("Battery probe matrix: GPIO1/GPIO3/GPIO7/GPIO8/GPIO9 ADC1 candidates");
    debug_println!("Battery probe matrix: probe values are diagnostics only, not UI source");
    debug_println!(
        "Battery probe enable: no confirmed vendor enable GPIO, matrix uses enable=NONE"
    );
    debug_println!("Battery init order: GPIO8 ADC sampled immediately after Peripherals::take");
    debug_println!("Battery init phases: pre-i2c pre-wifi pre-exio post-exio post-display");
    debug_println!("Battery diagnostics repo alignment: BAT_Init before I2C/Wi-Fi/EXIO/SD/display");
    debug_println!("Battery C-shim parity: native ESP-IDF adc_oneshot ADC1_CH7 GPIO8 probe");
    debug_println!(
        "Battery C-shim parity: Rust and C raw/mV logged side-by-side, UI source unchanged"
    );
    debug_println!(
        "Battery C-first ownership: native adc_oneshot probe runs before Rust AdcDriver"
    );
    debug_println!("Battery C-first ownership: C-first and Rust-after results are compared, UI source unchanged");
    debug_println!(
        "Battery enable path: rust-diagnostics inspected, no confirmed BAT enable pin found"
    );
    debug_println!(
        "Battery enable probe: C-first NONE baseline plus EXIO before/after init markers"
    );
    debug_println!("Battery enable probe: enable values are diagnostics only, UI source unchanged");
    debug_println!("Board profile: WAVESHARE_ESP32_S3_TOUCH_LCD_1_85C_V1 schematic locked");
    debug_println!("Battery schematic: BAT_ADC(GPIO8) confirmed on V1 schematic");
    debug_println!("Battery divider: schematic R ladder implies multiplier about 3.0");
    debug_println!("Battery UI: Device shows BAT ADC GPIO8 CONFIRMED / SIGNAL MISSING");
    debug_println!("Battery C-shim parity: post-boot polling disabled to avoid ADC1-in-use spam");
    debug_println!("Battery vendor ADC: GPIO8 ADC1_CH7 uses ESP-IDF ADC_ATTEN_DB_12");
    debug_println!("Battery vendor ADC source: C-SHIM GPIO8 VENDOR");
    debug_println!("Battery vendor ADC: Measurement_offset=0.994500 default multiplier active");
    debug_println!("Battery vendor ADC: valid C-shim calibrated mV promoted as UI source");
    debug_println!("Battery Rust ADC: retained as diagnostic-only comparison");
    debug_println!("Screen sleep: software Sleep Now, top-left long touch, or idle timeout turns backlight off");
    debug_println!("Screen wake: touch interrupt turns backlight on");
    debug_println!("Screen wake guard: first wake touch is swallowed, navigation blocked");
    debug_println!("Screen sleep policy: background RTC/Wi-Fi/weather/battery services continue");
    debug_println!(
        "Power GPIO diagnostics: GPIO6 configured with ESP-IDF gpio_config input pull-up"
    );
    debug_println!("Power GPIO diagnostics: raw level and active-low duration logged");
    debug_println!("Power GPIO diagnostics: report-only, not a sleep trigger");
    debug_println!("Software sleep: Settings > Display > Sleep Now active");
    debug_println!("Software sleep: optional top-left long-touch gesture active");
    debug_println!("Compile repair: Settings Display Sleep Now match arm comma restored");
    debug_println!("Monitor log cleanup: NORMAL default, DEBUG via compile constant or /LOG.TXT");
    debug_println!("Button discovery matrix: GPIO0 BOOT, GPIO6 POWER, GPIO7, GPIO9 monitored");
    debug_println!("Button discovery matrix: report-only, no candidate bound to sleep yet");
    debug_println!(
        "Button discovery matrix: GPIO0 BOOT never used for sleep while USB flashing is attached"
    );
    debug_println!("Button discovery noise gate: boot level is idle baseline for every candidate");
    debug_println!(
        "Button discovery noise gate: boot-active candidates marked noisy and not hold-spammed"
    );
    debug_println!(
        "EXIO input matrix: TCA9554 input register is polled read-only, no output toggles"
    );
    debug_println!("EXIO safe discovery: preserve EXIO0..2 outputs and configure EXIO3..7 inputs");
    debug_println!("EXIO safe discovery: input mask 0xF8, protected output mask 0x07");
    debug_println!(
        "EXIO safe discovery: report-only, no sleep binding until physical transition confirmed"
    );
    debug_println!("Battery power: USB/UNKNOWN until charger GPIO is validated");
    debug_println!("Battery refresh: visible page redraws on battery change");
    debug_println!("Storage detail: SD free/total space shown when available");
    debug_println!("Weather units repair: footer tap toggles F/C and auto-fetches");
    debug_println!("Weather hourly repair: current time is floored to matching hourly slot");
    debug_println!("Weather cache: SD last-known-good WEATHER.TXT restored across reboot");
    debug_println!("Weather diagnostics: HTTP byte count and response sample on parse failure");
    debug_println!(
        "Weather repair: parse Open-Meteo current/current_weather object, not units block"
    );
    debug_println!("Compile repair: single st77916_datetime_t typedef in time shim header");
    debug_println!("Compile repair: time sync loop state declared before run_app event loop");
    debug_println!("Rustmix Wave alignment: Wi-Fi + SNTP + RTC + weather provider path tracked");
    debug_println!("Build cleanup: retained fallback helpers use crate-level dead_code allowance");
    debug_println!("button-loop: poll={}ms", BUTTON_POLL_MS);
    debug_println!(
        "power-gpio: configured gpio=GPIO{} active_low=YES diag_ms={} long_press_ms={}",
        board::POWER_BUTTON_GPIO,
        POWER_GPIO_DIAG_MS,
        POWER_LONG_PRESS_MS
    );
    debug_println!("button-discovery: GPIO0 BOOT included but sleep_bind_allowed=false guard=USB_FLASHING_GUARD");
    debug_println!(
        "button-discovery: no candidate is bound to sleep until physical press is confirmed"
    );
    debug_println!(
        "button-discovery: boot-level baseline active, stuck-low GPIO7/GPIO9 noise-gated"
    );
    debug_println!(
        "exio-input-matrix: poll_ms={} mode=READ_ONLY no_output_toggle=YES",
        EXIO_INPUT_POLL_MS
    );
    debug_println!("exio-safe-input: config=0x{:02X} input_mask=0x{:02X} protected_output_mask=0x{:02X} bits=EXIO3..7 mode=REPORT_ONLY", board::EXIO_DISCOVERY_CONFIG, board::EXIO_DISCOVERY_INPUT_MASK, board::EXIO_SAFE_OUTPUT_MASK);
    debug_println!("software-sleep-control: settings_display_sleep_now=ENABLED corner_hold_ms={} corner=top-left x<= {} y<= {} wake=TOUCH_INT", SOFTWARE_SLEEP_CORNER_HOLD_MS, SOFTWARE_SLEEP_CORNER_X_MAX, SOFTWARE_SLEEP_CORNER_Y_MAX);
    debug_println!("screen-sleep-guard: idle/software sleep and touch wake active; hardware POWER sleep is report-only");
    probe_battery_adc_matrix!(adc, bat_pin, pins);
    debug_println!("I2C probes:");
    debug_println!(
        "  0x20 TCA9554  => {:?}",
        exio.ping(&mut i2c, board::TCA9554_ADDR)
    );
    debug_println!(
        "  0x15 CST816   => {:?}",
        touch.ping(&mut i2c, board::CST816_ADDR)
    );
    debug_println!(
        "  0x51 PCF85063 => {:?}",
        rtc.ping(&mut i2c, board::PCF85063_ADDR)
    );

    let _ = exio.set_output_port(&mut i2c, board::TCA9554_ADDR, 0xFF);
    let _ = exio.write_pin(&mut i2c, board::TCA9554_ADDR, board::EXIO_TOUCH_RST, true);
    let _ = exio.write_pin(&mut i2c, board::TCA9554_ADDR, board::EXIO_LCD_RST, true);
    let _ = exio.write_pin(&mut i2c, board::TCA9554_ADDR, board::EXIO_SD_CS, true);
    let exio_safe_config_result =
        exio.set_config(&mut i2c, board::TCA9554_ADDR, board::EXIO_DISCOVERY_CONFIG);
    debug_println!(
        "exio-safe-input-config: addr=0x{:02X} config=0x{:02X} protected_output_mask=0x{:02X} input_mask=0x{:02X} protected_outputs=EXIO0_TOUCH_RST,EXIO1_LCD_RST,EXIO2_SD_CS discovery_inputs=EXIO3,EXIO4,EXIO5,EXIO6,EXIO7 result={:?} no_output_toggle_on_inputs=YES",
        board::TCA9554_ADDR,
        board::EXIO_DISCOVERY_CONFIG,
        board::EXIO_SAFE_OUTPUT_MASK,
        board::EXIO_DISCOVERY_INPUT_MASK,
        exio_safe_config_result
    );

    let exio_config_boot = exio.read_config(&mut i2c, board::TCA9554_ADDR).ok();
    let exio_input_boot_raw = exio.read_input_port(&mut i2c, board::TCA9554_ADDR).ok();
    let exio_input_boot = exio_input_boot_raw.map(|v| v & board::EXIO_DISCOVERY_INPUT_MASK);
    debug_println!(
        "exio-input-matrix: init addr=0x{:02X} config={} raw_baseline={} discovery_baseline={} discovery_bits={} input_mask=0x{:02X} protected_output_mask=0x{:02X} mode=READ_ONLY no_output_toggle=YES",
        board::TCA9554_ADDR,
        exio_config_boot
            .map(|v| format!("0x{:02X}", v))
            .unwrap_or_else(|| "ERR".to_string()),
        exio_input_boot_raw
            .map(|v| format!("0x{:02X}", v))
            .unwrap_or_else(|| "ERR".to_string()),
        exio_input_boot
            .map(|v| format!("0x{:02X}", v))
            .unwrap_or_else(|| "ERR".to_string()),
        exio_input_boot
            .map(|v| format!("{:08b}", v))
            .unwrap_or_else(|| "ERR".to_string()),
        board::EXIO_DISCOVERY_INPUT_MASK,
        board::EXIO_SAFE_OUTPUT_MASK
    );

    debug_println!("battery-init-phase: post-exio order=after-tca9554-config-before-display");
    log_battery_enable_probe_skipped(
        "post-exio-after-tca9554-config",
        "TCA9554",
        "NO_CONFIRMED_ENABLE",
        "NO_TOGGLE_EXIO_BITS_0_2_ASSIGNED_TOUCH_LCD_SD",
    );
    sample_battery_adc_batch!(adc, bat_pin, model, "post-exio");

    pulse_exio(
        &mut exio,
        &mut i2c,
        board::TCA9554_ADDR,
        board::EXIO_LCD_RST,
    );

    if !unsafe { ffi::st77916_panel_init() } {
        bail!("st77916_panel_init() failed");
    }

    debug_println!("panel init ok");

    backlight.set_high()?;
    debug_println!("backlight on");

    debug_println!(
        "battery-init-phase: post-display order=after-panel-backlight-before-touch-rtc-sd"
    );
    log_battery_enable_probe_skipped(
        "post-display",
        "TCA9554",
        "NO_CONFIRMED_ENABLE",
        "NO_TOGGLE_AFTER_DISPLAY_INIT",
    );
    sample_battery_adc_batch!(adc, bat_pin, model, "post-display");

    pulse_exio(
        &mut exio,
        &mut i2c,
        board::TCA9554_ADDR,
        board::EXIO_TOUCH_RST,
    );

    let touch_cfg = touch.read_config(&mut i2c, board::CST816_ADDR).ok();
    let _ = touch.disable_auto_sleep(&mut i2c, board::CST816_ADDR);

    if let Some(cfg) = touch_cfg {
        debug_println!(
            "Touch cfg: version=0x{:02X} chip_id=0x{:02X} project_id=0x{:02X} fw=0x{:02X}",
            cfg.version,
            cfg.chip_id,
            cfg.project_id,
            cfg.fw_version
        );
    }

    if let Ok(dt) = rtc.read_datetime(&mut i2c, board::PCF85063_ADDR) {
        model.update_rtc(dt);
        model.time.note_rtc_boot_read();
        debug_println!(
            "time-rtc: boot read {} {}",
            model.rtc_ymd(),
            model.rtc_hms_full()
        );
    } else {
        model.time.note_rtc_boot_failed();
        debug_println!("time-rtc: boot read failed");
    }
    debug_println!("battery-init-phase: boot-precal order=after-rtc-before-sd-cache");
    sample_battery_adc_batch!(adc, bat_pin, model, "boot-precal");
    load_weather_cache_from_sd(&mut model);

    let mut frame = FrameBuffer::new_rgb565(W * H)?;
    let mut asset_cache = UiAssetCache::new()?;

    debug_println!(
        "heap free: 8bit={} psram={}",
        unsafe { heap_caps_get_free_size(MALLOC_CAP_8BIT) },
        unsafe { heap_caps_get_free_size(MALLOC_CAP_SPIRAM) }
    );

    load_battery_config_from_sd(&mut model);
    sample_battery_adc_batch!(adc, bat_pin, model, "boot-cal");
    refresh_wifi(&mut model, &mut wifi);
    refresh_sd(&mut model);

    // RAW-R40-CURRENT-STARTUP-REPORT-AFTER-REFRESH
    let r40_battery_voltage_text = model.battery_voltage_text();
    let r40_battery_percent_text = model.battery_home_text();

    boot_report::log_current_startup(
        board::BOARD_NAME,
        runtime_log_profile.label(),
        runtime_log_profile_source,
        sd_persistent_boot,
        unsafe { ffi::st77916_sd_persistent_mount_count() },
        model.battery_adc_source_text(),
        r40_battery_voltage_text.as_str(),
        r40_battery_percent_text.as_str(),
        model.battery_cal_status_text(),
        model.settings.volume_percent,
    );
    println!(
        "weather: cache={} location={} provider=open-meteo",
        model.weather.status_label(),
        model.weather.location_label()
    );
    audio_foundation::log_audio_foundation_boot();
    internet_radio::log_radio_boot();
    audio_foundation::apply_volume_percent_silent(model.settings.volume_percent);

    let mut dirty = true;
    let mut last_render = Instant::now() - Duration::from_millis(RENDER_MIN_INTERVAL_MS);
    let mut last_audio_progress_sequence = audio_foundation::music_progress_sequence();
    let mut last_radio_progress_sequence = internet_radio::progress_sequence();
    render_if_dirty(
        &mut dirty,
        &model,
        frame.as_mut_slice(),
        &mut asset_cache,
        true,
        &mut last_render,
    )?;
    debug_println!("polished circular home page rendered");

    let mut touch_tracker = TouchTracker::default();
    let mut power_tracker = ButtonTracker::new(Duration::from_millis(POWER_LONG_PRESS_MS));
    let mut power_last_level = power_gpio_initial_level;
    let mut power_pressed_since: Option<Instant> =
        if power_gpio_down_from_level(power_gpio_initial_level) {
            Some(Instant::now())
        } else {
            None
        };
    let mut last_power_diag = Instant::now() - Duration::from_millis(POWER_GPIO_DIAG_MS);
    debug_println!(
        "power-gpio-diag: gpio=GPIO{} level={} active_low_down={} duration_ms=0 source=boot",
        board::POWER_BUTTON_GPIO,
        power_gpio_initial_level,
        power_gpio_down_from_level(power_gpio_initial_level)
    );
    let mut button_pin_states: [ButtonPinState; 4] = core::array::from_fn(|idx| {
        let candidate = BUTTON_PIN_CANDIDATES[idx];
        let level = button_candidate_level(candidate);
        let down = button_candidate_expected_down(candidate, level);
        let noisy_at_boot = button_candidate_noisy_at_boot(candidate, level);
        debug_println!(
                "button-pin-diag: label={} gpio=GPIO{} baseline_level={} level={} active_low_down={} active_high_down={} expected_down={} noisy_at_boot={} duration_ms=0 source=boot sleep_bind_allowed={} guard={}",
                candidate.label,
                candidate.gpio,
                level,
                level,
                level == 0,
                level == 1,
                down,
                noisy_at_boot,
                candidate.sleep_bind_allowed,
                candidate.guard
            );
        if noisy_at_boot {
            debug_println!(
                    "button-pin-noise: label={} gpio=GPIO{} baseline_level={} classification=STUCK_LOW_OR_BOOT_ACTIVE action=HOLD_SPAM_SUPPRESSED",
                    candidate.label,
                    candidate.gpio,
                    level
                );
        }
        ButtonPinState {
            boot_level: level,
            last_level: level,
            active_since: None,
            last_diag: Instant::now() - Duration::from_millis(POWER_GPIO_DIAG_MS),
            long_reported: false,
            noisy_at_boot,
        }
    });
    let mut exio_input_baseline = exio_input_boot;
    let mut exio_input_last = exio_input_boot;
    let mut last_exio_input_poll = Instant::now() - Duration::from_millis(EXIO_INPUT_POLL_MS);
    let mut last_touch_poll = Instant::now();
    let mut last_button_poll = Instant::now();
    let mut last_rtc = Instant::now();
    let mut last_time_sync = Instant::now();
    let mut time_sync_deadline: Option<Instant> = None;
    let mut last_battery = Instant::now();
    let mut last_wifi = Instant::now();
    let mut wifi_connect_deadline: Option<Instant> = if saved_wifi_config {
        Some(Instant::now() + Duration::from_millis(WIFI_CONNECT_TIMEOUT_MS))
    } else {
        None
    };
    let mut last_navigation = Instant::now() - Duration::from_millis(TOUCH_NAV_COOLDOWN_MS);
    let mut last_user_activity = Instant::now();
    let mut screen_sleeping = false;
    let mut wake_guard_until: Option<Instant> = None;
    let mut sleep_wait_touch_release = false;
    println!(
        "screen-sleep: policy idle_ms={} wake_guard_ms={} sleep_sources=SETTINGS_DISPLAY_SLEEP_NOW,TOP_LEFT_LONG_TOUCH,IDLE wake_sources=TOUCH_INT power=REPORT_ONLY",
        SCREEN_SLEEP_IDLE_MS,
        SCREEN_WAKE_GUARD_MS
    );

    loop {
        let now = Instant::now();

        if wake_guard_until.is_some_and(|deadline| now >= deadline) {
            wake_guard_until = None;
            println!("screen-wake-guard: released");
        }

        if !screen_sleeping
            && !touch_tracker.active
            && now.duration_since(last_user_activity) >= Duration::from_millis(SCREEN_SLEEP_IDLE_MS)
        {
            screen_sleeping = true;
            wake_guard_until = None;
            sleep_wait_touch_release = false;
            touch_tracker.reset_fields();
            dirty = false;
            model.last_action = "SCREEN SLEEP";
            backlight.set_low()?;
            println!(
                "screen-sleep: source=idle idle_ms={} backlight=OFF render=PAUSED services=ACTIVE",
                SCREEN_SLEEP_IDLE_MS
            );
        }

        if last_touch_poll.elapsed() >= Duration::from_millis(TOUCH_POLL_MS) {
            last_touch_poll = now;

            if screen_sleeping {
                if sleep_wait_touch_release {
                    if touch_int.is_high() {
                        sleep_wait_touch_release = false;
                        println!("screen-sleep: source=software touch released wake=ARMED");
                    }
                } else if touch_int.is_low() {
                    screen_sleeping = false;
                    wake_guard_until = Some(now + Duration::from_millis(SCREEN_WAKE_GUARD_MS));
                    last_user_activity = now;
                    touch_tracker.reset_fields();
                    model.last_action = "SCREEN WAKE";
                    backlight.set_high()?;
                    dirty = true;
                    println!(
                        "screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH",
                        SCREEN_WAKE_GUARD_MS
                    );
                }
            } else if wake_guard_until.is_some() {
                if touch_tracker.active {
                    touch_tracker.reset_fields();
                }
                if touch_int.is_low() {
                    last_user_activity = now;
                    println!("screen-wake-guard: touch ignored source=int action=NO_NAVIGATION");
                }
            } else if touch_tracker.active {
                match touch.read_touch(&mut i2c, board::CST816_ADDR) {
                    Ok(point) if point.fingers > 0 => {
                        last_user_activity = now;
                        model.note_touch(point.x, point.y, point.fingers, point.gesture);
                        touch_tracker.update_down(now, "poll", point.x, point.y, point.gesture);
                    }
                    Ok(_) => {
                        touch_tracker.note_no_touch();
                    }
                    Err(_) => {
                        touch_tracker.note_no_touch();
                    }
                }

                if touch_tracker.software_sleep_corner_ready(now) {
                    let hold_ms = touch_tracker.active_elapsed(now).as_millis();
                    let start_x = touch_tracker.start_x;
                    let start_y = touch_tracker.start_y;
                    let span_x = touch_tracker.span_x();
                    let span_y = touch_tracker.span_y();
                    screen_sleeping = true;
                    wake_guard_until = None;
                    sleep_wait_touch_release = true;
                    touch_tracker.reset_fields();
                    dirty = false;
                    last_user_activity = now;
                    model.last_action = "SCREEN SLEEP";
                    backlight.set_low()?;
                    println!(
                        "screen-sleep: source=top-left-long-touch hold_ms={} start=({}, {}) span=({}, {}) backlight=OFF render=PAUSED services=ACTIVE wake=TOUCH_RELEASE_THEN_TOUCH_INT",
                        hold_ms,
                        start_x,
                        start_y,
                        span_x,
                        span_y
                    );
                } else if let Some(reason) = touch_tracker.finish_reason(now) {
                    if let Some(summary) = touch_tracker.finish(now, reason) {
                        last_user_activity = now;
                        if touch_router::process_touch_summary(
                            &mut model,
                            &providers,
                            &mut wifi,
                            &mut wifi_connect_deadline,
                            summary,
                            now,
                            &mut last_navigation,
                        ) {
                            dirty = true;
                        }
                        if model.settings.take_software_sleep_request() {
                            screen_sleeping = true;
                            wake_guard_until = None;
                            sleep_wait_touch_release = true;
                            touch_tracker.reset_fields();
                            dirty = false;
                            last_user_activity = now;
                            model.last_action = "SCREEN SLEEP";
                            backlight.set_low()?;
                            println!(
                                "screen-sleep: source=settings-display-sleep-now backlight=OFF render=PAUSED services=ACTIVE wake=TOUCH_RELEASE_THEN_TOUCH_INT"
                            );
                        }
                    }
                }
            } else if touch_int.is_low() {
                if let Ok(point) = touch.read_touch(&mut i2c, board::CST816_ADDR) {
                    if point.fingers > 0 {
                        last_user_activity = now;
                        model.note_touch(point.x, point.y, point.fingers, point.gesture);
                        touch_tracker.update_down(now, "int", point.x, point.y, point.gesture);
                    }
                }
            }
        }

        if last_button_poll.elapsed() >= Duration::from_millis(BUTTON_POLL_MS) {
            last_button_poll = now;

            for (idx, candidate) in BUTTON_PIN_CANDIDATES.iter().copied().enumerate() {
                let level = button_candidate_level(candidate);
                let expected_down = button_candidate_expected_down(candidate, level);
                let state = &mut button_pin_states[idx];
                let transition_active = button_candidate_transition_active(*state, level);

                if level != state.last_level {
                    state.active_since = if transition_active { Some(now) } else { None };
                    state.last_level = level;
                    state.last_diag = now;
                    state.long_reported = false;
                    debug_println!(
                        "button-pin-diag: label={} gpio=GPIO{} baseline_level={} level={} active_low_down={} active_high_down={} expected_down={} transition_active={} noisy_at_boot={} duration_ms={} source={} sleep_bind_allowed={} guard={}",
                        candidate.label,
                        candidate.gpio,
                        state.boot_level,
                        level,
                        level == 0,
                        level == 1,
                        expected_down,
                        transition_active,
                        state.noisy_at_boot,
                        button_candidate_duration_ms(now, state.active_since),
                        if transition_active { "state-change-away-from-baseline" } else { "return-to-baseline" },
                        candidate.sleep_bind_allowed,
                        candidate.guard
                    );
                } else if transition_active
                    && !state.noisy_at_boot
                    && state.last_diag.elapsed() >= Duration::from_millis(POWER_GPIO_DIAG_MS)
                {
                    state.last_diag = now;
                    debug_println!(
                        "button-pin-diag: label={} gpio=GPIO{} baseline_level={} level={} active_low_down={} active_high_down={} expected_down={} transition_active=true noisy_at_boot=false duration_ms={} source=hold sleep_bind_allowed={} guard={}",
                        candidate.label,
                        candidate.gpio,
                        state.boot_level,
                        level,
                        level == 0,
                        level == 1,
                        expected_down,
                        button_candidate_duration_ms(now, state.active_since),
                        candidate.sleep_bind_allowed,
                        candidate.guard
                    );
                }

                let duration_ms = button_candidate_duration_ms(now, state.active_since);
                if transition_active
                    && !state.noisy_at_boot
                    && !state.long_reported
                    && duration_ms >= BUTTON_DISCOVERY_HOLD_REPORT_MS as u128
                {
                    state.long_reported = true;
                    debug_println!(
                        "button-pin-event: label={} gpio=GPIO{} event=LongCandidate baseline_level={} level={} active_low_down={} active_high_down={} duration_ms={} action=REPORT_ONLY sleep_bind_allowed={} guard={}",
                        candidate.label,
                        candidate.gpio,
                        state.boot_level,
                        level,
                        level == 0,
                        level == 1,
                        duration_ms,
                        candidate.sleep_bind_allowed,
                        candidate.guard
                    );
                }
            }

            if last_exio_input_poll.elapsed() >= Duration::from_millis(EXIO_INPUT_POLL_MS) {
                last_exio_input_poll = now;
                match exio.read_input_port(&mut i2c, board::TCA9554_ADDR) {
                    Ok(raw_value) => {
                        let value = raw_value & board::EXIO_DISCOVERY_INPUT_MASK;
                        if exio_input_baseline.is_none() {
                            exio_input_baseline = Some(value);
                            exio_input_last = Some(value);
                            debug_println!(
                                "exio-input-matrix: baseline-late raw_value=0x{:02X} discovery_value=0x{:02X} bits={:08b} input_mask=0x{:02X} mode=READ_ONLY no_output_toggle=YES",
                                raw_value,
                                value,
                                value,
                                board::EXIO_DISCOVERY_INPUT_MASK
                            );
                        } else if Some(value) != exio_input_last {
                            let previous = exio_input_last.unwrap_or(value);
                            let baseline = exio_input_baseline.unwrap_or(previous);
                            let changed_previous = previous ^ value;
                            let changed_baseline = baseline ^ value;
                            exio_input_last = Some(value);
                            debug_println!(
                                "exio-input-change: addr=0x{:02X} raw_value=0x{:02X} discovery_value=0x{:02X} discovery_bits={:08b} previous=0x{:02X} baseline=0x{:02X} changed_previous=0x{:02X} changed_baseline=0x{:02X} changed_bits={} input_mask=0x{:02X} protected_output_mask=0x{:02X} mode=READ_ONLY no_output_toggle=YES",
                                board::TCA9554_ADDR,
                                raw_value,
                                value,
                                value,
                                previous,
                                baseline,
                                changed_previous,
                                changed_baseline,
                                exio_changed_bits_text(changed_previous),
                                board::EXIO_DISCOVERY_INPUT_MASK,
                                board::EXIO_SAFE_OUTPUT_MASK
                            );
                        }
                    }
                    Err(_) => {
                        if exio_input_last.is_some() {
                            debug_println!(
                                "exio-input-change: addr=0x{:02X} status=READ_ERROR input_mask=0x{:02X} mode=READ_ONLY no_output_toggle=YES",
                                board::TCA9554_ADDR,
                                board::EXIO_DISCOVERY_INPUT_MASK
                            );
                            exio_input_last = None;
                        }
                    }
                }
            }

            let power_level = power_gpio_level();
            let power_down = power_gpio_down_from_level(power_level);

            if power_level != power_last_level {
                power_pressed_since = if power_down { Some(now) } else { None };
                power_last_level = power_level;
                last_power_diag = now;
                debug_println!(
                    "power-gpio-diag: gpio=GPIO{} level={} active_low_down={} duration_ms={} source=state-change",
                    board::POWER_BUTTON_GPIO,
                    power_level,
                    power_down,
                    power_gpio_duration_ms(now, power_pressed_since)
                );
            } else if power_down
                && last_power_diag.elapsed() >= Duration::from_millis(POWER_GPIO_DIAG_MS)
            {
                last_power_diag = now;
                debug_println!(
                    "power-gpio-diag: gpio=GPIO{} level={} active_low_down=true duration_ms={} source=hold",
                    board::POWER_BUTTON_GPIO,
                    power_level,
                    power_gpio_duration_ms(now, power_pressed_since)
                );
            }

            if let Some(event) = power_tracker.update(now, power_down) {
                let duration_ms = power_gpio_duration_ms(now, power_pressed_since);
                debug_println!(
                    "power-gpio-event: gpio=GPIO{} event={:?} level={} active_low_down={} duration_ms={} confirmed=YES action=REPORT_ONLY sleep_bind_allowed=false",
                    board::POWER_BUTTON_GPIO,
                    event,
                    power_level,
                    power_down,
                    duration_ms
                );
                last_user_activity = now;

                if screen_sleeping {
                    debug_println!(
                        "screen-wake: source=power-{:?} action=SKIP_UNBOUND_CANDIDATE guard_ms={} note=use_touch_wake_until_pin_confirmed",
                        event,
                        SCREEN_WAKE_GUARD_MS
                    );
                } else if matches!(event, ButtonEvent::Long) {
                    debug_println!(
                        "screen-sleep: source=power-long action=SKIP_UNBOUND_CANDIDATE gpio=GPIO{} duration_ms={} render=UNCHANGED services=ACTIVE",
                        board::POWER_BUTTON_GPIO,
                        duration_ms
                    );
                } else {
                    debug_println!(
                        "power-gpio-event: gpio=GPIO{} event={:?} action=NO_UI_BINDING_DISCOVERY_ONLY",
                        board::POWER_BUTTON_GPIO,
                        event
                    );
                }
            }
        }

        if !touch_tracker.active && last_rtc.elapsed() >= Duration::from_millis(RTC_REFRESH_MS) {
            last_rtc = now;
            if let Ok(dt) = rtc.read_datetime(&mut i2c, board::PCF85063_ADDR) {
                let previous_minute = model.rtc.map(|old| old.minute);
                model.update_rtc(dt);
                if model.current_page == AssistantPage::Home && previous_minute != Some(dt.minute) {
                    dirty = true;
                }
            }
        }

        if !touch_tracker.active
            && last_battery.elapsed() >= Duration::from_millis(BATTERY_REFRESH_MS)
        {
            last_battery = now;
            let previous_battery = model.battery_mv;
            sample_battery_adc_batch!(adc, bat_pin, model, "poll");
            if previous_battery != model.battery_mv {
                dirty = true;
            }
        }

        let wifi_poll_due = if wifi_connect_deadline.is_some() {
            last_wifi.elapsed() >= Duration::from_millis(WIFI_CONNECT_POLL_MS)
        } else {
            last_wifi.elapsed() >= Duration::from_millis(WIFI_REFRESH_MS)
        };

        if !touch_tracker.active && wifi_poll_due {
            last_wifi = now;
            let previous_wifi = (
                model.wifi_count,
                model.wifi_connected,
                model.wifi_ssid.clone(),
                model.settings.wifi_provisioning,
                model.settings.wifi_last_error,
            );
            refresh_wifi(&mut model, &mut wifi);
            update_wifi_connect_progress(&mut model, &mut wifi_connect_deadline, now);
            let wifi_visible = model.current_page == AssistantPage::Home
                || (model.current_page == AssistantPage::Settings
                    && model.settings.selected == SettingsPanel::Network);
            if wifi_visible
                && previous_wifi
                    != (
                        model.wifi_count,
                        model.wifi_connected,
                        model.wifi_ssid.clone(),
                        model.settings.wifi_provisioning,
                        model.settings.wifi_last_error,
                    )
            {
                dirty = true;
            }
        }

        if !touch_tracker.active
            && model.wifi_connected
            && model.time.phase != TimeSyncPhase::Persisted
            && model.time.phase != TimeSyncPhase::Failed
            && last_time_sync.elapsed() >= Duration::from_millis(TIME_SYNC_POLL_MS)
        {
            last_time_sync = now;
            let previous_time_phase = model.time.phase;
            if model.time.should_start_after_wifi() {
                unsafe {
                    ffi::st77916_sntp_start();
                }
                model.time.start_sntp_wait();
                time_sync_deadline = Some(now + Duration::from_millis(TIME_SYNC_TIMEOUT_MS));
                debug_println!("time-sync: sntp start servers=pool.ntp.org,time.google.com");
            }

            if unsafe { ffi::st77916_sntp_is_synced() } {
                model.time.note_sntp_synced();
                if let Some(dt) = local_datetime_from_system_time() {
                    match rtc.write_datetime(&mut i2c, board::PCF85063_ADDR, dt) {
                        Ok(()) => {
                            model.update_rtc(dt);
                            model.time.note_rtc_persisted();
                            time_sync_deadline = None;
                            println!(
                                "time-sync: sntp synced epoch={} local={} {}",
                                unsafe { ffi::st77916_time_epoch() },
                                model.rtc_ymd(),
                                model.rtc_hms_full()
                            );
                            println!("time-rtc: persisted source=NTP");
                        }
                        Err(_) => {
                            model.time.note_failed("RTC WRITE");
                            time_sync_deadline = None;
                            println!("time-rtc: persist failed reason=RTC WRITE");
                        }
                    }
                } else {
                    model.time.note_failed("LOCALTIME");
                    time_sync_deadline = None;
                    println!("time-sync: failed reason=LOCALTIME");
                }
            } else if time_sync_deadline.is_some_and(|deadline| now >= deadline) {
                model.time.note_failed("SNTP TIMEOUT");
                time_sync_deadline = None;
                println!("time-sync: failed reason=SNTP TIMEOUT");
            }

            if model.current_page == AssistantPage::Home
                || (model.current_page == AssistantPage::Settings && model.settings.detail_open)
                || previous_time_phase != model.time.phase
            {
                dirty = true;
            }
        }

        if !touch_tracker.active && !screen_sleeping && model.current_page == AssistantPage::Music {
            let audio_progress_sequence = audio_foundation::music_progress_sequence();
            if audio_progress_sequence != last_audio_progress_sequence {
                last_audio_progress_sequence = audio_progress_sequence;
                dirty = true;
            } else if audio_foundation::music_progress_active() {
                dirty = true;
            }
        }

        if !touch_tracker.active
            && !screen_sleeping
            && model.current_page == AssistantPage::InternetRadio
        {
            let radio_progress_sequence = internet_radio::progress_sequence();
            if radio_progress_sequence != last_radio_progress_sequence {
                last_radio_progress_sequence = radio_progress_sequence;
                dirty = true;
            } else if internet_radio::progress_active() {
                dirty = true;
            }
        }

        render_if_dirty(
            &mut dirty,
            &model,
            frame.as_mut_slice(),
            &mut asset_cache,
            !touch_tracker.active && !screen_sleeping,
            &mut last_render,
        )?;
        thread::sleep(Duration::from_millis(5));
    }
}

fn load_weather_cache_from_sd(model: &mut AppState) {
    let mut buf = [0_u8; WEATHER_CACHE_MAX_BYTES];
    let read_len = unsafe { ffi::st77916_read_sd_weather_txt(buf.as_mut_ptr(), buf.len() as u32) };

    if read_len <= 0 {
        debug_println!(
            "weather-cache: not loaded reason={}",
            screens::weather::weather_sd_error_label(read_len)
        );
        return;
    }

    let len = (read_len as usize).min(buf.len());
    let Ok(text) = core::str::from_utf8(&buf[..len]) else {
        debug_println!("weather-cache: not loaded reason=UTF8");
        return;
    };

    if model.weather.apply_cache_text(text) {
        debug_println!(
            "weather-cache: loaded bytes={} location={} status={}",
            len,
            model.weather.location_label(),
            model.weather.status_label()
        );
    } else {
        debug_println!("weather-cache: not loaded reason=EMPTY");
    }
}

fn persist_weather_cache(model: &AppState) {
    let text = model.weather.cache_text();
    let result = unsafe { ffi::st77916_write_sd_weather_txt(text.as_ptr(), text.len() as u32) };

    if result > 0 {
        println!(
            "weather-cache: persisted bytes={} location={} units={} status={}",
            result,
            model.weather.location_label(),
            model.weather.units.suffix(),
            model.weather.status_label()
        );
    } else {
        println!(
            "weather-cache: persist failed reason={}",
            screens::weather::weather_sd_error_label(result)
        );
    }
}

fn refresh_weather_provider(model: &mut AppState) {
    if !model.wifi_connected {
        model.weather.apply_failed("NO WIFI");
        println!(
            "weather-fetch: skipped location={} reason=NO WIFI",
            model.weather.location_label()
        );
        return;
    }

    let provider_url = model.weather.provider_url();
    let url = match CString::new(provider_url.as_str()) {
        Ok(url) => url,
        Err(_) => {
            model.weather.apply_failed("BAD URL");
            println!(
                "weather-fetch: failed location={} reason=BAD URL",
                model.weather.location_label()
            );
            return;
        }
    };

    let mut buf = [0_u8; WEATHER_HTTP_MAX_BYTES];
    println!(
        "weather-fetch: start location={} provider=open-meteo",
        model.weather.location_label()
    );

    let read_len =
        unsafe { ffi::st77916_http_get(url.as_ptr(), buf.as_mut_ptr(), buf.len() as u32) };

    if read_len <= 0 {
        let reason = screens::weather::weather_http_error_label(read_len);
        model.weather.apply_failed(reason);
        println!(
            "weather-fetch: failed location={} reason={}",
            model.weather.location_label(),
            reason
        );
        return;
    }

    let len = (read_len as usize).min(buf.len());
    println!(
        "weather-fetch: http bytes={} location={}",
        len,
        model.weather.location_label()
    );

    let text = match core::str::from_utf8(&buf[..len]) {
        Ok(text) => text,
        Err(_) => {
            model.weather.apply_failed("UTF8");
            println!(
                "weather-fetch: failed location={} reason=UTF8",
                model.weather.location_label()
            );
            return;
        }
    };

    match WeatherSample::parse_open_meteo(text, model.weather.units) {
        Ok(sample) => {
            model
                .weather
                .apply_live_weather(sample, unsafe { ffi::st77916_time_epoch() });
            debug_println!(
                "weather-fetch: live location={} temp={} condition={} status={}",
                model.weather.location_label(),
                model.weather.temperature_label(),
                model.weather.condition_label(),
                model.weather.status_label()
            );
            debug_println!(
                "weather-fetch: hourly location={} slots={}",
                model.weather.location_label(),
                model.weather.hourly_summary()
            );
            persist_weather_cache(model);
        }
        Err(reason) => {
            let sample = screens::weather::weather_body_sample(text, 240);
            debug_println!("weather-fetch: body sample={}", sample);
            model.weather.apply_failed(reason);
            println!(
                "weather-fetch: failed location={} reason={}",
                model.weather.location_label(),
                reason
            );
        }
    }
}

fn local_datetime_from_system_time() -> Option<DateTime> {
    let mut local = ffi::St77916DateTime::default();
    if !unsafe { ffi::st77916_get_local_datetime(&mut local) } {
        return None;
    }

    Some(DateTime {
        second: local.second,
        minute: local.minute,
        hour: local.hour,
        day: local.day,
        month: local.month,
        year: local.year,
    })
}

fn saved_wifi_client_config_present(wifi: &EspWifi<'static>) -> bool {
    match wifi.get_configuration() {
        Ok(Configuration::Client(client)) => !client.ssid.as_str().is_empty(),
        _ => false,
    }
}

fn start_wifi_credential_import_and_connect(
    model: &mut AppState,
    wifi: &mut EspWifi<'static>,
    wifi_connect_deadline: &mut Option<Instant>,
) {
    println!("wifi-import: requested /WIFI.TXT credential import");
    model
        .settings
        .set_wifi_stage(WifiProvisioningStep::ImportSd, "IMPORT");

    let credential_text = match read_wifi_txt_from_sources() {
        Ok(text) => text,
        Err(reason) => {
            println!("wifi-import: failed reason={}", reason);
            model
                .settings
                .set_wifi_stage(WifiProvisioningStep::Failed, reason);
            *wifi_connect_deadline = None;
            return;
        }
    };

    let credentials = match WifiCredentials::parse(&credential_text) {
        Ok(credentials) => credentials,
        Err(err) => {
            println!("wifi-import: parse failed reason={}", err.label());
            model
                .settings
                .set_wifi_stage(WifiProvisioningStep::Failed, err.label());
            *wifi_connect_deadline = None;
            return;
        }
    };

    if let Err(reason) = apply_wifi_credentials_and_connect(wifi, &credentials) {
        println!(
            "wifi-connect: failed to start ssid={} reason={}",
            clipped_log_label(&credentials.ssid, 16),
            reason
        );
        model
            .settings
            .set_wifi_stage(WifiProvisioningStep::Failed, reason);
        *wifi_connect_deadline = None;
        return;
    }

    println!(
        "wifi-connect: connecting ssid={} source=WIFI.TXT",
        clipped_log_label(&credentials.ssid, 16)
    );
    model
        .settings
        .set_wifi_stage(WifiProvisioningStep::Connecting, "WIFI.TXT");
    *wifi_connect_deadline = Some(Instant::now() + Duration::from_millis(WIFI_CONNECT_TIMEOUT_MS));
}

fn apply_wifi_credentials_and_connect(
    wifi: &mut EspWifi<'static>,
    credentials: &WifiCredentials,
) -> Result<(), &'static str> {
    let ssid = credentials
        .ssid
        .as_str()
        .try_into()
        .map_err(|_| "SSID LONG")?;
    let password = credentials
        .password
        .as_str()
        .try_into()
        .map_err(|_| "PASS LONG")?;

    let auth_method = if credentials.password.is_empty() {
        AuthMethod::None
    } else {
        AuthMethod::WPA2Personal
    };

    let client = ClientConfiguration {
        ssid,
        password,
        auth_method,
        ..Default::default()
    };

    wifi.set_configuration(&Configuration::Client(client))
        .map_err(|_| "SET CFG")?;
    wifi.start().map_err(|_| "START")?;
    wifi.connect().map_err(|_| "CONNECT")?;

    Ok(())
}

fn update_wifi_connect_progress(
    model: &mut AppState,
    wifi_connect_deadline: &mut Option<Instant>,
    now: Instant,
) {
    if wifi_connect_deadline.is_none() {
        return;
    }

    if model.wifi_connected {
        model
            .settings
            .set_wifi_stage(WifiProvisioningStep::Connected, "NVS");
        debug_println!("wifi-connect: connected ssid={}", model.wifi_ssid_label());
        *wifi_connect_deadline = None;
        return;
    }

    if wifi_connect_deadline.is_some_and(|deadline| now >= deadline) {
        model
            .settings
            .set_wifi_stage(WifiProvisioningStep::Failed, "TIMEOUT");
        debug_println!("wifi-connect: failed reason=TIMEOUT");
        *wifi_connect_deadline = None;
    }
}

fn read_wifi_txt_from_sources() -> Result<String, &'static str> {
    let mut sd_buf = [0_u8; WIFI_TXT_MAX_BYTES];
    let sd_read =
        unsafe { ffi::st77916_read_sd_wifi_txt(sd_buf.as_mut_ptr(), sd_buf.len() as u32) };

    if sd_read > 0 {
        let len = (sd_read as usize).min(sd_buf.len());
        let text = core::str::from_utf8(&sd_buf[..len]).map_err(|_| "UTF8")?;
        debug_println!("wifi-import: read source=/sdcard/WIFI.TXT bytes={}", len);
        return Ok(text.to_string());
    }

    for path in WIFI_TXT_FALLBACK_PATHS {
        match std::fs::read_to_string(path) {
            Ok(text) if !text.trim().is_empty() => {
                debug_println!("wifi-import: read source={} bytes={}", path, text.len());
                return Ok(text);
            }
            _ => {}
        }
    }

    Err("NO WIFI.TXT")
}

fn clipped_log_label(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

const WIFI_TXT_FALLBACK_PATHS: &[&str] = &[
    "/WIFI.TXT",
    "/wifi.txt",
    "/storage/WIFI.TXT",
    "/storage/wifi.txt",
    "/spiflash/WIFI.TXT",
    "/spiflash/wifi.txt",
    "/sdcard/WIFI.TXT",
    "/sdcard/wifi.txt",
];

const BATTERY_TXT_FALLBACK_PATHS: &[&str] = &[
    "/BATTERY.TXT",
    "/battery.txt",
    "/storage/BATTERY.TXT",
    "/storage/battery.txt",
    "/spiflash/BATTERY.TXT",
    "/spiflash/battery.txt",
    "/sdcard/BATTERY.TXT",
    "/sdcard/battery.txt",
];

fn refresh_wifi(model: &mut AppState, wifi: &mut EspWifi<'static>) {
    let previous = (
        model.wifi_count,
        model.wifi_connected,
        model.wifi_ssid.clone(),
    );

    let connected = wifi.is_connected().unwrap_or(false);
    let ssid = if connected {
        match wifi.get_configuration() {
            Ok(Configuration::Client(client)) if !client.ssid.as_str().is_empty() => {
                Some(client.ssid.as_str().to_string())
            }
            _ => None,
        }
    } else {
        None
    };

    let ap_count = if connected && log_debug_enabled() {
        match wifi.scan() {
            Ok(aps) => Some(aps.len() as u16),
            Err(err) => {
                debug_println!("wifi-scan: failed reason={:?}", err);
                previous.0
            }
        }
    } else {
        if !connected {
            debug_println!(
                "wifi-scan: skipped reason=station-not-connected profile={}",
                RuntimeLogProfile::from_debug(log_debug_enabled()).label()
            );
        } else {
            debug_println!("wifi-scan: skipped reason=normal-profile connected=YES");
        }
        previous.0
    };

    model.update_wifi_status(ap_count, connected, ssid);

    let current = (
        model.wifi_count,
        model.wifi_connected,
        model.wifi_ssid.clone(),
    );

    if previous != current {
        println!(
            "wifi-status: connected={} ssid={} aps={}",
            model.wifi_connected_label(),
            model.wifi_ssid_label(),
            model.wifi_ap_count_label()
        );
    }
}

fn refresh_sd(model: &mut AppState) {
    let mut present = false;
    let mut capacity_mb = 0u32;
    let mut free_mb = 0u32;

    let ok =
        unsafe { ffi::st77916_probe_sd_space_mb(&mut present, &mut capacity_mb, &mut free_mb) };

    model.sd_present = ok && present;
    model.sd_capacity_mb = if ok && present && capacity_mb > 0 {
        Some(capacity_mb)
    } else {
        None
    };
    model.sd_free_mb = if ok && present && free_mb > 0 {
        Some(free_mb)
    } else {
        None
    };
}

fn mjpeg_status_label(status: i32, ok: bool) -> &'static str {
    if !ok {
        return "PROBE-FAIL";
    }

    match status {
        0 => "READY",
        -2 => "SD-MOUNT",
        -3 => "NO-DIR",
        -4 => "NO-FILES",
        -5 => "OPEN-FAIL",
        -6 => "NO-FRAME",
        _ => "UNKNOWN",
    }
}

fn mjpeg_name_from_c_buf(buf: &[u8; 32]) -> String {
    let len = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    core::str::from_utf8(&buf[..len])
        .unwrap_or("--")
        .to_string()
}

fn render_if_dirty(
    dirty: &mut bool,
    model: &AppState,
    frame: &mut [u16],
    asset_cache: &mut UiAssetCache,
    allow_render: bool,
    last_render: &mut Instant,
) -> Result<()> {
    if !*dirty || !allow_render {
        return Ok(());
    }

    if last_render.elapsed() < Duration::from_millis(RENDER_MIN_INTERVAL_MS) {
        return Ok(());
    }

    screens::assistant::draw_assistant_page(model, frame, asset_cache)?;
    debug_println!("render: coalesced repaint ok");
    *last_render = Instant::now();
    *dirty = false;
    Ok(())
}

fn handle_button_event(
    model: &mut AppState,
    _providers: &LocalProviders,
    _wifi: &mut EspWifi<'static>,
    _wifi_connect_deadline: &mut Option<Instant>,
    button: ButtonName,
    event: ButtonEvent,
) {
    let kind = match event {
        ButtonEvent::Short => ButtonPressKind::Short,
        ButtonEvent::Long => ButtonPressKind::Long,
    };
    model.note_button(button, kind);

    match (button, event) {
        (ButtonName::Boot, ButtonEvent::Short) => {
            println!("button: BOOT short -> reserved");
            model.note_intent(UiIntent::BootReserved);
        }
        (ButtonName::Boot, ButtonEvent::Long) => {
            println!("button: BOOT long -> reserved");
            model.note_intent(UiIntent::BootReserved);
        }
        (ButtonName::Power, ButtonEvent::Short) => {
            println!("button: POWER short -> report-only no firmware sleep binding");
            model.note_intent(UiIntent::PowerMenu);
        }
        (ButtonName::Power, ButtonEvent::Long) => {
            println!("button: POWER long -> report-only no firmware sleep binding");
            model.note_intent(UiIntent::PowerMenu);
        }
    }
}

fn log_settings_detail_vertical_intent(
    intent: UiIntent,
    signed_span_dy: i16,
    summary: &TouchSummary,
) -> Option<UiIntent> {
    match intent {
        UiIntent::SettingsPreviousDetail => {
            println!(
                "touch-class: settings detail swipe-up accepted previous dy={} span_y={} gesture=0x{:02X}",
                signed_span_dy, summary.span_y, summary.gesture
            );
        }
        UiIntent::SettingsNextDetail => {
            println!(
                "touch-class: settings detail swipe-down accepted next dy={} span_y={} gesture=0x{:02X}",
                signed_span_dy, summary.span_y, summary.gesture
            );
        }
        _ => {}
    }

    Some(intent)
}

fn log_span_intent(intent: UiIntent) -> Option<UiIntent> {
    match intent {
        UiIntent::NextPage => println!("touch-class: span swipe-left accepted next"),
        UiIntent::PreviousPage => println!("touch-class: span swipe-right accepted previous"),
        _ => {}
    }

    Some(intent)
}

fn draw_calendar_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    stroke_rect(frame, cx - 7, cy - 7, 14, 14, color);
    draw_line(frame, cx - 7, cy - 3, cx + 7, cy - 3, color);
    draw_line(frame, cx - 4, cy - 10, cx - 4, cy - 6, color);
    draw_line(frame, cx + 4, cy - 10, cx + 4, cy - 6, color);
    fill_rect(frame, cx - 3, cy + 1, 2, 2, color);
    fill_rect(frame, cx + 2, cy + 1, 2, 2, color);
}

fn draw_internet_radio_tile(model: &AppState, frame: &mut [u16]) {
    // v0.1.36-r31-r2: InternetRadio delegates to dedicated screen module.
    internet_radio_screen::draw(frame, &model.rtc_hms());
}
fn draw_internet_radio_screen(model: &AppState, frame: &mut [u16]) {
    // v0.1.36-r31: explicit screen entry point for InternetRadio dispatch.
    draw_internet_radio_tile(model, frame);
}

fn mjpeg_decode_status_label(status: i32, ok: bool) -> &'static str {
    if !ok {
        return "DECODE-FAIL";
    }
    match status {
        0 => "DECODED",
        -2 => "SD-MOUNT",
        -4 => "SHORT-READ",
        -5 => "OPEN-FAIL",
        -7 => "BAD-PATH",
        -8 => "NO-MEM",
        -9 => "SEEK-FAIL",
        -10 => "BAD-JPEG",
        -11 => "JPEG-FAIL",
        -12 => "LOOP",
        -6 => "NO-FRAME",
        _ => "NOT-READY",
    }
}

fn draw_watch_outer(frame: &mut [u16], accent: u16) {
    fill_circle(frame, CX, CY, R_OUTER - 6, BG);
    stroke_circle(frame, CX, CY, R_OUTER, RING_DIM);
    draw_arc_segment(frame, CX, CY, 170, 1, 224, 316, accent);
}

fn draw_complication_battery(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    fill_circle(frame, cx, cy, 18, BG_DARK);
    stroke_circle(frame, cx, cy, 18, color);
    stroke_rect(frame, cx - 8, cy - 5, 15, 10, WHITE);
    fill_rect(frame, cx + 8, cy - 2, 2, 4, WHITE);
    fill_rect(frame, cx - 6, cy - 3, 9, 6, color);
}

fn draw_complication_wifi(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    fill_circle(frame, cx, cy, 18, BG_DARK);
    stroke_circle(frame, cx, cy, 18, color);
    draw_arc_segment(frame, cx, cy + 8, 13, 1, 220, 320, WHITE);
    draw_arc_segment(frame, cx, cy + 8, 8, 1, 230, 310, WHITE);
    fill_circle(frame, cx, cy + 8, 2, color);
}

fn draw_complication_sd(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    fill_circle(frame, cx, cy, 18, BG_DARK);
    stroke_circle(frame, cx, cy, 18, color);
    stroke_rect(frame, cx - 7, cy - 10, 14, 18, WHITE);
    fill_rect(frame, cx + 3, cy - 10, 4, 5, BG_DARK);
}

fn draw_sun_cloud_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    draw_sun_icon(frame, cx - 16, cy - 6, 16, color);
    draw_cloud_icon(frame, cx + 16, cy + 8, WHITE);
}

fn draw_sun_icon(frame: &mut [u16], cx: i32, cy: i32, r: i32, color: u16) {
    fill_circle(frame, cx, cy, r, color);
    for angle in (0..360).step_by(45) {
        let rad = (angle as f32).to_radians();
        let x0 = cx + (rad.cos() * (r + 7) as f32).round() as i32;
        let y0 = cy + (rad.sin() * (r + 7) as f32).round() as i32;
        let x1 = cx + (rad.cos() * (r + 14) as f32).round() as i32;
        let y1 = cy + (rad.sin() * (r + 14) as f32).round() as i32;
        draw_line(frame, x0, y0, x1, y1, color);
    }
}

fn draw_cloud_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    fill_circle(frame, cx - 22, cy + 4, 16, color);
    fill_circle(frame, cx, cy - 5, 21, color);
    fill_circle(frame, cx + 22, cy + 6, 15, color);
    fill_rounded_rect(frame, cx - 40, cy + 3, 80, 24, 12, color);
}

fn draw_cloud_icon_medium(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    fill_circle(frame, cx - 18, cy + 4, 13, color);
    fill_circle(frame, cx, cy - 4, 17, color);
    fill_circle(frame, cx + 18, cy + 5, 12, color);
    fill_rounded_rect(frame, cx - 33, cy + 3, 66, 20, 10, color);
}

fn draw_wind_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    draw_line(frame, cx - 54, cy - 16, cx + 54, cy - 16, color);
    draw_line(frame, cx - 36, cy, cx + 42, cy, color);
    draw_line(frame, cx - 50, cy + 16, cx + 30, cy + 16, color);
}

fn draw_wind_icon_compact(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    draw_line(frame, cx - 34, cy - 10, cx + 34, cy - 10, color);
    draw_line(frame, cx - 26, cy, cx + 28, cy, color);
    draw_line(frame, cx - 32, cy + 10, cx + 22, cy + 10, color);
}

fn draw_hour_chip(frame: &mut [u16], cx: i32, cy: i32, hour: &str, temp: &str) {
    fill_rounded_rect(frame, cx - 26, cy - 18, 52, 38, 12, 0x0354);
    stroke_round_chip(frame, cx - 26, cy - 18, 52, 38, 12, ACCENT_WEATHER_BLUE);
    draw_text_centered_at(frame, cx, cy - 5, hour, WHITE, 1);
    draw_text_centered_at(frame, cx, cy + 12, temp, WHITE, 1);
}

#[derive(Clone, Copy)]
enum WeatherMiniIcon {
    Sun,
    PartlyCloudy,
    Cloud,
    Rain,
    Storm,
    Wind,
}

#[derive(Clone)]
struct WeatherHourRender {
    hour: String,
    temp: String,
    icon: WeatherMiniIcon,
}

fn fallback_timeline_entries(condition: &str) -> Vec<WeatherHourRender> {
    let raw = match condition {
        "CLEAR" | "LOCAL" => [
            ("11A", "72", WeatherMiniIcon::PartlyCloudy),
            ("12P", "73", WeatherMiniIcon::Sun),
            ("1P", "72", WeatherMiniIcon::Cloud),
            ("2P", "70", WeatherMiniIcon::Wind),
        ],
        "SUNNY" => [
            ("11A", "75", WeatherMiniIcon::Sun),
            ("12P", "77", WeatherMiniIcon::Sun),
            ("1P", "79", WeatherMiniIcon::PartlyCloudy),
            ("2P", "76", WeatherMiniIcon::Wind),
        ],
        "CLOUDY" => [
            ("11A", "68", WeatherMiniIcon::Cloud),
            ("12P", "68", WeatherMiniIcon::Cloud),
            ("1P", "70", WeatherMiniIcon::PartlyCloudy),
            ("2P", "67", WeatherMiniIcon::Rain),
        ],
        "BREEZY" => [
            ("11A", "70", WeatherMiniIcon::Wind),
            ("12P", "71", WeatherMiniIcon::PartlyCloudy),
            ("1P", "69", WeatherMiniIcon::Wind),
            ("2P", "68", WeatherMiniIcon::Storm),
        ],
        _ => [
            ("11A", "72", WeatherMiniIcon::PartlyCloudy),
            ("12P", "73", WeatherMiniIcon::Sun),
            ("1P", "72", WeatherMiniIcon::Cloud),
            ("2P", "70", WeatherMiniIcon::Wind),
        ],
    };

    raw.iter()
        .map(|(hour, temp, icon)| WeatherHourRender {
            hour: (*hour).to_string(),
            temp: (*temp).to_string(),
            icon: *icon,
        })
        .collect()
}

fn mini_icon_from_weather_code(code: i32) -> WeatherMiniIcon {
    match condition_from_weather_code(code) {
        "CLEAR" | "SUNNY" => WeatherMiniIcon::Sun,
        "CLOUDY" => WeatherMiniIcon::Cloud,
        "RAIN" => WeatherMiniIcon::Rain,
        "SNOW" => WeatherMiniIcon::Cloud,
        "STORM" => WeatherMiniIcon::Storm,
        "BREEZY" => WeatherMiniIcon::Wind,
        _ => WeatherMiniIcon::PartlyCloudy,
    }
}

fn draw_tiny_temp_value(frame: &mut [u16], cx: i32, y: i32, text: &str, color: u16) {
    // Kept for future numeric experiments. Large-strip Option D uses scale-2
    // text because the cards are now tall enough for readable hourly temps.
    draw_text_centered_at(frame, cx, y, text, color, 2);
}

fn draw_timeline_sun_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    fill_circle(frame, cx, cy, 8, color);
    let rays = [
        (0, -13, 0, -11),
        (0, 11, 0, 13),
        (-13, 0, -11, 0),
        (11, 0, 13, 0),
        (-9, -9, -8, -8),
        (8, -8, 9, -9),
        (-9, 9, -8, 8),
        (8, 8, 9, 9),
    ];
    for (x0, y0, x1, y1) in rays {
        draw_line(frame, cx + x0, cy + y0, cx + x1, cy + y1, color);
    }
}

fn draw_timeline_cloud_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    fill_circle(frame, cx - 9, cy + 3, 7, color);
    fill_circle(frame, cx, cy - 4, 9, color);
    fill_circle(frame, cx + 9, cy + 3, 7, color);
    fill_rounded_rect(frame, cx - 16, cy + 3, 32, 10, 5, color);
}

fn draw_timeline_partly_cloudy_icon(frame: &mut [u16], cx: i32, cy: i32) {
    fill_circle(frame, cx - 8, cy - 6, 6, ACCENT_WEATHER);
    draw_line(frame, cx - 8, cy - 15, cx - 8, cy - 13, ACCENT_WEATHER);
    draw_line(frame, cx - 17, cy - 6, cx - 15, cy - 6, ACCENT_WEATHER);
    draw_line(frame, cx + 1, cy - 6, cx + 3, cy - 6, ACCENT_WEATHER);
    draw_timeline_cloud_icon(frame, cx + 5, cy + 2, WHITE);
}

fn draw_timeline_rain_icon(frame: &mut [u16], cx: i32, cy: i32) {
    draw_timeline_cloud_icon(frame, cx, cy - 5, WHITE);
    draw_line(frame, cx - 8, cy + 9, cx - 9, cy + 13, ACCENT_WEATHER_BLUE);
    draw_line(frame, cx, cy + 9, cx - 1, cy + 13, ACCENT_WEATHER_BLUE);
    draw_line(frame, cx + 8, cy + 9, cx + 7, cy + 13, ACCENT_WEATHER_BLUE);
}

fn draw_timeline_storm_icon(frame: &mut [u16], cx: i32, cy: i32) {
    draw_timeline_cloud_icon(frame, cx, cy - 5, WHITE);
    draw_line(frame, cx + 3, cy + 8, cx - 2, cy + 12, ACCENT_WEATHER);
    draw_line(frame, cx - 2, cy + 12, cx + 2, cy + 12, ACCENT_WEATHER);
    draw_line(frame, cx + 2, cy + 12, cx - 1, cy + 17, ACCENT_WEATHER);
}

fn draw_timeline_wind_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    draw_line(frame, cx - 15, cy - 8, cx + 15, cy - 8, color);
    draw_line(frame, cx - 12, cy, cx + 13, cy, color);
    draw_line(frame, cx - 14, cy + 8, cx + 10, cy + 8, color);
}

fn compact_condition(condition: &str) -> &'static str {
    if condition.contains("CLOUDY") {
        "CLOUDY"
    } else if condition.contains("SUNNY") {
        "SUNNY"
    } else if condition.contains("BREEZY") {
        "BREEZY"
    } else if condition.contains("CLEAR") {
        "CLEAR"
    } else if condition.contains("RAIN") {
        "RAIN"
    } else if condition.contains("STORM") {
        "STORM"
    } else if condition.contains("SNOW") {
        "CLOUDY"
    } else {
        "LOCAL"
    }
}

fn draw_album_tile(frame: &mut [u16], cx: i32, cy: i32) {
    fill_rounded_rect(frame, cx - 25, cy - 25, 50, 50, 7, 0x2108);
    fill_rect(frame, cx - 21, cy - 21, 42, 17, ACCENT_MUSIC);
    fill_rect(frame, cx - 21, cy - 4, 42, 22, ACCENT_MUSIC_BLUE);
    fill_rect(frame, cx - 21, cy + 18, 42, 3, ACCENT_WEATHER);
    draw_text_centered_at(frame, cx, cy + 3, "AI", WHITE, 2);
}

fn mock_music_progress(toggle_count: u32) -> u8 {
    match toggle_count % 4 {
        0 => 15,
        1 => 35,
        2 => 60,
        _ => 85,
    }
}

fn draw_media_button(frame: &mut [u16], cx: i32, cy: i32, playing: bool) {
    fill_circle(frame, cx, cy, 42, BG_DARK);
    stroke_circle(frame, cx, cy, 44, ACCENT_MUSIC);
    if playing {
        fill_rounded_rect(frame, cx - 13, cy - 21, 8, 42, 4, WHITE);
        fill_rounded_rect(frame, cx + 5, cy - 21, 8, 42, 4, WHITE);
    } else {
        fill_play_triangle(frame, cx + 2, cy, 44, WHITE);
    }
}

fn draw_skip_icon(frame: &mut [u16], cx: i32, cy: i32, next: bool, color: u16) {
    if next {
        fill_play_triangle(frame, cx + 5, cy, 18, color);
        fill_rect(frame, cx + 12, cy - 10, 2, 20, color);
    } else {
        fill_left_triangle(frame, cx - 5, cy, 18, color);
        fill_rect(frame, cx - 14, cy - 10, 2, 20, color);
    }
}

fn draw_track_label(frame: &mut [u16], y: i32, label: &str) {
    // Keep music title as one line to avoid source-pill overlap.
    draw_text_centered(frame, y, label, WHITE, 2);
}

fn draw_waveform(frame: &mut [u16], cx: i32, cy: i32, amp: i32, color: u16) {
    let xs = [-70, -54, -38, -22, -8, 8, 22, 38, 54, 70];
    let heights = [6, 16, 28, 14, amp, amp, 14, 28, 16, 6];
    for i in 0..xs.len() - 1 {
        let x0 = cx + xs[i];
        let y0 = cy
            + if i % 2 == 0 {
                -heights[i] / 2
            } else {
                heights[i] / 2
            };
        let x1 = cx + xs[i + 1];
        let y1 = cy
            + if (i + 1) % 2 == 0 {
                -heights[i + 1] / 2
            } else {
                heights[i + 1] / 2
            };
        draw_line(frame, x0, y0, x1, y1, color);
    }
    fill_circle(frame, cx, cy, 4, WHITE);
}

fn draw_microphone_button(frame: &mut [u16], cx: i32, cy: i32, listening: bool) {
    let color = if listening {
        ACCENT_ASSISTANT
    } else {
        ACCENT_ASSISTANT_BLUE
    };
    fill_circle(frame, cx, cy, 24, color);
    fill_rounded_rect(frame, cx - 6, cy - 13, 12, 20, 6, WHITE);
    draw_line(frame, cx, cy + 7, cx, cy + 16, WHITE);
    draw_arc_segment(frame, cx, cy + 4, 12, 1, 40, 140, WHITE);
}

fn draw_cancel_glyph(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    draw_line(frame, cx - 7, cy - 7, cx + 7, cy + 7, color);
    draw_line(frame, cx - 7, cy + 7, cx + 7, cy - 7, color);
}

fn draw_scroll_arc(frame: &mut [u16]) {
    draw_arc_segment(frame, CX, CY, 168, 2, 332, 28, MUTED);
}

fn draw_toggle_ring(frame: &mut [u16], cx: i32, cy: i32, on: bool) {
    let color = if on { ACCENT_SETTINGS } else { MUTED };
    draw_ring_meter(
        frame,
        cx,
        cy,
        54,
        3,
        if on { 82 } else { 25 },
        RING_DIM,
        color,
    );
    fill_circle(frame, cx, cy, 39, BG_DARK);
    fill_circle(frame, cx, if on { cy - 54 } else { cy + 54 }, 3, color);
}

fn draw_wifi_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    draw_arc_segment(frame, cx, cy + 6, 13, 1, 220, 320, color);
    draw_arc_segment(frame, cx, cy + 6, 8, 1, 230, 310, color);
    fill_circle(frame, cx, cy + 8, 2, color);
}

fn draw_bell_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    stroke_circle(frame, cx, cy, 9, color);
    fill_rect(frame, cx - 7, cy + 6, 14, 3, color);
    fill_circle(frame, cx, cy + 12, 2, color);
}

fn draw_mini_gear(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    stroke_circle(frame, cx, cy, 9, color);
    stroke_circle(frame, cx, cy, 4, color);
    for angle in (0..360).step_by(45) {
        let rad = (angle as f32).to_radians();
        let x0 = cx + (rad.cos() * 11.0).round() as i32;
        let y0 = cy + (rad.sin() * 11.0).round() as i32;
        let x1 = cx + (rad.cos() * 14.0).round() as i32;
        let y1 = cy + (rad.sin() * 14.0).round() as i32;
        draw_line(frame, x0, y0, x1, y1, color);
    }
}

fn draw_page_dots(model: &AppState, frame: &mut [u16]) {
    let count = ALL_PAGES.len() as i32;
    let spacing = 12;
    let start_x = CX - ((count - 1) * spacing / 2);

    for (idx, page) in ALL_PAGES.iter().copied().enumerate() {
        let selected = page == model.current_page;
        let r = if selected { 3 } else { 2 };
        let color = if selected {
            page_orchestration::accent_for_page(page)
        } else {
            SOFT
        };
        fill_circle(
            frame,
            start_x + idx as i32 * spacing,
            FOOTER_DOTS_Y,
            r,
            color,
        );
    }
}

fn pulse_exio<I2C>(exio: &mut Tca9554, i2c: &mut I2C, addr: u8, pin: u8)
where
    I2C: embedded_hal::i2c::I2c,
{
    let _ = exio.write_pin(i2c, addr, pin, false);
    thread::sleep(Duration::from_millis(10));

    let _ = exio.write_pin(i2c, addr, pin, true);
    thread::sleep(Duration::from_millis(50));
}

fn inside_circle(x: i32, y: i32) -> bool {
    let dx = x - CX;
    let dy = y - CY;
    dx * dx + dy * dy <= R_OUTER * R_OUTER
}

fn set_pixel(frame: &mut [u16], x: i32, y: i32, color: u16) {
    if x >= 0 && y >= 0 && x < W as i32 && y < H as i32 && inside_circle(x, y) {
        frame[y as usize * W + x as usize] = color;
    }
}

fn fill_circle(frame: &mut [u16], cx: i32, cy: i32, r: i32, color: u16) {
    let rr = r * r;
    for y in (cy - r).max(0)..=(cy + r).min(H as i32 - 1) {
        for x in (cx - r).max(0)..=(cx + r).min(W as i32 - 1) {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= rr {
                set_pixel(frame, x, y, color);
            }
        }
    }
}

fn stroke_circle(frame: &mut [u16], cx: i32, cy: i32, r: i32, color: u16) {
    let outer = r * r;
    let inner = (r - 1) * (r - 1);
    for y in (cy - r).max(0)..=(cy + r).min(H as i32 - 1) {
        for x in (cx - r).max(0)..=(cx + r).min(W as i32 - 1) {
            let dx = x - cx;
            let dy = y - cy;
            let d = dx * dx + dy * dy;
            if d <= outer && d >= inner {
                set_pixel(frame, x, y, color);
            }
        }
    }
}

fn fill_rect(frame: &mut [u16], x: i32, y: i32, w: i32, h: i32, color: u16) {
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + w).min(W as i32);
    let y1 = (y + h).min(H as i32);

    for yy in y0..y1 {
        for xx in x0..x1 {
            set_pixel(frame, xx, yy, color);
        }
    }
}

fn stroke_rect(frame: &mut [u16], x: i32, y: i32, w: i32, h: i32, color: u16) {
    draw_line(frame, x, y, x + w - 1, y, color);
    draw_line(frame, x, y + h - 1, x + w - 1, y + h - 1, color);
    draw_line(frame, x, y, x, y + h - 1, color);
    draw_line(frame, x + w - 1, y, x + w - 1, y + h - 1, color);
}

fn stroke_rounded_rect(frame: &mut [u16], x: i32, y: i32, w: i32, h: i32, r: i32, color: u16) {
    // v0.1.13-r1 compile repair: Settings Option A used rounded-row outlines
    // but the raw renderer only had fill_rounded_rect. Keep this helper local and
    // primitive-based so no renderer/touch behavior changes are introduced.
    if w <= 0 || h <= 0 {
        return;
    }

    let radius = r.max(0).min(w / 2).min(h / 2);
    if radius == 0 {
        stroke_rect(frame, x, y, w, h, color);
        return;
    }

    draw_line(frame, x + radius, y, x + w - radius - 1, y, color);
    draw_line(
        frame,
        x + radius,
        y + h - 1,
        x + w - radius - 1,
        y + h - 1,
        color,
    );
    draw_line(frame, x, y + radius, x, y + h - radius - 1, color);
    draw_line(
        frame,
        x + w - 1,
        y + radius,
        x + w - 1,
        y + h - radius - 1,
        color,
    );

    draw_arc_segment(frame, x + radius, y + radius, radius, 1, 180, 270, color);
    draw_arc_segment(
        frame,
        x + w - radius - 1,
        y + radius,
        radius,
        1,
        270,
        360,
        color,
    );
    draw_arc_segment(
        frame,
        x + w - radius - 1,
        y + h - radius - 1,
        radius,
        1,
        0,
        90,
        color,
    );
    draw_arc_segment(
        frame,
        x + radius,
        y + h - radius - 1,
        radius,
        1,
        90,
        180,
        color,
    );
}

fn fill_rounded_rect(frame: &mut [u16], x: i32, y: i32, w: i32, h: i32, r: i32, color: u16) {
    fill_rect(frame, x + r, y, w - 2 * r, h, color);
    fill_rect(frame, x, y + r, w, h - 2 * r, color);
    fill_circle(frame, x + r, y + r, r, color);
    fill_circle(frame, x + w - r - 1, y + r, r, color);
    fill_circle(frame, x + r, y + h - r - 1, r, color);
    fill_circle(frame, x + w - r - 1, y + h - r - 1, r, color);
}

fn draw_chip(
    frame: &mut [u16],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    label: &str,
    accent: u16,
    selected: bool,
) {
    let bg = if selected { 0x2124 } else { 0x1082 };
    fill_rounded_rect(frame, x, y, w, h, h / 2, bg);
    stroke_round_chip(frame, x, y, w, h, h / 2, accent);
    draw_text_centered_at(frame, x + w / 2, y + h / 2 + 4, label, WHITE, 1);
}

fn stroke_round_chip(frame: &mut [u16], x: i32, y: i32, w: i32, h: i32, r: i32, color: u16) {
    draw_line(frame, x + r, y, x + w - r, y, color);
    draw_line(frame, x + r, y + h - 1, x + w - r, y + h - 1, color);
    draw_arc_segment(frame, x + r, y + r, r, 1, 180, 270, color);
    draw_arc_segment(frame, x + w - r - 1, y + r, r, 1, 270, 0, color);
    draw_arc_segment(frame, x + r, y + h - r - 1, r, 1, 90, 180, color);
    draw_arc_segment(frame, x + w - r - 1, y + h - r - 1, r, 1, 0, 90, color);
}

fn draw_ring_meter(
    frame: &mut [u16],
    cx: i32,
    cy: i32,
    r: i32,
    thickness: i32,
    progress: u8,
    base: u16,
    accent: u16,
) {
    draw_arc_segment(frame, cx, cy, r, thickness, 135, 45, base);
    let sweep = (progress.min(100) as i32 * 270) / 100;
    let end = (135 + sweep) % 360;
    draw_arc_segment(frame, cx, cy, r, thickness, 135, end, accent);
}

fn draw_arc_segment(
    frame: &mut [u16],
    cx: i32,
    cy: i32,
    r: i32,
    thickness: i32,
    start_deg: i32,
    end_deg: i32,
    color: u16,
) {
    let thickness = thickness.max(1);
    let half = thickness / 2;

    for offset in -half..=half {
        draw_arc_line(frame, cx, cy, r + offset, start_deg, end_deg, color);
    }
}

fn draw_arc_line(
    frame: &mut [u16],
    cx: i32,
    cy: i32,
    r: i32,
    start_deg: i32,
    end_deg: i32,
    color: u16,
) {
    let mut angle = normalize_deg(start_deg);
    let end = normalize_deg(end_deg);
    let mut guard = 0;
    let mut prev: Option<(i32, i32)> = None;

    loop {
        let rad = (angle as f32).to_radians();
        let point = (
            cx + (rad.cos() * r as f32).round() as i32,
            cy + (rad.sin() * r as f32).round() as i32,
        );

        if let Some((px, py)) = prev {
            draw_line(frame, px, py, point.0, point.1, color);
        } else {
            set_pixel(frame, point.0, point.1, color);
        }

        prev = Some(point);

        if angle == end || guard >= 360 {
            break;
        }

        angle = normalize_deg(angle + 1);
        guard += 1;
    }
}

fn normalize_deg(deg: i32) -> i32 {
    let mut value = deg % 360;
    if value < 0 {
        value += 360;
    }
    value
}

fn draw_line(frame: &mut [u16], mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: u16) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        set_pixel(frame, x0, y0, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn fill_play_triangle(frame: &mut [u16], cx: i32, cy: i32, size: i32, color: u16) {
    let top = cy - size / 2;
    let bottom = cy + size / 2;
    let left = cx - size / 3;
    let right = cx + size / 2;

    for y in top..=bottom {
        let half = if y <= cy { y - top } else { bottom - y };
        let denom = (size / 2).max(1);
        let x_right = left + ((right - left) * half / denom);
        for x in left..=x_right {
            set_pixel(frame, x, y, color);
        }
    }
}

fn fill_left_triangle(frame: &mut [u16], cx: i32, cy: i32, size: i32, color: u16) {
    let top = cy - size / 2;
    let bottom = cy + size / 2;
    let right = cx + size / 3;
    let left = cx - size / 2;

    for y in top..=bottom {
        let half = if y <= cy { y - top } else { bottom - y };
        let denom = (size / 2).max(1);
        let x_left = right - ((right - left) * half / denom);
        for x in x_left..=right {
            set_pixel(frame, x, y, color);
        }
    }
}

fn draw_text_centered_at(frame: &mut [u16], cx: i32, y: i32, text: &str, color: u16, scale: i32) {
    let x = cx - text_width(text, scale) / 2;
    draw_text(frame, x, y, text, color, scale);
}

fn blit_rgb565_asset(frame: &mut [u16], asset: &[u8; RGB565_ASSET_BYTES]) {
    for (dst, px) in frame.iter_mut().zip(asset.chunks_exact(2)) {
        *dst = u16::from_le_bytes([px[0], px[1]]);
    }
}

fn draw_numeric_value_centered(
    frame: &mut [u16],
    y: i32,
    text: &str,
    digit_w: i32,
    stroke: i32,
    color: u16,
) {
    let width = numeric_value_width(text, digit_w, stroke);
    let mut x = CX - width / 2;

    for ch in text.chars() {
        let char_w = numeric_char_width(ch, digit_w, stroke);
        draw_numeric_char(frame, x, y, ch, digit_w, stroke, color);
        x += char_w + stroke;
    }
}

fn numeric_value_width(text: &str, digit_w: i32, stroke: i32) -> i32 {
    let mut width = 0;
    let mut first = true;

    for ch in text.chars() {
        if !first {
            width += stroke;
        }
        width += numeric_char_width(ch, digit_w, stroke);
        first = false;
    }

    width
}

fn numeric_char_width(ch: char, digit_w: i32, stroke: i32) -> i32 {
    match ch {
        ':' => stroke * 3,
        'F' | 'f' => digit_w - stroke * 2,
        '-' => digit_w / 2,
        _ => digit_w,
    }
}

fn draw_numeric_char(
    frame: &mut [u16],
    x: i32,
    y: i32,
    ch: char,
    digit_w: i32,
    stroke: i32,
    color: u16,
) {
    match ch {
        '0' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [true, true, true, true, true, true, false],
        ),
        '1' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [false, true, true, false, false, false, false],
        ),
        '2' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [true, true, false, true, true, false, true],
        ),
        '3' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [true, true, true, true, false, false, true],
        ),
        '4' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [false, true, true, false, false, true, true],
        ),
        '5' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [true, false, true, true, false, true, true],
        ),
        '6' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [true, false, true, true, true, true, true],
        ),
        '7' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [true, true, true, false, false, false, false],
        ),
        '8' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [true, true, true, true, true, true, true],
        ),
        '9' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [true, true, true, true, false, true, true],
        ),
        ':' => {
            fill_circle(frame, x + stroke, y + digit_w / 2, stroke / 2 + 1, color);
            fill_circle(
                frame,
                x + stroke,
                y + digit_w + digit_w / 2,
                stroke / 2 + 1,
                color,
            );
        }
        'F' | 'f' => draw_letter_f(frame, x, y, digit_w, stroke, color),
        '-' => fill_rounded_rect(
            frame,
            x,
            y + digit_w,
            digit_w / 2,
            stroke,
            stroke / 2,
            color,
        ),
        _ => {}
    }
}

fn draw_segment_mask(
    frame: &mut [u16],
    x: i32,
    y: i32,
    w: i32,
    s: i32,
    color: u16,
    seg: [bool; 7],
) {
    let h = w * 2 - s;
    let mid = y + h / 2 - s / 2;
    let bottom = y + h - s;

    if seg[0] {
        fill_rounded_rect(frame, x + s, y, w - 2 * s, s, s / 2, color);
    }
    if seg[1] {
        fill_rounded_rect(frame, x + w - s, y + s, s, h / 2 - s, s / 2, color);
    }
    if seg[2] {
        fill_rounded_rect(frame, x + w - s, mid + s, s, h / 2 - s, s / 2, color);
    }
    if seg[3] {
        fill_rounded_rect(frame, x + s, bottom, w - 2 * s, s, s / 2, color);
    }
    if seg[4] {
        fill_rounded_rect(frame, x, mid + s, s, h / 2 - s, s / 2, color);
    }
    if seg[5] {
        fill_rounded_rect(frame, x, y + s, s, h / 2 - s, s / 2, color);
    }
    if seg[6] {
        fill_rounded_rect(frame, x + s, mid, w - 2 * s, s, s / 2, color);
    }
}

fn draw_letter_f(frame: &mut [u16], x: i32, y: i32, w: i32, s: i32, color: u16) {
    let h = w * 2 - s;
    fill_rounded_rect(frame, x, y, s, h, s / 2, color);
    fill_rounded_rect(frame, x, y, w - s, s, s / 2, color);
    fill_rounded_rect(frame, x, y + h / 2 - s / 2, w - s * 2, s, s / 2, color);
}

fn text_width(text: &str, scale: i32) -> i32 {
    text.chars().count() as i32 * 6 * scale
}

fn draw_text_centered(frame: &mut [u16], y: i32, text: &str, color: u16, scale: i32) {
    let x = CX - text_width(text, scale) / 2;
    draw_text(frame, x, y, text, color, scale);
}

fn draw_text(frame: &mut [u16], x: i32, y: i32, text: &str, color: u16, scale: i32) {
    let mut cursor_x = x;

    for ch in text.chars() {
        draw_char(frame, cursor_x, y, ch, color, scale);
        cursor_x += 6 * scale;
    }
}

fn draw_char(frame: &mut [u16], x: i32, y: i32, ch: char, color: u16, scale: i32) {
    let glyph = glyph_5x7(ch);

    for (row, bits) in glyph.iter().enumerate() {
        for col in 0..5 {
            if ((bits >> (4 - col)) & 0x01) != 0 {
                fill_rect(
                    frame,
                    x + (col * scale),
                    y + (row as i32 * scale) - (7 * scale) + scale,
                    scale,
                    scale,
                    color,
                );
            }
        }
    }
}

fn glyph_5x7(ch: char) -> [u8; 7] {
    // RADIO_R34_LOWERCASE_ASCII_GLYPHS
    // The 5x7 table stores uppercase glyphs only. Convert ASCII
    // lowercase before matching so raw station labels such as
    // SanskarRadio and NonStopHindi do not render as S R / N S H.
    let ch = ch.to_ascii_uppercase();
    match ch {
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' => [0x1C, 0x12, 0x11, 0x11, 0x11, 0x12, 0x1C],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0E],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1F],
        'J' => [0x01, 0x01, 0x01, 0x01, 0x11, 0x11, 0x0E],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x1B, 0x11],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        ':' => [0x00, 0x04, 0x04, 0x00, 0x04, 0x04, 0x00],
        '-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        '/' => [0x01, 0x01, 0x02, 0x04, 0x08, 0x10, 0x10],
        '%' => [0x18, 0x19, 0x02, 0x04, 0x08, 0x13, 0x03],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x04, 0x04, 0x08],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
        '<' => [0x01, 0x02, 0x04, 0x08, 0x04, 0x02, 0x01],
        '>' => [0x10, 0x08, 0x04, 0x02, 0x04, 0x08, 0x10],
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        _ => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    }
}

// v0.1.36-r31: removed legacy r27 station overlay call.
// v0.1.36-r27: final overlay.  The reused Music-style radio page can still
// draw compact station initials ("N S H", "I F", "S 1") after the normal
// title path.  Clear the title band and draw the real RADIO.TXT name last.
// v0.1.36-r31-r2: removed orphan r27 station-title overlay fragment.
// v0.1.36-r31-r2: removed legacy r27 station-title helper call.

// RAW-R42-VIDEO-DEAD-SOURCE-CLEANUP

// RAW-R42-R1-VIDEO-CALLSITE-COMPILE-REPAIR

// RAW-R45-HOME-MODULE-CALLSITE

// RAW-R45-R1-HOME-MODULE-COMPILE-REPAIR

// RAW-R46-R1-WEATHER-MODULE-CALLSITE

// RAW-R47-MUSIC-MODULE-CALLSITE

// RAW-R48-ASSISTANT-MODULE-CALLSITE

// RAW-R49-SETTINGS-MODULE-CALLSITE

// RAW-R51-MAIN-ORCHESTRATION-CALLSITES

// RAW-R51-R1-PAGE-ORCHESTRATION-AFTER-DEBUG-MACRO

// RAW-R52-TOUCH-ROUTER-AFTER-DEBUG-MACRO

// RAW-R52-R1-MAIN-TOUCH-ROUTER-CALLSITE

// RAW-R53-PAGE-ASSETS-MODULE-AFTER-DEBUG-MACRO

// RAW-R54-MEDIA-ACTION-ROUTER-AFTER-DEBUG-MACRO

// RAW-R55-SETTINGS-ACTION-ROUTER-AFTER-DEBUG-MACRO

// RAW-R56-WEATHER-ACTION-ROUTER-AFTER-DEBUG-MACRO

// RAW-R56-R1-WEATHER-ACTION-ROUTER-AFTER-DEBUG-MACRO
