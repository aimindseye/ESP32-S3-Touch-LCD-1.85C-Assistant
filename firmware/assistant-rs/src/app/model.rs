use crate::{
    app::{
        assistant::AssistantState,
        home::HomeState,
        music::MusicState,
        pages::AssistantPage,
        settings::SettingsState,
        weather::WeatherState,
    },
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
    pub wifi_connected: bool,
    pub wifi_ssid: Option<String>,
    pub flash_mib: u32,
    pub psram_mib: u32,
    pub backlight_percent: u8,
    pub i2c_ok: bool,
    pub touch_ready: bool,
    pub rtc_ready: bool,
    pub home: HomeState,
    pub weather: WeatherState,
    pub music: MusicState,
    pub assistant: AssistantState,
    pub settings: SettingsState,
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
            wifi_connected: false,
            wifi_ssid: None,
            flash_mib: board::BOARD_FLASH_MIB,
            psram_mib: board::BOARD_PSRAM_MIB,
            backlight_percent: 100,
            i2c_ok: false,
            touch_ready: false,
            rtc_ready: false,
            home: HomeState::new(),
            weather: WeatherState::new(),
            music: MusicState::new(),
            assistant: AssistantState::new(),
            settings: SettingsState::new(),
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

    pub fn update_wifi_status(
        &mut self,
        count: Option<u16>,
        connected: bool,
        ssid: Option<String>,
    ) {
        self.wifi_count = count;
        self.wifi_connected = connected;
        self.wifi_ssid = ssid;
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

    pub fn battery_percent_value(&self) -> Option<u8> {
        let mv = self.battery_mv?;
        if mv < 3200 {
            None
        } else {
            let pct = ((mv.saturating_sub(3300) as u32) * 100 / 900).min(100);
            Some(pct as u8)
        }
    }

    pub fn battery_home_text(&self) -> String {
        match self.battery_percent_value() {
            Some(pct) => format!("{}%", pct),
            None => "USB".to_string(),
        }
    }

    pub fn wifi_text(&self) -> String {
        match self.wifi_count {
            Some(count) => count.to_string(),
            None => "--".to_string(),
        }
    }

    pub fn wifi_home_text(&self) -> String {
        if self.wifi_connected {
            if let Some(ssid) = self.wifi_ssid.as_ref() {
                let clipped = clipped_upper_ascii(ssid, 8);
                if !clipped.is_empty() {
                    return clipped;
                }
            }
            "WIFI".to_string()
        } else if self.wifi_count.is_some() {
            "NO WIFI".to_string()
        } else {
            "OFF".to_string()
        }
    }

    pub fn rtc_day_date(&self) -> String {
        match self.rtc {
            Some(dt) => {
                let year = 2000 + dt.year as i32;
                format!("{} {:02}", weekday3(year, dt.month, dt.day), dt.day)
            }
            None => "--- --".to_string(),
        }
    }

    pub fn rtc_home_date_text(&self) -> String {
        match self.rtc {
            Some(dt) => {
                let year = 2000 + dt.year as i32;
                format!(
                    "{} {:02} {}",
                    weekday3(year, dt.month, dt.day),
                    dt.day,
                    month3(dt.month)
                )
            }
            None => "--- -- ---".to_string(),
        }
    }

    pub fn home_weather_condition(&self) -> &'static str {
        let condition = self.weather.condition_label;
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
        } else {
            "LOCAL"
        }
    }

    pub fn home_weather_temp(&self) -> &'static str {
        if self.weather.temperature_label == "--" {
            "72F"
        } else {
            self.weather.temperature_label
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


fn clipped_upper_ascii(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if out.chars().count() >= max_chars {
            break;
        }
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '.' {
            out.push(ch.to_ascii_uppercase());
        }
    }
    out
}

fn weekday3(year: i32, month: u8, day: u8) -> &'static str {
    let offsets = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut y = year;
    let m = month.clamp(1, 12) as usize;
    if m < 3 {
        y -= 1;
    }
    let weekday = (y + y / 4 - y / 100 + y / 400 + offsets[m - 1] + day as i32).rem_euclid(7);
    match weekday {
        0 => "SUN",
        1 => "MON",
        2 => "TUE",
        3 => "WED",
        4 => "THU",
        5 => "FRI",
        _ => "SAT",
    }
}


fn month3(month: u8) -> &'static str {
    match month.clamp(1, 12) {
        1 => "JAN",
        2 => "FEB",
        3 => "MAR",
        4 => "APR",
        5 => "MAY",
        6 => "JUN",
        7 => "JUL",
        8 => "AUG",
        9 => "SEP",
        10 => "OCT",
        11 => "NOV",
        _ => "DEC",
    }
}
