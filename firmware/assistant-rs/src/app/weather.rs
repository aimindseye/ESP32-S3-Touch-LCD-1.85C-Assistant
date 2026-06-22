//! Weather provider state and configuration.
//!
//! v0.1.21 expands the accepted v0.1.20-r2 provider foundation:
//! - configurable location/unit state
//! - simple on-device location/unit editor
//! - real hourly timeline parsing
//! - SD-backed last-known-good cache

use std::fmt::Write as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherProviderStatus {
    Local,
    Fetching,
    Live,
    Stale,
    Failed,
}

impl WeatherProviderStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Local => "LOCAL",
            Self::Fetching => "FETCHING",
            Self::Live => "LIVE",
            Self::Stale => "STALE",
            Self::Failed => "FAILED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherUnits {
    Fahrenheit,
    Celsius,
}

impl WeatherUnits {
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Fahrenheit => "F",
            Self::Celsius => "C",
        }
    }

    pub const fn api_value(self) -> &'static str {
        match self {
            Self::Fahrenheit => "fahrenheit",
            Self::Celsius => "celsius",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Fahrenheit => "UNITS F",
            Self::Celsius => "UNITS C",
        }
    }

    pub const fn toggled(self) -> Self {
        match self {
            Self::Fahrenheit => Self::Celsius,
            Self::Celsius => Self::Fahrenheit,
        }
    }

    pub fn from_label(value: &str) -> Self {
        if value.trim().eq_ignore_ascii_case("C") || value.trim().eq_ignore_ascii_case("CELSIUS") {
            Self::Celsius
        } else {
            Self::Fahrenheit
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WeatherLocation {
    pub label: &'static str,
    pub latitude: &'static str,
    pub longitude: &'static str,
    pub timezone_url: &'static str,
}

pub const WEATHER_LOCATIONS: [WeatherLocation; 4] = [
    WeatherLocation {
        label: "JERSEY CITY",
        latitude: "40.7282",
        longitude: "-74.0776",
        timezone_url: "America%2FNew_York",
    },
    WeatherLocation {
        label: "NEW YORK",
        latitude: "40.7128",
        longitude: "-74.0060",
        timezone_url: "America%2FNew_York",
    },
    WeatherLocation {
        label: "EDISON",
        latitude: "40.5187",
        longitude: "-74.4121",
        timezone_url: "America%2FNew_York",
    },
    WeatherLocation {
        label: "MUMBAI",
        latitude: "19.0760",
        longitude: "72.8777",
        timezone_url: "Asia%2FKolkata",
    },
];

#[derive(Debug, Clone)]
pub struct WeatherHourSlot {
    pub hour_label: String,
    pub temp_label: String,
    pub weather_code: i32,
}

#[derive(Debug, Clone)]
pub struct WeatherState {
    pub refresh_attempts: u32,
    pub status: String,
    pub temperature_label: String,
    pub condition_label: String,
    pub location_label: String,
    pub provider_status: WeatherProviderStatus,
    pub last_error: String,
    pub last_live_epoch: i64,
    pub location_index: usize,
    pub units: WeatherUnits,
    pub hourly_slots: Vec<WeatherHourSlot>,
    pub config_generation: u32,
}

#[derive(Debug, Clone)]
pub struct WeatherSample {
    pub temp_value: i32,
    pub weather_code: i32,
    pub hourly_slots: Vec<WeatherHourSlot>,
}

impl WeatherState {
    pub fn new() -> Self {
        let location = WEATHER_LOCATIONS[0];
        Self {
            refresh_attempts: 0,
            status: WeatherProviderStatus::Local.label().to_string(),
            temperature_label: "--".to_string(),
            condition_label: "LOCAL ONLY".to_string(),
            location_label: location.label.to_string(),
            provider_status: WeatherProviderStatus::Local,
            last_error: "NONE".to_string(),
            last_live_epoch: 0,
            location_index: 0,
            units: WeatherUnits::Fahrenheit,
            hourly_slots: Vec::new(),
            config_generation: 0,
        }
    }

    pub fn request_fetch(&mut self) {
        self.refresh_attempts = self.refresh_attempts.saturating_add(1);
        self.provider_status = WeatherProviderStatus::Fetching;
        self.status = WeatherProviderStatus::Fetching.label().to_string();
        self.last_error = "FETCH".to_string();
    }

    pub fn cycle_location(&mut self) {
        self.location_index = (self.location_index + 1) % WEATHER_LOCATIONS.len();
        self.sync_location_label();
        self.clear_for_config_change("LOCATION");
    }

    pub fn toggle_units(&mut self) {
        self.units = self.units.toggled();
        self.clear_for_config_change(self.units.label());
    }

    pub fn apply_live_weather(&mut self, sample: WeatherSample, epoch: i64) {
        self.temperature_label = format!("{}{}", sample.temp_value, self.units.suffix());
        self.condition_label = condition_from_weather_code(sample.weather_code).to_string();
        self.hourly_slots = sample.hourly_slots;
        self.provider_status = WeatherProviderStatus::Live;
        self.status = WeatherProviderStatus::Live.label().to_string();
        self.last_error = "NONE".to_string();
        self.last_live_epoch = epoch;
    }

    pub fn apply_failed(&mut self, reason: &str) {
        self.last_error = clipped_weather_text(reason, 14);

        if self.last_live_epoch > 0 {
            self.provider_status = WeatherProviderStatus::Stale;
            self.status = WeatherProviderStatus::Stale.label().to_string();
        } else {
            self.provider_status = WeatherProviderStatus::Failed;
            self.status = WeatherProviderStatus::Failed.label().to_string();
            if self.temperature_label == "--" {
                self.condition_label = "LOCAL ONLY".to_string();
            }
        }
    }

    pub fn apply_cache_text(&mut self, input: &str) -> bool {
        let mut loaded_any = false;
        let mut hourly_slots = Vec::new();

        for raw in input.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            let key = key.trim();
            let value = value.trim();

            match key {
                "location_index" => {
                    if let Ok(index) = value.parse::<usize>() {
                        self.location_index = index.min(WEATHER_LOCATIONS.len() - 1);
                        self.sync_location_label();
                        loaded_any = true;
                    }
                }
                "units" => {
                    self.units = WeatherUnits::from_label(value);
                    loaded_any = true;
                }
                "temp" => {
                    self.temperature_label = value.to_string();
                    loaded_any = true;
                }
                "condition" => {
                    self.condition_label = value.to_string();
                    loaded_any = true;
                }
                "last_live_epoch" => {
                    self.last_live_epoch = value.parse::<i64>().unwrap_or(0);
                }
                key if key.starts_with('h') => {
                    if let Some(slot) = parse_cache_hour(value) {
                        hourly_slots.push(slot);
                    }
                }
                _ => {}
            }
        }

        if !hourly_slots.is_empty() {
            self.hourly_slots = hourly_slots.into_iter().take(4).collect();
        }

        if loaded_any && self.temperature_label != "--" {
            self.provider_status = WeatherProviderStatus::Stale;
            self.status = WeatherProviderStatus::Stale.label().to_string();
            self.last_error = "CACHE".to_string();
        }

        loaded_any
    }

    pub fn cache_text(&self) -> String {
        let mut output = String::new();
        let _ = writeln!(output, "location_index={}", self.location_index);
        let _ = writeln!(output, "location={}", self.location_label());
        let _ = writeln!(output, "units={}", self.units.suffix());
        let _ = writeln!(output, "temp={}", self.temperature_label());
        let _ = writeln!(output, "condition={}", self.condition_label());
        let _ = writeln!(output, "last_live_epoch={}", self.last_live_epoch);

        for (index, slot) in self.hourly_slots.iter().take(4).enumerate() {
            let _ = writeln!(
                output,
                "h{}={}|{}|{}",
                index, slot.hour_label, slot.temp_label, slot.weather_code
            );
        }

        output
    }

    pub fn provider_url(&self) -> String {
        let location = WEATHER_LOCATIONS[self.location_index.min(WEATHER_LOCATIONS.len() - 1)];
        format!(
            "http://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,weather_code&hourly=temperature_2m,weather_code&forecast_days=1&temperature_unit={}&timezone={}",
            location.latitude,
            location.longitude,
            self.units.api_value(),
            location.timezone_url
        )
    }

    pub fn footer_label(&self) -> String {
        format!(
            "{} {} {}",
            self.location_label,
            self.units.suffix(),
            self.provider_status.label()
        )
    }

    pub fn location_label(&self) -> &str {
        self.location_label.as_str()
    }

    pub fn status_label(&self) -> &str {
        self.status.as_str()
    }

    pub fn temperature_label(&self) -> &str {
        self.temperature_label.as_str()
    }

    pub fn condition_label(&self) -> &str {
        self.condition_label.as_str()
    }

    pub fn last_error_label(&self) -> &str {
        self.last_error.as_str()
    }

    pub fn hourly_summary(&self) -> String {
        if self.hourly_slots.is_empty() {
            return "NONE".to_string();
        }

        self.hourly_slots
            .iter()
            .take(4)
            .map(|slot| format!("{}:{}", slot.hour_label, slot.temp_label))
            .collect::<Vec<_>>()
            .join(",")
    }

    fn sync_location_label(&mut self) {
        self.location_label = WEATHER_LOCATIONS
            [self.location_index.min(WEATHER_LOCATIONS.len() - 1)]
        .label
        .to_string();
    }

    fn clear_for_config_change(&mut self, reason: &str) {
        self.config_generation = self.config_generation.saturating_add(1);
        self.temperature_label = "--".to_string();
        self.condition_label = "TAP FETCH".to_string();
        self.hourly_slots.clear();
        self.provider_status = WeatherProviderStatus::Local;
        self.status = WeatherProviderStatus::Local.label().to_string();
        self.last_error = reason.to_string();
        self.last_live_epoch = 0;
    }
}

impl WeatherSample {
    pub fn parse_open_meteo(input: &str, units: WeatherUnits) -> Result<Self, &'static str> {
        let current = json_object_after_key(input, "current")
            .or_else(|| json_object_after_key(input, "current_weather"))
            .ok_or("NO CURRENT")?;

        let temp = parse_json_number_field(current, "temperature_2m")
            .or_else(|| parse_json_number_field(current, "temperature"))
            .ok_or("NO TEMP")?;
        let code = parse_json_number_field(current, "weather_code")
            .or_else(|| parse_json_number_field(current, "weathercode"))
            .ok_or("NO CODE")?;

        let current_time = parse_json_string_field(current, "time");
        let hourly_slots = parse_hourly_slots(input, current_time.as_deref(), units);

        Ok(Self {
            temp_value: round_f32_to_i32(temp),
            weather_code: round_f32_to_i32(code),
            hourly_slots,
        })
    }
}

fn parse_hourly_slots(
    input: &str,
    current_time: Option<&str>,
    units: WeatherUnits,
) -> Vec<WeatherHourSlot> {
    let Some(hourly) = json_object_after_key(input, "hourly") else {
        return Vec::new();
    };

    let times = parse_json_string_array_field(hourly, "time").unwrap_or_default();
    let temps = parse_json_number_array_field(hourly, "temperature_2m").unwrap_or_default();
    let codes = parse_json_number_array_field(hourly, "weather_code").unwrap_or_default();

    let len = times.len().min(temps.len()).min(codes.len());
    if len == 0 {
        return Vec::new();
    }

    let mut start = 0;
    if let Some(current_time) = current_time {
        let target = hour_floor_from_iso(current_time).unwrap_or_else(|| current_time.to_string());
        if let Some(index) = times
            .iter()
            .position(|time| time.as_str() >= target.as_str())
        {
            start = index;
        }
    }

    let mut slots = Vec::new();
    for index in start..len.min(start + 4) {
        slots.push(WeatherHourSlot {
            hour_label: hour_label_from_iso(&times[index]),
            temp_label: format!("{}", round_f32_to_i32(temps[index])),
            weather_code: round_f32_to_i32(codes[index]),
        });
    }

    if slots.is_empty() && len > 0 {
        for index in 0..len.min(4) {
            slots.push(WeatherHourSlot {
                hour_label: hour_label_from_iso(&times[index]),
                temp_label: format!("{}{}", round_f32_to_i32(temps[index]), units.suffix()),
                weather_code: round_f32_to_i32(codes[index]),
            });
        }
    }

    slots
}

fn parse_cache_hour(value: &str) -> Option<WeatherHourSlot> {
    let mut parts = value.split('|');
    let hour = parts.next()?.trim();
    let temp = parts.next()?.trim();
    let code = parts.next()?.trim().parse::<i32>().ok()?;

    Some(WeatherHourSlot {
        hour_label: hour.to_string(),
        temp_label: temp.to_string(),
        weather_code: code,
    })
}

fn hour_floor_from_iso(value: &str) -> Option<String> {
    // Open-Meteo current.time can include minutes. Hourly arrays use HH:00.
    // Align to the current hour instead of the next hour, so a 14:14 current
    // reading displays 2P first rather than incorrectly starting at 3P.
    let date_hour = value.get(0..13)?;
    Some(format!("{}:00", date_hour))
}

fn hour_label_from_iso(value: &str) -> String {
    let hour = value
        .split('T')
        .nth(1)
        .and_then(|time| time.get(0..2))
        .and_then(|hour| hour.parse::<u8>().ok())
        .unwrap_or(0);

    let suffix = if hour < 12 { 'A' } else { 'P' };
    let hour12 = match hour % 12 {
        0 => 12,
        value => value,
    };

    format!("{}{}", hour12, suffix)
}

fn json_object_after_key<'a>(input: &'a str, key: &str) -> Option<&'a str> {
    let marker = format!("\"{}\"", key);
    let key_pos = input.find(&marker)?;
    let after_key = &input[key_pos + marker.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    if !after_colon.starts_with('{') {
        return None;
    }

    let start_offset = input.len() - after_colon.len();
    skip_json_collection(input, start_offset, b'{', b'}')
}

fn json_array_after_key<'a>(input: &'a str, key: &str) -> Option<&'a str> {
    let marker = format!("\"{}\"", key);
    let key_pos = input.find(&marker)?;
    let after_key = &input[key_pos + marker.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    if !after_colon.starts_with('[') {
        return None;
    }

    let start_offset = input.len() - after_colon.len();
    skip_json_collection(input, start_offset, b'[', b']')
}

fn skip_json_collection<'a>(
    input: &'a str,
    start_offset: usize,
    open: u8,
    close: u8,
) -> Option<&'a str> {
    let bytes = input.as_bytes();
    let mut index = start_offset;
    let mut depth = 0_i32;
    let mut in_string = false;
    let mut escaped = false;

    while index < bytes.len() {
        let byte = bytes[index];

        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            index += 1;
            continue;
        }

        if byte == b'"' {
            in_string = true;
        } else if byte == open {
            depth += 1;
        } else if byte == close {
            depth -= 1;
            if depth == 0 {
                return input.get(start_offset..=index);
            }
        }

        index += 1;
    }

    None
}

fn parse_json_number_field(input: &str, key: &str) -> Option<f32> {
    let marker = format!("\"{}\"", key);
    let key_pos = input.find(&marker)?;
    let after_key = &input[key_pos + marker.len()..];
    let colon_pos = after_key.find(':')?;
    parse_number_prefix(after_key[colon_pos + 1..].trim_start())
}

fn parse_json_string_field(input: &str, key: &str) -> Option<String> {
    let marker = format!("\"{}\"", key);
    let key_pos = input.find(&marker)?;
    let after_key = &input[key_pos + marker.len()..];
    let colon_pos = after_key.find(':')?;
    let rest = after_key[colon_pos + 1..].trim_start();
    parse_json_string_prefix(rest)
}

fn parse_json_string_array_field(input: &str, key: &str) -> Option<Vec<String>> {
    let array = json_array_after_key(input, key)?;
    let mut values = Vec::new();
    let bytes = array.as_bytes();
    let mut index = 1;

    while index + 1 < bytes.len() {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if bytes.get(index) == Some(&b']') {
            break;
        }
        if bytes.get(index) == Some(&b'"') {
            let rest = &array[index..];
            let value = parse_json_string_prefix(rest)?;
            index += quoted_json_len(rest)?;
            values.push(value);
        }
        while index < bytes.len() && !matches!(bytes[index], b',' | b']') {
            index += 1;
        }
        if bytes.get(index) == Some(&b',') {
            index += 1;
        }
    }

    Some(values)
}

fn parse_json_number_array_field(input: &str, key: &str) -> Option<Vec<f32>> {
    let array = json_array_after_key(input, key)?;
    let inner = array.trim().trim_start_matches('[').trim_end_matches(']');
    let mut values = Vec::new();

    for raw in inner.split(',') {
        let value = raw.trim();
        if value.is_empty() || value == "null" {
            continue;
        }
        values.push(value.parse::<f32>().ok()?);
    }

    Some(values)
}

fn parse_number_prefix(input: &str) -> Option<f32> {
    let mut end = 0;
    for ch in input.chars() {
        if ch.is_ascii_digit() || ch == '-' || ch == '+' || ch == '.' {
            end += ch.len_utf8();
        } else {
            break;
        }
    }

    if end == 0 {
        None
    } else {
        input[..end].parse::<f32>().ok()
    }
}

fn parse_json_string_prefix(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    if bytes.first() != Some(&b'"') {
        return None;
    }

    let end = quoted_json_len(input)?;
    let body = &input[1..end - 1];
    Some(body.replace("\\\"", "\"").replace("\\\\", "\\"))
}

fn quoted_json_len(input: &str) -> Option<usize> {
    let bytes = input.as_bytes();
    if bytes.first() != Some(&b'"') {
        return None;
    }

    let mut index = 1;
    let mut escaped = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == b'"' {
            return Some(index + 1);
        }
        index += 1;
    }

    None
}

fn round_f32_to_i32(value: f32) -> i32 {
    if value >= 0.0 {
        (value + 0.5) as i32
    } else {
        (value - 0.5) as i32
    }
}

pub fn condition_from_weather_code(code: i32) -> &'static str {
    match code {
        0 => "CLEAR",
        1 | 2 => "SUNNY",
        3 | 45 | 48 => "CLOUDY",
        51..=67 | 80..=82 => "RAIN",
        71..=77 | 85..=86 => "SNOW",
        95..=99 => "STORM",
        _ => "BREEZY",
    }
}

fn clipped_weather_text(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

pub const WEATHER_DEFAULT_LOCATION: &str = "JERSEY CITY";
pub const WEATHER_CACHE_PATH: &str = "/sdcard/WEATHER.TXT";
pub const WEATHER_PROVIDER_MARKER: &str =
    "v0.1.21-r2 weather center location autofetch: cached city repair";
