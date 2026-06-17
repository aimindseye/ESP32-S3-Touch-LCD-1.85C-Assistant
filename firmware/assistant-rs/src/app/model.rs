use crate::{
    app::pages::AssistantPage,
    board,
    drivers::pcf85063::DateTime,
};

#[derive(Debug, Clone, Copy)]
pub struct TouchSnapshot {
    pub x: u16,
    pub y: u16,
    pub fingers: u8,
    pub gesture: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonName {
    Boot,
    Power,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonPressKind {
    Short,
    Long,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiIntent {
    NextPage,
    PreviousPage,
    Select,
    BackHome,
    AssistantHold,
    PowerMenu,
    BootReserved,
}

#[derive(Debug, Clone)]
pub struct OnboardModel {
    pub current_page: AssistantPage,
    pub rtc: Option<DateTime>,
    pub touch_events: u32,
    pub button_events: u32,
    pub nav_events: u32,
    pub last_touch: Option<TouchSnapshot>,
    pub last_action: &'static str,
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
}

impl OnboardModel {
    pub fn new() -> Self {
        Self {
            current_page: AssistantPage::Home,
            rtc: None,
            touch_events: 0,
            button_events: 0,
            nav_events: 0,
            last_touch: None,
            last_action: "READY",
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
        }
    }

    pub fn set_page(&mut self, page: AssistantPage) {
        if self.current_page != page {
            self.nav_events = self.nav_events.saturating_add(1);
        }
        self.current_page = page;
    }

    pub fn next_page(&mut self) {
        let next = self.current_page.next();
        self.set_page(next);
        self.last_action = "NEXT PAGE";
    }

    pub fn previous_page(&mut self) {
        let previous = self.current_page.previous();
        self.set_page(previous);
        self.last_action = "PREV PAGE";
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

    pub fn note_touch(&mut self, x: u16, y: u16, fingers: u8, gesture: u8) {
        self.touch_events = self.touch_events.saturating_add(1);
        self.last_touch = Some(TouchSnapshot {
            x,
            y,
            fingers,
            gesture,
        });
    }

    pub fn note_button(&mut self, button: ButtonName, kind: ButtonPressKind) {
        self.button_events = self.button_events.saturating_add(1);
        self.last_action = match (button, kind) {
            (ButtonName::Boot, ButtonPressKind::Short) => "BOOT RSV",
            (ButtonName::Boot, ButtonPressKind::Long) => "BOOT RSV",
            (ButtonName::Power, ButtonPressKind::Short) => "POWER HOME",
            (ButtonName::Power, ButtonPressKind::Long) => "POWER MENU",
        };
    }

    pub fn note_intent(&mut self, intent: UiIntent) {
        self.last_action = match intent {
            UiIntent::NextPage => "NEXT PAGE",
            UiIntent::PreviousPage => "PREV PAGE",
            UiIntent::Select => "SELECT",
            UiIntent::BackHome => "HOME",
            UiIntent::AssistantHold => "ASSIST HOLD",
            UiIntent::PowerMenu => "POWER MENU",
            UiIntent::BootReserved => "BOOT RSV",
        };
    }

    pub fn rtc_hms(&self) -> String {
        match self.rtc {
            Some(dt) => format!("{:02}:{:02}", dt.hour, dt.minute),
            None => "--:--".to_string(),
        }
    }

    pub fn rtc_hms_full(&self) -> String {
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

    pub fn button_count_text(&self) -> String {
        self.button_events.to_string()
    }

    pub fn nav_count_text(&self) -> String {
        self.nav_events.to_string()
    }

    pub fn battery_text(&self) -> String {
        match self.battery_mv {
            Some(mv) => format!("{:.2}V", mv as f32 / 1000.0),
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
                Some(mb) if mb >= 1024 => format!("{}G", mb / 1024),
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
