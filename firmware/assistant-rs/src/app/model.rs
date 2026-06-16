use crate::{board, drivers::pcf85063::DateTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardId {
    Rtc,
    Touch,
    Flash,
    Psram,
    Battery,
    Wifi,
    Sd,
    Backlight,
}

#[derive(Debug, Clone, Copy)]
pub struct TouchSnapshot {
    pub x: u16,
    pub y: u16,
    pub fingers: u8,
}

#[derive(Debug, Clone)]
pub struct OnboardModel {
    pub rtc: Option<DateTime>,
    pub touch_events: u32,
    pub last_touch: Option<TouchSnapshot>,
    pub battery_mv: Option<u16>,
    pub sd_present: bool,
    pub sd_capacity_mb: Option<u32>,
    pub wifi_count: Option<u16>,
    pub flash_mib: u32,
    pub psram_mib: u32,
    pub backlight_percent: u8,
    pub i2c_ok: bool,
    pub touch_ready: bool,
    pub rtc_ready: bool,
    pub selected: Option<CardId>,
}

impl OnboardModel {
    pub fn new() -> Self {
        Self {
            rtc: None,
            touch_events: 0,
            last_touch: None,
            battery_mv: None,
            sd_present: false,
            sd_capacity_mb: None,
            wifi_count: None,
            flash_mib: board::BOARD_FLASH_MIB,
            psram_mib: board::BOARD_PSRAM_MIB,
            backlight_percent: 100,
            i2c_ok: false,
            touch_ready: false,
            rtc_ready: false,
            selected: Some(CardId::Rtc),
        }
    }

    pub fn set_probe_status(&mut self, i2c_ok: bool, touch_ready: bool, rtc_ready: bool) {
        self.i2c_ok = i2c_ok;
        self.touch_ready = touch_ready;
        self.rtc_ready = rtc_ready;
    }

    pub fn update_rtc(&mut self, dt: DateTime) {
        self.rtc = Some(dt);
        self.rtc_ready = true;
    }

    pub fn note_touch(&mut self, x: u16, y: u16, fingers: u8) {
        self.touch_events = self.touch_events.saturating_add(1);
        self.last_touch = Some(TouchSnapshot { x, y, fingers });
    }

    pub fn rtc_hms(&self) -> String {
        match self.rtc {
            Some(dt) => format!("{:02}:{:02}:{:02}", dt.hour, dt.minute, dt.second),
            None => "--:--:--".to_string(),
        }
    }

    pub fn rtc_ymd(&self) -> String {
        match self.rtc {
            Some(dt) => format!("{:04}-{:02}-{:02}", 2000 + dt.year as u16, dt.month, dt.day),
            None => "---- -- --".to_string(),
        }
    }

    pub fn touch_count_text(&self) -> String {
        self.touch_events.to_string()
    }

    pub fn battery_text(&self) -> String {
        match self.battery_mv {
            Some(mv) => format!("{}MV", mv),
            None => "--".to_string(),
        }
    }

    pub fn wifi_text(&self) -> String {
        match self.wifi_count {
            Some(count) => count.to_string(),
            None => "--".to_string(),
        }
    }

    pub fn sd_text(&self) -> String {
        if self.sd_present {
            match self.sd_capacity_mb {
                Some(mb) => format!("{}M", mb),
                None => "ON".to_string(),
            }
        } else {
            "NO".to_string()
        }
    }

    pub fn backlight_text(&self) -> String {
        format!("{}%", self.backlight_percent)
    }

    pub fn flash_text(&self) -> String {
        format!("{}M", self.flash_mib)
    }

    pub fn psram_text(&self) -> String {
        format!("{}M", self.psram_mib)
    }
}