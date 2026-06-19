#![allow(dead_code)]

mod app;
mod board;
mod drivers;
mod ffi;

use anyhow::{bail, Result};
use app::{
    actions::handle_select_action,
    model::{ButtonName, ButtonPressKind, OnboardModel, UiIntent},
    pages::{AssistantPage, ALL_PAGES},
};
use core::{ffi::c_void, mem::size_of, ptr::NonNull, slice};
use drivers::{cst816::Cst816, pcf85063::Pcf85063, tca9554::Tca9554};
use embedded_svc::wifi::{ClientConfiguration, Configuration, Wifi};
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
    thread,
    time::{Duration, Instant},
};

const W: usize = 360;
const H: usize = 360;
const PIXELS: usize = W * H;
const RGB565_ASSET_BYTES: usize = PIXELS * 2;
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

static HOME_BASE_RGB565: &[u8; RGB565_ASSET_BYTES] =
    include_bytes!("../assets/rgb565/home_base.rgb565");
static WEATHER_BASE_RGB565: &[u8; RGB565_ASSET_BYTES] =
    include_bytes!("../assets/rgb565/weather_base.rgb565");
static MUSIC_BASE_RGB565: &[u8; RGB565_ASSET_BYTES] =
    include_bytes!("../assets/rgb565/music_base.rgb565");
static ASSISTANT_BASE_RGB565: &[u8; RGB565_ASSET_BYTES] =
    include_bytes!("../assets/rgb565/assistant_base.rgb565");
static SETTINGS_BASE_RGB565: &[u8; RGB565_ASSET_BYTES] =
    include_bytes!("../assets/rgb565/settings_base.rgb565");

const TOUCH_POLL_MS: u64 = 8;
const BUTTON_POLL_MS: u64 = 25;
const POWER_LONG_PRESS_MS: u64 = 850;
const BUTTON_DEBOUNCE_MS: u64 = 40;
const BATTERY_REFRESH_MS: u64 = 5_000;
const WIFI_REFRESH_MS: u64 = 30_000;
const RTC_REFRESH_MS: u64 = 1_000;
const TOUCH_TAP_MAX_MS: u64 = 450;
const TAP_MOVEMENT_MAX_PX: i16 = 25;
const TOUCH_NAV_COOLDOWN_MS: u64 = 300;
const TOUCH_ACTIVE_POLL_WINDOW_MS: u64 = 180;
const TOUCH_NO_TOUCH_FINISH_COUNT: u8 = 3;
const TOUCH_GESTURE_SPAN_PREFER_PX: i16 = 35;
const UNIVERSAL_SWIPE_MIN_DX: i16 = 20;
const CENTER_TAP_X_MIN: u16 = 95;
const CENTER_TAP_X_MAX: u16 = 265;
const CENTER_TAP_Y_MIN: u16 = 95;
const CENTER_TAP_Y_MAX: u16 = 285;
const CENTER_TAP_MAX_MOVE_PX: i16 = 12;
const CST816_GESTURE_LEFT: u8 = 0x03;
const CST816_GESTURE_RIGHT: u8 = 0x04;
const RENDER_MIN_INTERVAL_MS: u64 = 80;

const BAT_ATTEN: adc_atten_t = attenuation::DB_11;

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

        println!("touch-track: begin id={}", self.touch_id);
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

        println!(
            "touch-track: sample id={} source={} x={} y={} gesture=0x{:02X}",
            self.touch_id, source, x, y, gesture
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
        println!(
            "touch-track: no-touch id={} count={}",
            self.touch_id, self.no_touch_count
        );
    }

    fn active_elapsed(&self, now: Instant) -> Duration {
        self.start_at
            .map(|start| now.duration_since(start))
            .unwrap_or_default()
    }

    fn finish_reason(&self, now: Instant) -> Option<&'static str> {
        if !self.active {
            return None;
        }

        if self.no_touch_count >= TOUCH_NO_TOUCH_FINISH_COUNT {
            return Some("no-touch");
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

        println!("touch-track: reset id={}", self.touch_id);
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

        let raw = unsafe { heap_caps_malloc(bytes, MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT) as *mut u16 };

        let ptr = NonNull::new(raw).ok_or_else(|| anyhow::anyhow!("PSRAM framebuffer alloc failed"))?;

        unsafe {
            core::ptr::write_bytes(ptr.as_ptr(), 0, len_words);
        }

        Ok(Self { ptr, len_words })
    }

    fn as_mut_slice(&mut self) -> &mut [u16] {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len_words) }
    }
}

impl Drop for FrameBuffer {
    fn drop(&mut self) {
        unsafe {
            heap_caps_free(self.ptr.as_ptr() as *mut c_void);
        }
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

fn run_app() -> Result<()> {
    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;
    let modem = peripherals.modem;

    let mut power_button = PinDriver::input(pins.gpio6)?;
    power_button.set_pull(Pull::Up)?;

    let mut backlight = PinDriver::output(pins.gpio5)?;
    backlight.set_low()?;

    let mut touch_int = PinDriver::input(pins.gpio4)?;
    touch_int.set_pull(Pull::Up)?;

    let i2c_cfg = I2cConfig::new().baudrate(400.kHz().into());
    let mut i2c = I2cDriver::new(peripherals.i2c0, pins.gpio11, pins.gpio10, &i2c_cfg)?;

    let adc = AdcDriver::new(peripherals.adc1)?;
    let bat_config = AdcChannelConfig {
        attenuation: BAT_ATTEN,
        ..Default::default()
    };
    let mut bat_pin = AdcChannelDriver::new(&adc, pins.gpio8, &bat_config)?;

    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let mut wifi = EspWifi::new(modem, sys_loop, Some(nvs))?;
    let _ = wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()));
    let _ = wifi.start();

    let mut exio = Tca9554::new();
    let touch = Cst816::new();
    let rtc = Pcf85063::new();

    let exio_ok = exio.ping(&mut i2c, board::TCA9554_ADDR).is_ok();
    let touch_ok = touch.ping(&mut i2c, board::CST816_ADDR).is_ok();
    let rtc_ok = rtc.ping(&mut i2c, board::PCF85063_ADDR).is_ok();

    let mut model = OnboardModel::new();
    model.set_probe_status(exio_ok && touch_ok && rtc_ok, touch_ok, rtc_ok);
    model.backlight_percent = 100;

    println!("\n=== {} ===", board::BOARD_NAME);
    println!("Hybrid Rust + ESP-IDF display backend");
    println!("v0.1.14-r2 Weather Baseline Guard Marker Repair");
    println!("Screens frozen: Home r3 | Weather r8-r2 | Music v0.1.11 | Assistant v0.1.12 | Settings Option A");
    println!(
        "Input: r12 gesture-first touch, poll={}ms, window={}ms, cooldown={}ms",
        TOUCH_POLL_MS, TOUCH_ACTIVE_POLL_WINDOW_MS, TOUCH_NAV_COOLDOWN_MS
    );
    println!("Renderer: hybrid RGB565 five page assets + dynamic overlays");
    println!("Integrations: mocked/local; periodic SD/GPIO refresh disabled");
    println!("UI baseline: frozen five-screen layout with regression guards");
    println!("Asset guard: stale non-frozen RGB565 files are cleaned before validation");
    println!("Weather guard: timeline temp marker aligned with accepted y=262 layout");
    println!("Build cleanup: retained fallback helpers use crate-level dead_code allowance");
    println!("button-loop: poll={}ms", BUTTON_POLL_MS);
    println!("I2C probes:");
    println!(
        "  0x20 TCA9554  => {:?}",
        exio.ping(&mut i2c, board::TCA9554_ADDR)
    );
    println!(
        "  0x15 CST816   => {:?}",
        touch.ping(&mut i2c, board::CST816_ADDR)
    );
    println!(
        "  0x51 PCF85063 => {:?}",
        rtc.ping(&mut i2c, board::PCF85063_ADDR)
    );

    let _ = exio.set_config(&mut i2c, board::TCA9554_ADDR, 0x00);
    let _ = exio.write_pin(&mut i2c, board::TCA9554_ADDR, board::EXIO_TOUCH_RST, true);
    let _ = exio.write_pin(&mut i2c, board::TCA9554_ADDR, board::EXIO_LCD_RST, true);
    let _ = exio.write_pin(&mut i2c, board::TCA9554_ADDR, board::EXIO_SD_CS, true);

    pulse_exio(
        &mut exio,
        &mut i2c,
        board::TCA9554_ADDR,
        board::EXIO_LCD_RST,
    );

    if !unsafe { ffi::st77916_panel_init() } {
        bail!("st77916_panel_init() failed");
    }

    println!("panel init ok");

    backlight.set_high()?;
    println!("backlight on");

    pulse_exio(
        &mut exio,
        &mut i2c,
        board::TCA9554_ADDR,
        board::EXIO_TOUCH_RST,
    );

    let touch_cfg = touch.read_config(&mut i2c, board::CST816_ADDR).ok();
    let _ = touch.disable_auto_sleep(&mut i2c, board::CST816_ADDR);

    if let Some(cfg) = touch_cfg {
        println!(
            "Touch cfg: version=0x{:02X} chip_id=0x{:02X} project_id=0x{:02X} fw=0x{:02X}",
            cfg.version, cfg.chip_id, cfg.project_id, cfg.fw_version
        );
    }

    if let Ok(dt) = rtc.read_datetime(&mut i2c, board::PCF85063_ADDR) {
        model.update_rtc(dt);
    }

    let mut frame = FrameBuffer::new_rgb565(W * H)?;

    println!(
        "heap free: 8bit={} psram={}",
        unsafe { heap_caps_get_free_size(MALLOC_CAP_8BIT) },
        unsafe { heap_caps_get_free_size(MALLOC_CAP_SPIRAM) }
    );

    if let Ok(raw) = adc.read(&mut bat_pin) {
        let adc_mv = (raw as f32 / 4095.0) * 3300.0;
        let battery_mv =
            ((adc_mv * board::BATTERY_DIVIDER_SCALE) / board::BATTERY_MEASUREMENT_OFFSET) as u16;
        model.battery_mv = Some(battery_mv);
    }
    refresh_wifi(&mut model, &mut wifi);
    refresh_sd(&mut model);

    let mut dirty = true;
    let mut last_render = Instant::now() - Duration::from_millis(RENDER_MIN_INTERVAL_MS);
    render_if_dirty(&mut dirty, &model, frame.as_mut_slice(), true, &mut last_render)?;
    println!("polished circular home page rendered");

    let mut touch_tracker = TouchTracker::default();
    let mut power_tracker = ButtonTracker::new(Duration::from_millis(POWER_LONG_PRESS_MS));
    let mut last_touch_poll = Instant::now();
    let mut last_button_poll = Instant::now();
    let mut last_rtc = Instant::now();
    let mut last_battery = Instant::now();
    let mut last_wifi = Instant::now();
    let mut last_navigation = Instant::now() - Duration::from_millis(TOUCH_NAV_COOLDOWN_MS);

    loop {
        let now = Instant::now();

        if last_touch_poll.elapsed() >= Duration::from_millis(TOUCH_POLL_MS) {
            last_touch_poll = now;

            if touch_tracker.active {
                match touch.read_touch(&mut i2c, board::CST816_ADDR) {
                    Ok(point) if point.fingers > 0 => {
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

                if let Some(reason) = touch_tracker.finish_reason(now) {
                    if let Some(summary) = touch_tracker.finish(now, reason) {
                        if process_touch_summary(&mut model, summary, now, &mut last_navigation) {
                            dirty = true;
                        }
                    }
                }
            } else if touch_int.is_low() {
                if let Ok(point) = touch.read_touch(&mut i2c, board::CST816_ADDR) {
                    if point.fingers > 0 {
                        model.note_touch(point.x, point.y, point.fingers, point.gesture);
                        touch_tracker.update_down(now, "int", point.x, point.y, point.gesture);
                    }
                }
            }
        }

        if last_button_poll.elapsed() >= Duration::from_millis(BUTTON_POLL_MS) {
            last_button_poll = now;
            if let Some(event) = power_tracker.update(now, power_button.is_low()) {
                handle_button_event(&mut model, ButtonName::Power, event);
                dirty = true;
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

        if !touch_tracker.active && last_battery.elapsed() >= Duration::from_millis(BATTERY_REFRESH_MS) {
            last_battery = now;
            let previous_battery = model.battery_mv;
            if let Ok(raw) = adc.read(&mut bat_pin) {
                let adc_mv = (raw as f32 / 4095.0) * 3300.0;
                let battery_mv = ((adc_mv * board::BATTERY_DIVIDER_SCALE)
                    / board::BATTERY_MEASUREMENT_OFFSET) as u16;
                model.battery_mv = Some(battery_mv);
            }
            if model.current_page == AssistantPage::Home && previous_battery != model.battery_mv {
                dirty = true;
            }
        }

        if !touch_tracker.active && last_wifi.elapsed() >= Duration::from_millis(WIFI_REFRESH_MS) {
            last_wifi = now;
            let previous_wifi = model.wifi_count;
            refresh_wifi(&mut model, &mut wifi);
            if model.current_page == AssistantPage::Home && previous_wifi != model.wifi_count {
                dirty = true;
            }
        }

        render_if_dirty(
            &mut dirty,
            &model,
            frame.as_mut_slice(),
            !touch_tracker.active,
            &mut last_render,
        )?;
        thread::sleep(Duration::from_millis(5));
    }
}

fn refresh_wifi(model: &mut OnboardModel, wifi: &mut EspWifi<'static>) {
    let ap_count = match wifi.scan() {
        Ok(aps) => Some(aps.len() as u16),
        Err(_) => None,
    };

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

    model.update_wifi_status(ap_count, connected, ssid);
}

fn refresh_sd(model: &mut OnboardModel) {
    let mut present = false;
    let mut capacity_mb = 0u32;

    let ok = unsafe { ffi::st77916_probe_sd_capacity_mb(&mut present, &mut capacity_mb) };

    model.sd_present = ok && present;
    model.sd_capacity_mb = if ok && present && capacity_mb > 0 {
        Some(capacity_mb)
    } else {
        None
    };
}

fn render_if_dirty(
    dirty: &mut bool,
    model: &OnboardModel,
    frame: &mut [u16],
    allow_render: bool,
    last_render: &mut Instant,
) -> Result<()> {
    if !*dirty || !allow_render {
        return Ok(());
    }

    if last_render.elapsed() < Duration::from_millis(RENDER_MIN_INTERVAL_MS) {
        return Ok(());
    }

    draw_assistant_page(model, frame)?;
    println!("render: coalesced repaint ok");
    *last_render = Instant::now();
    *dirty = false;
    Ok(())
}

fn process_touch_summary(
    model: &mut OnboardModel,
    summary: TouchSummary,
    now: Instant,
    last_navigation: &mut Instant,
) -> bool {
    println!(
        "touch-track: finish id={} reason={} samples={} start=({}, {}) end=({}, {}) minmax=({}, {})..({}, {}) span={} dx={} dy={} ms={} gesture=0x{:02X}",
        summary.touch_id,
        summary.finish_reason,
        summary.sample_count,
        summary.start_x,
        summary.start_y,
        summary.end_x,
        summary.end_y,
        summary.min_x,
        summary.min_y,
        summary.max_x,
        summary.max_y,
        summary.span_x,
        summary.dx,
        summary.dy,
        summary.duration_ms,
        summary.gesture
    );

    if let Some(intent) = intent_from_touch_summary(&summary) {
        if matches!(intent, UiIntent::NextPage | UiIntent::PreviousPage) {
            if now.duration_since(*last_navigation) < Duration::from_millis(TOUCH_NAV_COOLDOWN_MS) {
                println!("touch: navigation ignored during cooldown");
                return false;
            }
            *last_navigation = now;
        }

        handle_intent(model, intent);
        true
    } else {
        println!("touch: ignored movement/tap outside classifier thresholds");
        false
    }
}

fn handle_button_event(model: &mut OnboardModel, button: ButtonName, event: ButtonEvent) {
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
            println!("button: POWER short -> back/home");
            handle_intent(model, UiIntent::BackHome);
        }
        (ButtonName::Power, ButtonEvent::Long) => {
            println!("button: POWER long -> power menu placeholder");
            handle_intent(model, UiIntent::PowerMenu);
        }
    }
}

fn handle_intent(model: &mut OnboardModel, intent: UiIntent) {
    match intent {
        UiIntent::NextPage => {
            model.next_page();
            println!("nav: NextPage -> {:?}", model.current_page);
        }
        UiIntent::PreviousPage => {
            model.previous_page();
            println!("nav: PreviousPage -> {:?}", model.current_page);
        }
        UiIntent::Select => {
            model.note_intent(UiIntent::Select);
            let action = handle_select_action(model);
            println!("{}", action.log_marker());
        }
        UiIntent::BackHome => {
            model.set_page(AssistantPage::Home);
            model.note_intent(UiIntent::BackHome);
            println!("nav: BackHome -> Home");
        }
        UiIntent::AssistantHold => {
            model.note_intent(UiIntent::AssistantHold);
            println!("action: Assistant placeholder");
        }
        UiIntent::BootReserved => {
            model.note_intent(UiIntent::BootReserved);
            println!("action: BOOT reserved");
        }
        UiIntent::PowerMenu => {
            model.note_intent(UiIntent::PowerMenu);
            println!("action: Power menu placeholder");
        }
    }
}

fn intent_from_touch_summary(summary: &TouchSummary) -> Option<UiIntent> {
    let left_travel = summary.start_x as i16 - summary.min_x as i16;
    let right_travel = summary.max_x as i16 - summary.start_x as i16;
    let signed_span_dx = if left_travel >= right_travel {
        -left_travel
    } else {
        right_travel
    };
    let abs_span_dx = signed_span_dx.abs();
    let abs_span_y = summary.span_y.abs();
    let horizontal_dominant = (abs_span_dx as i32 * 2) > (abs_span_y as i32 * 3);

    let gesture_intent = match summary.gesture {
        CST816_GESTURE_LEFT => Some(UiIntent::NextPage),
        CST816_GESTURE_RIGHT => Some(UiIntent::PreviousPage),
        _ => None,
    };

    let span_intent = if abs_span_dx >= UNIVERSAL_SWIPE_MIN_DX && horizontal_dominant {
        if signed_span_dx < 0 {
            Some(UiIntent::NextPage)
        } else {
            Some(UiIntent::PreviousPage)
        }
    } else {
        None
    };

    if let Some(gesture_intent) = gesture_intent {
        if let Some(span_intent) = span_intent {
            if gesture_intent != span_intent {
                println!(
                    "touch-class: gesture/span disagree gesture=0x{:02X} span_dx={} span={} prefer={}",
                    summary.gesture,
                    signed_span_dx,
                    summary.span_x,
                    if abs_span_dx < TOUCH_GESTURE_SPAN_PREFER_PX {
                        "gesture"
                    } else {
                        "span"
                    }
                );

                if abs_span_dx >= TOUCH_GESTURE_SPAN_PREFER_PX {
                    return log_span_intent(span_intent);
                }
            }
        }

        return log_gesture_intent(gesture_intent);
    }

    if let Some(span_intent) = span_intent {
        return log_span_intent(span_intent);
    }

    let center_tap = summary.span_x <= CENTER_TAP_MAX_MOVE_PX
        && summary.span_y <= CENTER_TAP_MAX_MOVE_PX
        && summary.duration_ms <= TOUCH_TAP_MAX_MS as u128
        && (CENTER_TAP_X_MIN..=CENTER_TAP_X_MAX).contains(&summary.end_x)
        && (CENTER_TAP_Y_MIN..=CENTER_TAP_Y_MAX).contains(&summary.end_y);

    if center_tap {
        println!("touch-class: center-tap accepted");
        return Some(UiIntent::Select);
    }

    if summary.sample_count < 2 {
        println!(
            "touch-class: ignored insufficient samples samples={} dx={} dy={} span={} gesture=0x{:02X}",
            summary.sample_count, summary.dx, summary.dy, summary.span_x, summary.gesture
        );
        return None;
    }

    if abs_span_y >= UNIVERSAL_SWIPE_MIN_DX && abs_span_y > abs_span_dx {
        println!(
            "touch-class: ignored vertical swipe dx={} dy={} span={} gesture=0x{:02X}",
            summary.dx, summary.dy, summary.span_x, summary.gesture
        );
        return None;
    }

    println!(
        "touch-class: ignored below-threshold movement dx={} dy={} span={} gesture=0x{:02X}",
        summary.dx, summary.dy, summary.span_x, summary.gesture
    );
    None
}

fn log_gesture_intent(intent: UiIntent) -> Option<UiIntent> {
    match intent {
        UiIntent::NextPage => println!("touch-class: gesture-left accepted next"),
        UiIntent::PreviousPage => println!("touch-class: gesture-right accepted previous"),
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


fn draw_assistant_page(model: &OnboardModel, frame: &mut [u16]) -> Result<()> {
    frame.fill(BLACK);
    fill_circle(frame, CX, CY, R_OUTER, BG);
    stroke_circle(frame, CX, CY, R_OUTER, RING_DIM);

    match model.current_page {
        AssistantPage::Home => draw_home_tile(model, frame),
        AssistantPage::Weather => draw_weather_tile(model, frame),
        AssistantPage::Music => draw_music_tile(model, frame),
        AssistantPage::Assistant => draw_ai_assistant_tile(model, frame),
        AssistantPage::Settings => draw_settings_tile(model, frame),
    }

    draw_page_dots(model, frame);

    if !unsafe { ffi::st77916_panel_draw_rgb565(0, 0, 359, 359, frame.as_mut_ptr()) } {
        bail!("st77916_panel_draw_rgb565 returned false");
    }

    Ok(())
}

fn draw_home_tile(model: &OnboardModel, frame: &mut [u16]) {
    if model.home.status_detail_open {
        draw_watch_outer(frame, ACCENT_HOME);
        draw_arc_segment(frame, CX, CY, 166, 2, 204, 258, ACCENT_HOME_BLUE);
        draw_arc_segment(frame, CX, CY, 166, 2, 282, 336, ACCENT_HOME);
        draw_arc_segment(frame, CX, CY, 166, 2, 24, 76, ACCENT_WEATHER);
        draw_arc_segment(frame, CX, CY, 166, 2, 104, 156, ACCENT_HOME_GREEN);
        draw_assistant_orb(frame, 180, 112, 28, ACCENT_HOME);
        draw_chip(frame, 78, 170, 88, 30, "BAT", ACCENT_HOME_BLUE, true);
        draw_chip(frame, 194, 170, 88, 30, "WIFI", ACCENT_HOME, true);
        draw_chip(frame, 116, 218, 128, 30, "SD / RTC", ACCENT_HOME_GREEN, true);
        draw_text_centered(frame, 270, model.home.detail_label(), ACCENT_HOME, 1);
    } else {
        blit_rgb565_asset(frame, HOME_BASE_RGB565);

        // v0.1.10-r2 Home Option C Minimal Dashboard.
        // Option C avoids heavy arcs and large top pills; it uses compact status labels,
        // a centered date capsule, dominant time, and a split weather card.
        draw_home_battery_complication(frame, 66, 58, model.battery_percent_value());
        draw_text(frame, 86, 53, &model.battery_home_text(), WHITE, 1);

        draw_wifi_icon(frame, 218, 58, WHITE);
        draw_text(frame, 238, 53, &model.wifi_home_text(), WHITE, 1);

        draw_text_centered_at(frame, 180, 102, &model.rtc_home_date_text(), WHITE, 2);

        draw_numeric_value_centered(frame, 122, &model.rtc_hms(), 42, 6, WHITE);

        let condition = model.home_weather_condition();
        draw_home_weather_icon(frame, 106, 250, condition);
        draw_text_centered_at(frame, 106, 278, condition, WHITE, 1);
        draw_text_centered_at(frame, 247, 262, model.home_weather_temp(), WHITE, 3);
    }
}

fn draw_home_battery_complication(frame: &mut [u16], cx: i32, cy: i32, percent: Option<u8>) {
    let x = cx - 10;
    let y = cy - 6;
    stroke_rect(frame, x, y, 20, 12, WHITE);
    fill_rect(frame, x + 21, y + 4, 3, 4, WHITE);

    if let Some(pct) = percent {
        let fill_w = ((pct.min(100) as i32) * 16 / 100).max(2);
        fill_rect(frame, x + 2, y + 2, fill_w, 8, ACCENT_HOME);
    } else {
        fill_rect(frame, x + 2, y + 2, 8, 8, ACCENT_HOME_BLUE);
    }
}

fn draw_calendar_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    stroke_rect(frame, cx - 7, cy - 7, 14, 14, color);
    draw_line(frame, cx - 7, cy - 3, cx + 7, cy - 3, color);
    draw_line(frame, cx - 4, cy - 10, cx - 4, cy - 6, color);
    draw_line(frame, cx + 4, cy - 10, cx + 4, cy - 6, color);
    fill_rect(frame, cx - 3, cy + 1, 2, 2, color);
    fill_rect(frame, cx + 2, cy + 1, 2, 2, color);
}

fn draw_home_weather_icon(frame: &mut [u16], cx: i32, cy: i32, condition: &str) {
    match condition {
        "CLEAR" | "SUNNY" => draw_timeline_sun_icon(frame, cx, cy, ACCENT_WEATHER),
        "CLOUDY" | "LOCAL" => draw_timeline_cloud_icon(frame, cx, cy, WHITE),
        "RAIN" => draw_timeline_rain_icon(frame, cx, cy),
        "STORM" => draw_timeline_storm_icon(frame, cx, cy),
        "BREEZY" => draw_timeline_wind_icon(frame, cx, cy, WHITE),
        _ => draw_timeline_cloud_icon(frame, cx, cy, WHITE),
    }
}


fn draw_weather_tile(model: &OnboardModel, frame: &mut [u16]) {
    blit_rgb565_asset(frame, WEATHER_BASE_RGB565);

    // Option D large-strip layout:
    // main summary is compact; the lower third has large pill cards.
    // main icon y=34..64, temperature y=70..118, condition y=138,
    // large timeline cards y=176..278, sample y=306.
    let temp = if model.weather.temperature_label == "--" {
        "72F"
    } else {
        model.weather.temperature_label
    };
    let condition = compact_condition(model.weather.condition_label);

    match condition {
        "CLEAR" | "SUNNY" => draw_sun_icon(frame, 180, 40, 11, ACCENT_WEATHER),
        "CLOUDY" => draw_cloud_icon_medium(frame, 180, 42, WHITE),
        "BREEZY" | "LOCAL" => draw_wind_icon_compact(frame, 180, 42, WHITE),
        _ => draw_wind_icon_compact(frame, 180, 42, WHITE),
    }

    draw_numeric_value_centered(frame, 70, temp, 24, 4, WHITE);
    draw_text_centered(frame, 138, condition, WHITE, 2);
    draw_weather_hour_values(frame, condition);
    draw_text_centered(
        frame,
        306,
        &format!("SAMPLE {}", model.weather.refresh_attempts),
        WHITE,
        1,
    );
}

fn draw_music_tile(model: &OnboardModel, frame: &mut [u16]) {
    blit_rgb565_asset(frame, MUSIC_BASE_RGB565);

    // v0.1.11 Music Screen Option C Minimal Equalizer.
    // Base asset provides equalizer, button shells, progress track, and source pill.
    // Runtime overlays keep local/mock state live and avoid overlap.
    draw_text_centered_at(frame, 180, 38, &model.rtc_hms(), WHITE, 2);
    draw_text_centered_at(frame, 180, 134, model.music.track_label, WHITE, 3);
    draw_text_centered_at(frame, 180, 162, model.music.subtitle_label, ACCENT_MUSIC_BLUE, 2);

    draw_music_transport_controls(frame, model.music.playing);
    draw_music_progress_row(
        frame,
        model.music.progress_percent(),
        model.music.elapsed_label(),
        model.music.duration_label(),
    );
    draw_text_centered_at(frame, 180, 318, model.music.source, WHITE, 1);
}


fn draw_music_transport_controls(frame: &mut [u16], playing: bool) {
    // Side buttons: compact and readable.
    fill_circle(frame, 80, 218, 28, BG_DARK);
    stroke_circle(frame, 80, 218, 28, ACCENT_MUSIC_BLUE);
    draw_skip_icon(frame, 80, 218, false, WHITE);

    fill_circle(frame, 280, 218, 28, BG_DARK);
    stroke_circle(frame, 280, 218, 28, ACCENT_MUSIC_BLUE);
    draw_skip_icon(frame, 280, 218, true, WHITE);

    // Center button: primary control.
    fill_circle(frame, 180, 222, 44, BG_DARK);
    stroke_circle(frame, 180, 222, 45, ACCENT_MUSIC_BLUE);
    stroke_circle(frame, 180, 222, 39, RING_DIM);

    if playing {
        fill_rounded_rect(frame, 168, 200, 8, 44, 4, WHITE);
        fill_rounded_rect(frame, 184, 200, 8, 44, 4, WHITE);
    } else {
        fill_play_triangle(frame, 183, 222, 44, WHITE);
    }
}

fn draw_music_progress_row(frame: &mut [u16], progress: u8, elapsed: &str, duration: &str) {
    draw_text_centered_at(frame, 77, 288, elapsed, ACCENT_MUSIC_BLUE, 2);
    draw_text_centered_at(frame, 285, 288, duration, WHITE, 2);

    let track_x = 112;
    let track_y = 281;
    let track_w = 136;
    fill_rounded_rect(frame, track_x, track_y, track_w, 7, 3, RING_DIM);

    let fill_w = ((track_w as u16 * progress.min(100) as u16) / 100) as i32;
    fill_rounded_rect(frame, track_x, track_y, fill_w.max(6), 7, 3, ACCENT_MUSIC_BLUE);
    fill_circle(frame, track_x + fill_w.max(6), track_y + 3, 7, ACCENT_MUSIC_BLUE);
}


fn draw_ai_assistant_tile(model: &OnboardModel, frame: &mut [u16]) {
    blit_rgb565_asset(frame, ASSISTANT_BASE_RGB565);

    // v0.1.12 AI Assistant Option B Conversation Card.
    // Base asset provides the clean card shell, lower control layout, and subtle screen frame.
    // Runtime overlays keep listening state, message, timestamp, and mic state live.
    draw_waveform(
        frame,
        180,
        70,
        if model.assistant.listening { 22 } else { 14 },
        ACCENT_ASSISTANT,
    );
    draw_text_centered_at(frame, 180, 112, model.assistant.title_label(), WHITE, 3);
    draw_text_centered_at(frame, 180, 140, model.assistant.subtitle_label(), MUTED, 2);

    draw_assistant_robot_badge(frame, 91, 190, model.assistant.listening);
    draw_text(frame, 126, 178, model.assistant.card_label(), WHITE, 2);
    draw_text(frame, 126, 204, model.assistant.card_aux_label(), MUTED, 1);

    draw_microphone_button(frame, 180, 272, model.assistant.listening);
    draw_cancel_glyph(frame, 116, 272, MUTED);
}


fn draw_assistant_robot_badge(frame: &mut [u16], cx: i32, cy: i32, listening: bool) {
    let outer = if listening { ACCENT_ASSISTANT } else { RING_DIM };
    fill_circle(frame, cx, cy, 25, 0x192A);
    stroke_circle(frame, cx, cy, 25, outer);

    fill_rounded_rect(frame, cx - 15, cy - 11, 30, 22, 8, BG_DARK);
    stroke_rect(frame, cx - 15, cy - 11, 30, 22, WHITE);
    fill_circle(frame, cx - 7, cy, 3, ACCENT_ASSISTANT);
    fill_circle(frame, cx + 7, cy, 3, ACCENT_ASSISTANT);
    draw_line(frame, cx, cy - 16, cx, cy - 12, WHITE);
    fill_circle(frame, cx, cy - 18, 2, WHITE);
}


fn draw_settings_tile(model: &OnboardModel, frame: &mut [u16]) {
    blit_rgb565_asset(frame, SETTINGS_BASE_RGB565);

    // v0.1.13 Settings Screen Option A List Style.
    // Overview is a clean list. Detail remains local-only and maps to current quiet-render setting.
    if !model.settings.detail_open {
        draw_text_centered_at(frame, 180, 32, &model.rtc_hms(), MUTED, 1);
        draw_text_centered_at(frame, 180, 76, "SETTINGS", ACCENT_SETTINGS, 2);

        draw_settings_list_row(frame, 55, 96, "WI-FI", SettingsIcon::Wifi, false);
        draw_settings_list_row(frame, 55, 146, "DISPLAY", SettingsIcon::Display, true);
        draw_settings_list_row(frame, 55, 196, "SOUND", SettingsIcon::Sound, false);
        draw_settings_list_row(frame, 55, 246, "ABOUT", SettingsIcon::About, false);
    } else {
        draw_settings_display_detail(frame, model);
    }
}


#[derive(Clone, Copy)]
enum SettingsIcon {
    Wifi,
    Display,
    Sound,
    About,
}

fn draw_settings_list_row(
    frame: &mut [u16],
    x: i32,
    y: i32,
    label: &str,
    icon: SettingsIcon,
    selected: bool,
) {
    let outline = if selected { ACCENT_SETTINGS } else { RING_DIM };
    let fill = if selected { 0x1096 } else { BG_DARK };

    fill_rounded_rect(frame, x, y, 250, 38, 16, fill);
    stroke_rounded_rect(frame, x, y, 250, 38, 16, outline);

    let icon_cx = x + 32;
    let icon_cy = y + 19;
    fill_circle(frame, icon_cx, icon_cy, 15, if selected { 0x18F8 } else { BG });
    stroke_circle(frame, icon_cx, icon_cy, 15, outline);
    draw_settings_row_icon(frame, icon, icon_cx, icon_cy, WHITE);

    draw_text(frame, x + 74, y + 26, label, WHITE, 2);
    draw_settings_chevron(frame, x + 226, y + 19, if selected { WHITE } else { MUTED });
}

fn draw_settings_row_icon(frame: &mut [u16], icon: SettingsIcon, cx: i32, cy: i32, color: u16) {
    match icon {
        SettingsIcon::Wifi => draw_wifi_icon(frame, cx, cy - 4, color),
        SettingsIcon::Display => draw_settings_sun_icon(frame, cx, cy, color),
        SettingsIcon::Sound => draw_settings_sound_icon(frame, cx, cy, color),
        SettingsIcon::About => draw_settings_info_icon(frame, cx, cy, color),
    }
}

fn draw_settings_chevron(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    draw_line(frame, cx - 4, cy - 8, cx + 4, cy, color);
    draw_line(frame, cx + 4, cy, cx - 4, cy + 8, color);
}

fn draw_settings_sun_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    stroke_circle(frame, cx, cy, 6, color);
    for angle in (0..360).step_by(45) {
        let rad = (angle as f32).to_radians();
        let x0 = cx + (rad.cos() * 9.0).round() as i32;
        let y0 = cy + (rad.sin() * 9.0).round() as i32;
        let x1 = cx + (rad.cos() * 13.0).round() as i32;
        let y1 = cy + (rad.sin() * 13.0).round() as i32;
        draw_line(frame, x0, y0, x1, y1, color);
    }
}

fn draw_settings_sound_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    fill_rect(frame, cx - 12, cy - 5, 5, 10, color);
    draw_line(frame, cx - 7, cy - 5, cx + 1, cy - 11, color);
    draw_line(frame, cx - 7, cy + 5, cx + 1, cy + 11, color);
    draw_line(frame, cx + 1, cy - 11, cx + 1, cy + 11, color);
    draw_arc_segment(frame, cx + 2, cy, 9, 1, 320, 40, color);
    draw_arc_segment(frame, cx + 2, cy, 14, 1, 320, 40, color);
}

fn draw_settings_info_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    stroke_circle(frame, cx, cy, 11, color);
    fill_circle(frame, cx, cy - 6, 2, color);
    fill_rect(frame, cx - 1, cy - 1, 3, 9, color);
}

fn draw_settings_display_detail(frame: &mut [u16], model: &OnboardModel) {
    draw_text_centered_at(frame, 180, 32, &model.rtc_hms(), MUTED, 1);
    draw_text_centered_at(frame, 180, 76, "DISPLAY", ACCENT_SETTINGS, 2);

    fill_circle(frame, 180, 126, 42, BG_DARK);
    stroke_circle(frame, 180, 126, 42, ACCENT_SETTINGS);
    draw_settings_sun_icon(frame, 180, 126, WHITE);

    draw_text_centered_at(frame, 180, 180, "BRIGHTNESS", WHITE, 2);
    fill_rounded_rect(frame, 92, 206, 150, 8, 4, RING_DIM);
    fill_rounded_rect(frame, 92, 206, 105, 8, 4, ACCENT_SETTINGS);
    fill_circle(frame, 197, 210, 8, WHITE);
    draw_text(frame, 254, 200, "70%", WHITE, 2);

    fill_rounded_rect(frame, 60, 246, 240, 42, 16, BG_DARK);
    stroke_rounded_rect(frame, 60, 246, 240, 42, 16, ACCENT_SETTINGS);
    draw_text(frame, 78, 271, "QUIET RENDER", WHITE, 1);
    draw_text(
        frame,
        220,
        271,
        if model.settings.quiet_render_enabled { "ON" } else { "OFF" },
        ACCENT_SETTINGS,
        1,
    );
    draw_settings_chevron(frame, 282, 266, WHITE);
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

fn draw_assistant_orb(frame: &mut [u16], cx: i32, cy: i32, r: i32, color: u16) {
    fill_circle(frame, cx, cy, r, BG_DARK);
    stroke_circle(frame, cx, cy, r, color);
    stroke_circle(frame, cx, cy, r - 8, SOFT);
    fill_circle(frame, cx, cy, 4, color);
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

#[derive(Clone, Copy)]
struct WeatherHourMock {
    hour: &'static str,
    temp: &'static str,
    icon: WeatherMiniIcon,
}

fn draw_weather_hour_values(frame: &mut [u16], condition: &str) {
    let entries = forecast_timeline_entries(condition);
    let centers = [74, 145, 216, 287];
    for (index, entry) in entries.iter().enumerate() {
        draw_weather_timeline_slot(frame, centers[index], *entry, index == 0);
    }
}

fn forecast_timeline_entries(condition: &str) -> [WeatherHourMock; 4] {
    match condition {
        "CLEAR" | "LOCAL" => [
            WeatherHourMock { hour: "11A", temp: "72", icon: WeatherMiniIcon::PartlyCloudy },
            WeatherHourMock { hour: "12P", temp: "73", icon: WeatherMiniIcon::Sun },
            WeatherHourMock { hour: "1P", temp: "72", icon: WeatherMiniIcon::Cloud },
            WeatherHourMock { hour: "2P", temp: "70", icon: WeatherMiniIcon::Wind },
        ],
        "SUNNY" => [
            WeatherHourMock { hour: "11A", temp: "75", icon: WeatherMiniIcon::Sun },
            WeatherHourMock { hour: "12P", temp: "77", icon: WeatherMiniIcon::Sun },
            WeatherHourMock { hour: "1P", temp: "79", icon: WeatherMiniIcon::PartlyCloudy },
            WeatherHourMock { hour: "2P", temp: "76", icon: WeatherMiniIcon::Wind },
        ],
        "CLOUDY" => [
            WeatherHourMock { hour: "11A", temp: "68", icon: WeatherMiniIcon::Cloud },
            WeatherHourMock { hour: "12P", temp: "68", icon: WeatherMiniIcon::Cloud },
            WeatherHourMock { hour: "1P", temp: "70", icon: WeatherMiniIcon::PartlyCloudy },
            WeatherHourMock { hour: "2P", temp: "67", icon: WeatherMiniIcon::Rain },
        ],
        "BREEZY" => [
            WeatherHourMock { hour: "11A", temp: "70", icon: WeatherMiniIcon::Wind },
            WeatherHourMock { hour: "12P", temp: "71", icon: WeatherMiniIcon::PartlyCloudy },
            WeatherHourMock { hour: "1P", temp: "69", icon: WeatherMiniIcon::Wind },
            WeatherHourMock { hour: "2P", temp: "68", icon: WeatherMiniIcon::Storm },
        ],
        _ => [
            WeatherHourMock { hour: "11A", temp: "72", icon: WeatherMiniIcon::PartlyCloudy },
            WeatherHourMock { hour: "12P", temp: "73", icon: WeatherMiniIcon::Sun },
            WeatherHourMock { hour: "1P", temp: "72", icon: WeatherMiniIcon::Cloud },
            WeatherHourMock { hour: "2P", temp: "70", icon: WeatherMiniIcon::Wind },
        ],
    }
}

fn draw_weather_timeline_slot(
    frame: &mut [u16],
    cx: i32,
    entry: WeatherHourMock,
    highlighted: bool,
) {
    // Large Option D slot contract: x=(cx-30..cx+30), y=176..278.
    // Larger text is allowed because hour, icon, and temperature stay in separate lanes.
    let hour_color = if highlighted { ACCENT_WEATHER } else { WHITE };
    draw_text_centered_at(frame, cx, 190, entry.hour, hour_color, 2);
    draw_weather_timeline_icon(frame, cx, 226, entry.icon);
    draw_text_centered_at(frame, cx, 262, entry.temp, WHITE, 2);
}

fn draw_tiny_temp_value(frame: &mut [u16], cx: i32, y: i32, text: &str, color: u16) {
    // Kept for future numeric experiments. Large-strip Option D uses scale-2
    // text because the cards are now tall enough for readable hourly temps.
    draw_text_centered_at(frame, cx, y, text, color, 2);
}

fn draw_weather_timeline_icon(frame: &mut [u16], cx: i32, cy: i32, icon: WeatherMiniIcon) {
    match icon {
        WeatherMiniIcon::Sun => draw_timeline_sun_icon(frame, cx, cy, ACCENT_WEATHER),
        WeatherMiniIcon::PartlyCloudy => draw_timeline_partly_cloudy_icon(frame, cx, cy),
        WeatherMiniIcon::Cloud => draw_timeline_cloud_icon(frame, cx, cy, WHITE),
        WeatherMiniIcon::Rain => draw_timeline_rain_icon(frame, cx, cy),
        WeatherMiniIcon::Storm => draw_timeline_storm_icon(frame, cx, cy),
        WeatherMiniIcon::Wind => draw_timeline_wind_icon(frame, cx, cy, WHITE),
    }
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
        let y0 = cy + if i % 2 == 0 { -heights[i] / 2 } else { heights[i] / 2 };
        let x1 = cx + xs[i + 1];
        let y1 = cy + if (i + 1) % 2 == 0 { -heights[i + 1] / 2 } else { heights[i + 1] / 2 };
        draw_line(frame, x0, y0, x1, y1, color);
    }
    fill_circle(frame, cx, cy, 4, WHITE);
}

fn draw_microphone_button(frame: &mut [u16], cx: i32, cy: i32, listening: bool) {
    let color = if listening { ACCENT_ASSISTANT } else { ACCENT_ASSISTANT_BLUE };
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
    draw_ring_meter(frame, cx, cy, 54, 3, if on { 82 } else { 25 }, RING_DIM, color);
    fill_circle(frame, cx, cy, 39, BG_DARK);
    fill_circle(frame, cx, if on { cy - 54 } else { cy + 54 }, 3, color);
}

fn draw_settings_row(frame: &mut [u16], y: i32, title: &str, icon: u8) {
    let icon_color = match icon {
        0 => MUTED,
        1 => ACCENT_SETTINGS,
        2 => SOFT,
        _ => RING,
    };
    fill_circle(frame, 90, y, 18, icon_color);
    match icon {
        0 => draw_sun_icon(frame, 90, y, 7, WHITE),
        1 => draw_wifi_icon(frame, 90, y, WHITE),
        2 => draw_bell_icon(frame, 90, y, WHITE),
        _ => draw_mini_gear(frame, 90, y, WHITE),
    }
    draw_text(frame, 124, y + 5, title, WHITE, 2);
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

fn draw_page_dots(model: &OnboardModel, frame: &mut [u16]) {
    let count = ALL_PAGES.len() as i32;
    let spacing = 12;
    let start_x = CX - ((count - 1) * spacing / 2);

    for (idx, page) in ALL_PAGES.iter().copied().enumerate() {
        let selected = page == model.current_page;
        let r = if selected { 3 } else { 2 };
        let color = if selected { accent_for_page(page) } else { SOFT };
        fill_circle(frame, start_x + idx as i32 * spacing, FOOTER_DOTS_Y, r, color);
    }
}

fn accent_for_page(page: AssistantPage) -> u16 {
    match page {
        AssistantPage::Home => ACCENT_HOME,
        AssistantPage::Weather => ACCENT_WEATHER,
        AssistantPage::Music => ACCENT_MUSIC,
        AssistantPage::Assistant => ACCENT_ASSISTANT,
        AssistantPage::Settings => ACCENT_SETTINGS,
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

fn draw_chip(frame: &mut [u16], x: i32, y: i32, w: i32, h: i32, label: &str, accent: u16, selected: bool) {
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

fn draw_ring_meter(frame: &mut [u16], cx: i32, cy: i32, r: i32, thickness: i32, progress: u8, base: u16, accent: u16) {
    draw_arc_segment(frame, cx, cy, r, thickness, 135, 45, base);
    let sweep = (progress.min(100) as i32 * 270) / 100;
    let end = (135 + sweep) % 360;
    draw_arc_segment(frame, cx, cy, r, thickness, 135, end, accent);
}

fn draw_arc_segment(frame: &mut [u16], cx: i32, cy: i32, r: i32, thickness: i32, start_deg: i32, end_deg: i32, color: u16) {
    let thickness = thickness.max(1);
    let half = thickness / 2;

    for offset in -half..=half {
        draw_arc_line(frame, cx, cy, r + offset, start_deg, end_deg, color);
    }
}

fn draw_arc_line(frame: &mut [u16], cx: i32, cy: i32, r: i32, start_deg: i32, end_deg: i32, color: u16) {
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
        let half = if y <= cy {
            y - top
        } else {
            bottom - y
        };
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
        let half = if y <= cy {
            y - top
        } else {
            bottom - y
        };
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

fn draw_numeric_value_centered(frame: &mut [u16], y: i32, text: &str, digit_w: i32, stroke: i32, color: u16) {
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

fn draw_numeric_char(frame: &mut [u16], x: i32, y: i32, ch: char, digit_w: i32, stroke: i32, color: u16) {
    match ch {
        '0' => draw_segment_mask(frame, x, y, digit_w, stroke, color, [true, true, true, true, true, true, false]),
        '1' => draw_segment_mask(frame, x, y, digit_w, stroke, color, [false, true, true, false, false, false, false]),
        '2' => draw_segment_mask(frame, x, y, digit_w, stroke, color, [true, true, false, true, true, false, true]),
        '3' => draw_segment_mask(frame, x, y, digit_w, stroke, color, [true, true, true, true, false, false, true]),
        '4' => draw_segment_mask(frame, x, y, digit_w, stroke, color, [false, true, true, false, false, true, true]),
        '5' => draw_segment_mask(frame, x, y, digit_w, stroke, color, [true, false, true, true, false, true, true]),
        '6' => draw_segment_mask(frame, x, y, digit_w, stroke, color, [true, false, true, true, true, true, true]),
        '7' => draw_segment_mask(frame, x, y, digit_w, stroke, color, [true, true, true, false, false, false, false]),
        '8' => draw_segment_mask(frame, x, y, digit_w, stroke, color, [true, true, true, true, true, true, true]),
        '9' => draw_segment_mask(frame, x, y, digit_w, stroke, color, [true, true, true, true, false, true, true]),
        ':' => {
            fill_circle(frame, x + stroke, y + digit_w / 2, stroke / 2 + 1, color);
            fill_circle(frame, x + stroke, y + digit_w + digit_w / 2, stroke / 2 + 1, color);
        }
        'F' | 'f' => draw_letter_f(frame, x, y, digit_w, stroke, color),
        '-' => fill_rounded_rect(frame, x, y + digit_w, digit_w / 2, stroke, stroke / 2, color),
        _ => {}
    }
}

fn draw_segment_mask(frame: &mut [u16], x: i32, y: i32, w: i32, s: i32, color: u16, seg: [bool; 7]) {
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
