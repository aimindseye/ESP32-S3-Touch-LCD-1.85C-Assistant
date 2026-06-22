use crate::{
    app::{
        assistant::AssistantState, home::HomeState, intents::UiIntent, music::MusicState,
        pages::AssistantPage, settings::SettingsState, time::TimeSyncState, weather::WeatherState,
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
    pub battery_adc_raw: Option<u16>,
    pub battery_adc_mv: Option<u16>,
    pub battery_adc_zero_count: u8,
    pub battery_adc_multiplier: f32,
    pub battery_empty_mv: u16,
    pub battery_full_mv: u16,
    pub battery_cal_status: &'static str,
    pub battery_source_label: &'static str,
    pub sd_present: bool,
    pub sd_capacity_mb: Option<u32>,
    pub sd_free_mb: Option<u32>,
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
    pub time: TimeSyncState,
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
            battery_adc_raw: None,
            battery_adc_mv: None,
            battery_adc_zero_count: 0,
            battery_adc_multiplier: board::BATTERY_DIVIDER_SCALE
                / board::BATTERY_MEASUREMENT_OFFSET,
            battery_empty_mv: 3000,
            battery_full_mv: 4200,
            battery_cal_status: "DEFAULT",
            battery_source_label: board::BATTERY_SOURCE_UNAVAILABLE,
            sd_present: false,
            sd_capacity_mb: None,
            sd_free_mb: None,
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
            time: TimeSyncState::new(),
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
            UiIntent::SettingsBackToOverview => "SETTINGS BACK",
            UiIntent::SettingsNextDetail => "SETTINGS NEXT",
            UiIntent::SettingsPreviousDetail => "SETTINGS PREV",
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

    pub fn time_source_label(&self) -> &'static str {
        self.time.source_label()
    }

    pub fn time_sync_label(&self) -> &'static str {
        self.time.status_label()
    }

    pub fn time_error_label(&self) -> &'static str {
        self.time.last_error
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

    pub fn set_battery_calibration(
        &mut self,
        adc_multiplier: f32,
        empty_mv: u16,
        full_mv: u16,
        status: &'static str,
    ) {
        self.battery_adc_multiplier = adc_multiplier;
        self.battery_empty_mv = empty_mv;
        self.battery_full_mv = full_mv;
        self.battery_cal_status = status;

        if let Some(adc_mv) = self.battery_adc_mv {
            self.battery_mv = Some(((adc_mv as f32) * self.battery_adc_multiplier).round() as u16);
        }
    }

    pub fn battery_adc_multiplier(&self) -> f32 {
        self.battery_adc_multiplier
    }

    pub fn battery_cal_status_text(&self) -> &'static str {
        self.battery_cal_status
    }

    pub fn battery_cal_text(&self) -> String {
        format!(
            "CAL {} {:.2}X",
            self.battery_cal_status_text(),
            self.battery_adc_multiplier
        )
    }

    pub fn note_battery_sample(&mut self, raw: u16, adc_mv: u16, battery_mv: u16) {
        self.note_battery_sample_with_source(
            raw,
            adc_mv,
            battery_mv,
            board::BATTERY_SOURCE_RUST_DIAG,
        );
    }

    pub fn note_battery_sample_with_source(
        &mut self,
        raw: u16,
        adc_mv: u16,
        battery_mv: u16,
        source: &'static str,
    ) {
        self.battery_adc_zero_count = 0;
        self.battery_adc_raw = Some(raw);
        self.battery_adc_mv = Some(adc_mv);
        self.battery_mv = Some(battery_mv);
        self.battery_source_label = source;
    }

    pub fn note_battery_zero_sample(&mut self) -> u8 {
        self.battery_adc_zero_count = self.battery_adc_zero_count.saturating_add(1);
        self.battery_adc_zero_count
    }

    pub fn battery_has_valid_sample(&self) -> bool {
        self.battery_adc_raw.unwrap_or(0) > 0 && self.battery_adc_mv.unwrap_or(0) > 0
    }

    pub fn battery_text(&self) -> String {
        self.battery_home_text()
    }

    pub fn battery_voltage_text(&self) -> String {
        match self.battery_mv {
            Some(mv) => format!("{:.2}V", mv as f32 / 1000.0),
            None => "--".to_string(),
        }
    }

    pub fn battery_adc_text(&self) -> String {
        match (self.battery_adc_raw, self.battery_adc_mv) {
            (Some(raw), Some(adc_mv)) => format!("{}/{}mV", raw, adc_mv),
            (Some(raw), None) => format!("{}/--", raw),
            _ => "--".to_string(),
        }
    }

    pub fn battery_adc_status_text(&self) -> &'static str {
        if self.battery_has_valid_sample() {
            board::BATTERY_ADC_CONFIRMED_TEXT
        } else {
            board::BATTERY_ADC_MISSING_TEXT
        }
    }

    pub const fn battery_adc_path_text(&self) -> &'static str {
        board::BATTERY_ADC_PATH_TEXT
    }

    pub fn battery_adc_source_text(&self) -> &'static str {
        self.battery_source_label
    }

    pub fn battery_adc_raw_label(&self) -> String {
        match self.battery_adc_raw {
            Some(raw) => raw.to_string(),
            None => "--".to_string(),
        }
    }

    pub fn battery_adc_mv_label(&self) -> String {
        match self.battery_adc_mv {
            Some(mv) => format!("{}mV", mv),
            None => "--".to_string(),
        }
    }

    pub fn battery_percent_value(&self) -> Option<u8> {
        let mv = self.battery_mv?;
        if self.battery_full_mv <= self.battery_empty_mv {
            return None;
        }

        let pct = if mv <= self.battery_empty_mv {
            0
        } else if mv >= self.battery_full_mv {
            100
        } else {
            let span = (self.battery_full_mv - self.battery_empty_mv) as u32;
            ((mv.saturating_sub(self.battery_empty_mv) as u32) * 100 / span).min(100) as u8
        };
        Some(pct)
    }

    pub fn battery_home_text(&self) -> String {
        match self.battery_percent_value() {
            Some(pct) => format!("{}%", pct),
            None => "--".to_string(),
        }
    }

    pub const fn battery_estimate_text(&self) -> &'static str {
        // This percentage is estimated from ADC-derived voltage until the
        // battery curve/divider are physically calibrated on this board.
        "EST"
    }

    pub fn battery_percent_detail_text(&self) -> String {
        match self.battery_percent_value() {
            Some(pct) => format!("{}% {}", pct, self.battery_estimate_text()),
            None => "-- EST".to_string(),
        }
    }

    pub const fn battery_power_text(&self) -> &'static str {
        // Do not claim charger state from ADC-only telemetry. This firmware has
        // no dedicated USB/VBUS/charge-status GPIO wired into AppState yet.
        "USB/UNKNOWN"
    }

    pub fn battery_detail_text(&self) -> String {
        self.battery_percent_detail_text()
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
        } else if let Some(count) = self.wifi_count {
            format!("AP{}", count.min(99))
        } else {
            "OFF".to_string()
        }
    }

    pub fn wifi_status_label(&self) -> &'static str {
        if self.wifi_connected {
            "CONNECTED"
        } else if self.wifi_count.is_some() {
            "STA READY"
        } else {
            "SCAN ERR"
        }
    }

    pub fn wifi_ssid_label(&self) -> String {
        if self.wifi_connected {
            if let Some(ssid) = self.wifi_ssid.as_ref() {
                let clipped = clipped_upper_ascii(ssid, 12);
                if !clipped.is_empty() {
                    return clipped;
                }
            }
            "CONNECTED".to_string()
        } else {
            "NOT SET".to_string()
        }
    }

    pub fn wifi_ap_count_label(&self) -> String {
        match self.wifi_count {
            Some(count) => format!("{} AP", count.min(999)),
            None => "-- AP".to_string(),
        }
    }

    pub fn wifi_connected_label(&self) -> &'static str {
        if self.wifi_connected {
            "YES"
        } else {
            "NO"
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

    pub fn home_weather_condition(&self) -> &str {
        let condition = self.weather.condition_label();
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
            self.weather.location_label()
        }
    }

    pub fn home_weather_temp(&self) -> &str {
        if self.weather.temperature_label() == "--" {
            "72F"
        } else {
            self.weather.temperature_label()
        }
    }

    pub fn sd_text(&self) -> String {
        if self.sd_present {
            "OK".to_string()
        } else {
            "NO".to_string()
        }
    }

    pub fn sd_total_text(&self) -> String {
        match self.sd_capacity_mb {
            Some(mb) if mb >= 1024 => format!("{}G", mb / 1024),
            Some(mb) => format!("{}M", mb),
            None => "--".to_string(),
        }
    }

    pub fn sd_free_text(&self) -> String {
        match self.sd_free_mb {
            Some(mb) if mb >= 1024 => format!("{}G", mb / 1024),
            Some(mb) => format!("{}M", mb),
            None => "--".to_string(),
        }
    }

    pub fn sd_free_total_text(&self) -> String {
        format!("{}/{}", self.sd_free_text(), self.sd_total_text())
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

// RAW-R42-VIDEO-MODEL-STATE-REMOVED
