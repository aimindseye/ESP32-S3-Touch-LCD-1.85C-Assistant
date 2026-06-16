mod app;
mod board;
mod drivers;
mod ffi;

use anyhow::{bail, Result};
use app::{
    model::{CardId, OnboardModel},
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

// Full-screen render. Let the physical bezel crop the corners.
const BG: u16 = 0x0841;
const BLACK: u16 = 0x0000;
const WHITE: u16 = 0xFFFF;
const SURFACE: u16 = 0x2104;
const SURFACE_2: u16 = 0x18C3;
const PANEL_BG: u16 = 0x2945;

const ACCENT_HOME: u16 = 0x07FF;
const ACCENT_WEATHER: u16 = 0x07E0;
const ACCENT_MUSIC: u16 = 0x4A69;
const ACCENT_SETTINGS: u16 = 0xFFE0;

const CARD_RTC: u16 = 0x07E0;
const CARD_TOUCH: u16 = 0xF81F;
const CARD_FLASH: u16 = 0xFD20;
const CARD_PSRAM: u16 = 0xA53F;
const CARD_BAT: u16 = 0xFFE0;
const CARD_WIFI: u16 = 0x07FF;
const CARD_SD: u16 = 0xC618;
const CARD_BL: u16 = 0x2D7F;

const STATUS_OK: u16 = 0x07E0;
const STATUS_BAD: u16 = 0xF800;

// Centered safe-content rectangle inside round display.
const PANEL_X: i32 = 34;
const PANEL_Y: i32 = 48;
const PANEL_W: i32 = 292;
const PANEL_H: i32 = 252;

// Header lives inside panel.
const HEADER_X: i32 = PANEL_X + 8;
const HEADER_Y: i32 = PANEL_Y + 10;
const HEADER_H: i32 = 22;
const TAB_W: i32 = 65;
const TAB_GAP: i32 = 4;

// Grid lives inside panel.
const GRID_X: i32 = PANEL_X + 12;
const GRID_Y: i32 = PANEL_Y + 44;
const GAP_X: i32 = 10;
const ROW_GAP: i32 = 8;
const CARD_W: i32 = 129;
const CARD_H: i32 = 40;

// Footer lives inside panel.
const FOOTER_X: i32 = PANEL_X + 22;
const FOOTER_Y: i32 = PANEL_Y + 236;
const FOOTER_W: i32 = PANEL_W - 44;
const FOOTER_H: i32 = 12;

const BAT_ATTEN: adc_atten_t = attenuation::DB_11;

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
    println!("Assistant app shell with Home/Weather/Music/Settings pages");
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

    draw_assistant_page(&model, frame.as_mut_slice())?;
    println!("home page rendered");

    let mut touch_latched = false;
    let mut last_fast = Instant::now();
    let mut last_wifi = Instant::now();
    let mut last_sd = Instant::now();

    loop {
        if touch_int.is_low() {
            if !touch_latched {
                touch_latched = true;

                if let Ok(point) = touch.read_touch(&mut i2c, board::CST816_ADDR) {
                    if point.fingers > 0 {
                        println!(
                            "touch: gesture=0x{:02X} fingers={} x={} y={}",
                            point.gesture, point.fingers, point.x, point.y
                        );

                        model.note_touch(point.x, point.y, point.fingers);

                        if let Some(page) = page_from_point(point.x as i32, point.y as i32) {
                            model.set_page(page);
                            println!("page: {:?}", page);
                        } else if model.current_page == AssistantPage::Home {
                            model.selected = card_from_point(point.x as i32, point.y as i32);
                        }

                        draw_assistant_page(&model, frame.as_mut_slice())?;
                        println!("repaint ok");
                    }
                }
            }
        } else {
            touch_latched = false;
        }

        if last_fast.elapsed() >= Duration::from_secs(1) {
            last_fast = Instant::now();

            if let Ok(dt) = rtc.read_datetime(&mut i2c, board::PCF85063_ADDR) {
                model.update_rtc(dt);

                println!(
                    "diag: rtc={:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                    2000 + dt.year as u16,
                    dt.month,
                    dt.day,
                    dt.hour,
                    dt.minute,
                    dt.second
                );
            }

            if let Ok(raw) = adc.read(&mut bat_pin) {
                let adc_mv = (raw as f32 / 4095.0) * 3300.0;
                let battery_mv = ((adc_mv * board::BATTERY_DIVIDER_SCALE)
                    / board::BATTERY_MEASUREMENT_OFFSET) as u16;
                model.battery_mv = Some(battery_mv);
            }

            draw_assistant_page(&model, frame.as_mut_slice())?;
        }

        if last_wifi.elapsed() >= Duration::from_secs(30) {
            last_wifi = Instant::now();
            refresh_wifi(&mut model, &mut wifi);
            draw_assistant_page(&model, frame.as_mut_slice())?;
        }

        if last_sd.elapsed() >= Duration::from_secs(30) {
            last_sd = Instant::now();
            refresh_sd(&mut model);
            draw_assistant_page(&model, frame.as_mut_slice())?;
        }

        thread::sleep(Duration::from_millis(50));
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

fn draw_assistant_page(model: &OnboardModel, frame: &mut [u16]) -> Result<()> {
    frame.fill(BG);

    fill_rect(frame, PANEL_X, PANEL_Y, PANEL_W, PANEL_H, PANEL_BG);
    stroke_rect(frame, PANEL_X, PANEL_Y, PANEL_W, PANEL_H, SURFACE_2);

    draw_page_tabs(model, frame);

    match model.current_page {
        AssistantPage::Home => draw_home_content(model, frame),
        AssistantPage::Weather => draw_placeholder_page(model, frame, AssistantPage::Weather),
        AssistantPage::Music => draw_placeholder_page(model, frame, AssistantPage::Music),
        AssistantPage::Settings => draw_placeholder_page(model, frame, AssistantPage::Settings),
    }

    draw_status_footer(model, frame);

    if !unsafe { ffi::st77916_panel_draw_rgb565(0, 0, 359, 359, frame.as_mut_ptr()) } {
        bail!("st77916_panel_draw_rgb565 returned false");
    }

    Ok(())
}

fn draw_page_tabs(model: &OnboardModel, frame: &mut [u16]) {
    for (idx, page) in ALL_PAGES.iter().copied().enumerate() {
        let x = HEADER_X + (idx as i32 * (TAB_W + TAB_GAP));
        let selected = page == model.current_page;
        let accent = accent_for_page(page);
        let fill = if selected { accent } else { SURFACE };
        let text = if selected { BLACK } else { WHITE };

        fill_rect(frame, x, HEADER_Y, TAB_W, HEADER_H, fill);
        stroke_rect(frame, x, HEADER_Y, TAB_W, HEADER_H, SURFACE_2);
        draw_text(frame, x + 5, HEADER_Y + 15, page.short_label(), text, 1);
    }
}

fn draw_home_content(model: &OnboardModel, frame: &mut [u16]) {
    let rtc_sub = Some(model.rtc_ymd());
    let touch_sub = model
        .last_touch
        .map(|p| format!("{:03},{:03}/{}", p.x, p.y, p.fingers));

    let cards = [
        (
            CardId::Rtc,
            GRID_X,
            GRID_Y,
            CARD_RTC,
            "RTC",
            model.rtc_hms(),
            rtc_sub,
        ),
        (
            CardId::Touch,
            GRID_X + CARD_W + GAP_X,
            GRID_Y,
            CARD_TOUCH,
            "TOUCH",
            model.touch_count_text(),
            touch_sub,
        ),
        (
            CardId::Flash,
            GRID_X,
            GRID_Y + (CARD_H + ROW_GAP),
            CARD_FLASH,
            "FLASH",
            model.flash_text(),
            None,
        ),
        (
            CardId::Psram,
            GRID_X + CARD_W + GAP_X,
            GRID_Y + (CARD_H + ROW_GAP),
            CARD_PSRAM,
            "PSRAM",
            model.psram_text(),
            None,
        ),
        (
            CardId::Battery,
            GRID_X,
            GRID_Y + 2 * (CARD_H + ROW_GAP),
            CARD_BAT,
            "BAT",
            model.battery_text(),
            None,
        ),
        (
            CardId::Wifi,
            GRID_X + CARD_W + GAP_X,
            GRID_Y + 2 * (CARD_H + ROW_GAP),
            CARD_WIFI,
            "WIFI",
            model.wifi_text(),
            None,
        ),
        (
            CardId::Sd,
            GRID_X,
            GRID_Y + 3 * (CARD_H + ROW_GAP),
            CARD_SD,
            "SD",
            model.sd_text(),
            None,
        ),
        (
            CardId::Backlight,
            GRID_X + CARD_W + GAP_X,
            GRID_Y + 3 * (CARD_H + ROW_GAP),
            CARD_BL,
            "BL",
            model.backlight_text(),
            None,
        ),
    ];

    for (id, x, y, accent, title, value, sub) in cards {
        draw_card(
            frame,
            x,
            y,
            CARD_W,
            CARD_H,
            title,
            &value,
            sub.as_deref(),
            accent,
            model.selected == Some(id),
        );
    }
}

fn draw_placeholder_page(model: &OnboardModel, frame: &mut [u16], page: AssistantPage) {
    let accent = accent_for_page(page);
    let title = page.title();

    fill_rect(frame, GRID_X, GRID_Y, CARD_W * 2 + GAP_X, 168, SURFACE);
    fill_rect(frame, GRID_X, GRID_Y, CARD_W * 2 + GAP_X, 4, accent);
    stroke_rect(frame, GRID_X, GRID_Y, CARD_W * 2 + GAP_X, 168, SURFACE_2);

    draw_text(frame, GRID_X + 14, GRID_Y + 28, title, WHITE, 1);
    draw_text(frame, GRID_X + 14, GRID_Y + 50, "PLACEHOLDER", WHITE, 1);
    draw_text(frame, GRID_X + 14, GRID_Y + 72, "V0.1.2 SHELL", WHITE, 1);

    match page {
        AssistantPage::Weather => {
            draw_text(frame, GRID_X + 14, GRID_Y + 98, "LOCAL FORECAST", WHITE, 1);
            draw_text(frame, GRID_X + 14, GRID_Y + 116, "DATA LATER", WHITE, 1);
        }
        AssistantPage::Music => {
            draw_text(frame, GRID_X + 14, GRID_Y + 98, "SD AUDIO", WHITE, 1);
            draw_text(frame, GRID_X + 14, GRID_Y + 116, "PLAYER LATER", WHITE, 1);
        }
        AssistantPage::Settings => {
            draw_text(frame, GRID_X + 14, GRID_Y + 98, "DEVICE OPTIONS", WHITE, 1);
            draw_text(frame, GRID_X + 14, GRID_Y + 116, "MENU LATER", WHITE, 1);
        }
        AssistantPage::Home => {}
    }

    draw_text(frame, GRID_X + 14, GRID_Y + 144, &format!("RTC {}", model.rtc_hms()), WHITE, 1);
    draw_text(
        frame,
        GRID_X + 140,
        GRID_Y + 144,
        &format!("TOUCH {}", model.touch_count_text()),
        WHITE,
        1,
    );
}

fn draw_status_footer(model: &OnboardModel, frame: &mut [u16]) {
    fill_rect(frame, FOOTER_X, FOOTER_Y, FOOTER_W, FOOTER_H, SURFACE_2);
    draw_status_badge(frame, FOOTER_X + 10, FOOTER_Y + 3, "I2C", model.i2c_ok);
    draw_status_badge(frame, FOOTER_X + 90, FOOTER_Y + 3, "RTC", model.rtc_ready);
    draw_status_badge(
        frame,
        FOOTER_X + 170,
        FOOTER_Y + 3,
        "TOUCH",
        model.touch_ready,
    );
}

fn accent_for_page(page: AssistantPage) -> u16 {
    match page {
        AssistantPage::Home => ACCENT_HOME,
        AssistantPage::Weather => ACCENT_WEATHER,
        AssistantPage::Music => ACCENT_MUSIC,
        AssistantPage::Settings => ACCENT_SETTINGS,
    }
}

fn draw_card(
    frame: &mut [u16],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    title: &str,
    value: &str,
    sub: Option<&str>,
    accent: u16,
    selected: bool,
) {
    fill_rect(frame, x, y, w, h, SURFACE);
    fill_rect(frame, x, y, w, 3, accent);

    let border = if selected { WHITE } else { SURFACE_2 };
    stroke_rect(frame, x, y, w, h, border);

    draw_text(frame, x + 5, y + 9, title, WHITE, 1);
    draw_text(frame, x + 5, y + 19, value, WHITE, 1);

    if let Some(subline) = sub {
        draw_text(frame, x + 5, y + 29, subline, WHITE, 1);
    }
}

fn draw_status_badge(frame: &mut [u16], x: i32, y: i32, label: &str, ok: bool) {
    let color = if ok { STATUS_OK } else { STATUS_BAD };

    fill_rect(frame, x, y, 6, 6, color);
    stroke_rect(frame, x, y, 6, 6, WHITE);
    draw_text(frame, x + 9, y + 6, label, WHITE, 1);
}

fn page_from_point(x: i32, y: i32) -> Option<AssistantPage> {
    if !(HEADER_Y..HEADER_Y + HEADER_H).contains(&y) {
        return None;
    }

    for (idx, page) in ALL_PAGES.iter().copied().enumerate() {
        let tab_x = HEADER_X + (idx as i32 * (TAB_W + TAB_GAP));
        if x >= tab_x && x < tab_x + TAB_W {
            return Some(page);
        }
    }

    None
}

fn card_from_point(x: i32, y: i32) -> Option<CardId> {
    let rects = [
        (CardId::Rtc, GRID_X, GRID_Y),
        (CardId::Touch, GRID_X + CARD_W + GAP_X, GRID_Y),
        (CardId::Flash, GRID_X, GRID_Y + (CARD_H + ROW_GAP)),
        (
            CardId::Psram,
            GRID_X + CARD_W + GAP_X,
            GRID_Y + (CARD_H + ROW_GAP),
        ),
        (CardId::Battery, GRID_X, GRID_Y + 2 * (CARD_H + ROW_GAP)),
        (
            CardId::Wifi,
            GRID_X + CARD_W + GAP_X,
            GRID_Y + 2 * (CARD_H + ROW_GAP),
        ),
        (CardId::Sd, GRID_X, GRID_Y + 3 * (CARD_H + ROW_GAP)),
        (
            CardId::Backlight,
            GRID_X + CARD_W + GAP_X,
            GRID_Y + 3 * (CARD_H + ROW_GAP),
        ),
    ];

    for (id, rx, ry) in rects {
        if x >= rx && x < rx + CARD_W && y >= ry && y < ry + CARD_H {
            return Some(id);
        }
    }

    None
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

fn inside_circle(_x: i32, _y: i32) -> bool {
    true
}

#[allow(dead_code)]
fn fill_circle_bg(frame: &mut [u16], color: u16) {
    frame.fill(color);
}

fn fill_rect(frame: &mut [u16], x: i32, y: i32, w: i32, h: i32, color: u16) {
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + w).min(W as i32);
    let y1 = (y + h).min(H as i32);

    for yy in y0..y1 {
        let row = yy as usize * W;

        for xx in x0..x1 {
            if inside_circle(xx, yy) {
                frame[row + xx as usize] = color;
            }
        }
    }
}

fn stroke_rect(frame: &mut [u16], x: i32, y: i32, w: i32, h: i32, color: u16) {
    fill_rect(frame, x, y, w, 1, color);
    fill_rect(frame, x, y + h - 1, w, 1, color);
    fill_rect(frame, x, y, 1, h, color);
    fill_rect(frame, x + w - 1, y, 1, h, color);
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
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        _ => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    }
}
