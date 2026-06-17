mod app;
mod board;
mod drivers;
mod ffi;

use anyhow::{bail, Result};
use app::{
    model::{ButtonName, ButtonPressKind, OnboardModel, UiIntent},
    pages::{AssistantPage, ALL_PAGES},
};
use core::{ffi::c_void, mem::size_of, ptr::NonNull, slice};
use drivers::{cst816::Cst816, pcf85063::Pcf85063, tca9554::Tca9554};
use embedded_svc::wifi::{ClientConfiguration, Configuration};
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
const CX: i32 = 180;
const CY: i32 = 180;
const R_OUTER: i32 = 178;

const BLACK: u16 = 0x0000;
const BG: u16 = 0x0841;
const RING: u16 = 0x18C3;
const WHITE: u16 = 0xFFFF;
const MUTED: u16 = 0xA514;

const ACCENT_HOME: u16 = 0x07FF;
const ACCENT_WEATHER: u16 = 0x07E0;
const ACCENT_MUSIC: u16 = 0x4A69;
const ACCENT_SETTINGS: u16 = 0xFFE0;
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
    println!("Active CST816 Polling and Gesture-First Navigation");
    println!("r12 touch contract: INT starts tracking, active poll gathers samples");
    println!("r3a build fix: last_render initialized before first render");
    println!("ADC battery read: inline accepted baseline path");
    println!("Keep v0.1.3 circular UI direction");
    println!("Watch UI polish: no divider lines, dim one-pixel outer ring");
    println!("touch-loop: poll={}ms", TOUCH_POLL_MS);
    println!("button-loop: poll={}ms", BUTTON_POLL_MS);
    println!("Touch classifier: gesture-first, span swipe dx>=20, active poll=8ms window=180ms");
    println!("Touch navigation cooldown={}ms", TOUCH_NAV_COOLDOWN_MS);
    println!("BOOT runtime control reserved while USB monitor is attached");
    println!("POWER candidate logging: GPIO6 experimental home/menu");
    println!("gpio-status: initialized once");
    println!("gpio-status: periodic reconfigure disabled");
    println!("Home page preserves PSRAM framebuffer + real BAT/WIFI/SD behavior");
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
                model.update_rtc(dt);
                if model.current_page == AssistantPage::Home {
                    dirty = true;
                }
            }
        }

        if !touch_tracker.active && last_battery.elapsed() >= Duration::from_millis(BATTERY_REFRESH_MS) {
            last_battery = now;
            if let Ok(raw) = adc.read(&mut bat_pin) {
                let adc_mv = (raw as f32 / 4095.0) * 3300.0;
                let battery_mv = ((adc_mv * board::BATTERY_DIVIDER_SCALE)
                    / board::BATTERY_MEASUREMENT_OFFSET) as u16;
                model.battery_mv = Some(battery_mv);
            }
            if model.current_page == AssistantPage::Home {
                dirty = true;
            }
        }

        if !touch_tracker.active && last_wifi.elapsed() >= Duration::from_millis(WIFI_REFRESH_MS) {
            last_wifi = now;
            refresh_wifi(&mut model, &mut wifi);
            if model.current_page == AssistantPage::Home {
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
    model.wifi_count = match wifi.scan() {
        Ok(aps) => Some(aps.len() as u16),
        Err(_) => None,
    };
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
            println!("action: Select on {:?}", model.current_page);
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
    stroke_circle(frame, CX, CY, R_OUTER, RING);

    draw_top_status(model, frame);
    draw_page_title(model.current_page, frame);

    match model.current_page {
        AssistantPage::Home => draw_home_tile(model, frame),
        AssistantPage::Weather => draw_weather_tile(model, frame),
        AssistantPage::Music => draw_music_tile(model, frame),
        AssistantPage::Settings => draw_settings_tile(model, frame),
    }

    draw_gesture_hint(frame);
    draw_page_dots(model, frame);

    if !unsafe { ffi::st77916_panel_draw_rgb565(0, 0, 359, 359, frame.as_mut_ptr()) } {
        bail!("st77916_panel_draw_rgb565 returned false");
    }

    Ok(())
}


fn draw_top_status(model: &OnboardModel, frame: &mut [u16]) {
    draw_status_dot(frame, 116, TOP_STATUS_Y, model.wifi_count.is_some(), "W");
    draw_status_dot(frame, 180, TOP_STATUS_Y, model.sd_present, "S");
    draw_status_dot(frame, 244, TOP_STATUS_Y, model.battery_mv.is_some(), "B");
}

fn draw_page_title(page: AssistantPage, frame: &mut [u16]) {
    draw_text_centered(frame, TITLE_Y, page.title(), accent_for_page(page), 2);
}

fn draw_home_tile(model: &OnboardModel, frame: &mut [u16]) {
    draw_text_centered(frame, 150, &model.rtc_hms(), WHITE, 5);
    draw_text_centered(frame, 198, "ASSISTANT READY", WHITE, 1);
    draw_text_centered(frame, 220, model.last_action, ACCENT_HOME, 2);
    draw_text_centered(frame, 266, "CENTER ASK", MUTED, 1);
    draw_text_centered(frame, 286, "POWER HOME", MUTED, 1);
}

fn draw_weather_tile(model: &OnboardModel, frame: &mut [u16]) {
    draw_text_centered(frame, 150, "--", WHITE, 5);
    draw_text_centered(frame, 198, "NO LOCATION", WHITE, 1);
    draw_text_centered(frame, 220, "DATA LATER", ACCENT_WEATHER, 1);
    draw_text_centered(frame, 266, "CENTER DETAILS", MUTED, 1);
    draw_text_centered(frame, 286, &format!("RTC {}", model.rtc_hms()), MUTED, 1);
}

fn draw_music_tile(_model: &OnboardModel, frame: &mut [u16]) {
    draw_text_centered(frame, 150, "NO TRACK", WHITE, 3);
    draw_text_centered(frame, 198, "PLAYER LATER", WHITE, 1);
    draw_text_centered(frame, 220, "SD AUDIO", ACCENT_MUSIC, 1);
    draw_text_centered(frame, 266, "CENTER PLAY", MUTED, 1);
    draw_text_centered(frame, 286, "RIGHT NEXT", MUTED, 1);
}

fn draw_settings_tile(_model: &OnboardModel, frame: &mut [u16]) {
    draw_text_centered(frame, SUBTITLE_Y, "SETTINGS", MUTED, 1);
    draw_menu_item(frame, 146, "SYSTEM INFO", ACCENT_SETTINGS, true);
    draw_menu_item(frame, 176, "BRIGHTNESS", WHITE, false);
    draw_menu_item(frame, 206, "WI-FI", WHITE, false);
    draw_menu_item(frame, 236, "ABOUT", WHITE, false);
    draw_text_centered(frame, 286, "CENTER OPEN", MUTED, 1);
}

fn draw_gesture_hint(frame: &mut [u16]) {
    draw_text_centered(frame, 306, "SWIPE ANYWHERE", MUTED, 1);
}

fn draw_menu_item(frame: &mut [u16], y: i32, text: &str, color: u16, selected: bool) {
    if selected {
        fill_circle(frame, 82, y - 4, 3, color);
    }
    draw_text_centered(frame, y, text, color, 1);
}

fn draw_status_dot(frame: &mut [u16], x: i32, y: i32, ok: bool, label: &str) {
    let color = if ok { STATUS_OK } else { STATUS_BAD };
    fill_circle(frame, x, y, 4, color);
    let label_x = x - text_width(label, 1) / 2;
    draw_text(frame, label_x, y + 14, label, MUTED, 1);
}

fn draw_page_dots(model: &OnboardModel, frame: &mut [u16]) {
    let count = ALL_PAGES.len() as i32;
    let spacing = 20;
    let start_x = CX - ((count - 1) * spacing / 2);

    for (idx, page) in ALL_PAGES.iter().copied().enumerate() {
        let selected = page == model.current_page;
        let r = if selected { 5 } else { 3 };
        let color = if selected { accent_for_page(page) } else { MUTED };
        fill_circle(frame, start_x + idx as i32 * spacing, FOOTER_DOTS_Y, r, color);
    }
}

fn accent_for_page(page: AssistantPage) -> u16 {
    match page {
        AssistantPage::Home => ACCENT_HOME,
        AssistantPage::Weather => ACCENT_WEATHER,
        AssistantPage::Music => ACCENT_MUSIC,
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
