// RAW-R46-R1-WEATHER-TRUE-SCREEN-MODULE
// True Rust module for the Weather screen renderer.
use crate::*;

// RAW-R44-SCREEN-WEATHER-RENDERER-SPLIT
// Included from main.rs; kept in crate-root item namespace intentionally.

pub(crate) fn weather_sd_error_label(code: i32) -> &'static str {
    match code {
        -1 => "BAD ARG",
        -2 => "SD MOUNT",
        -3 => "NO FILE",
        -4 => "SD IO",
        _ => "SD FAIL",
    }
}

pub(crate) fn weather_http_error_label(code: i32) -> &'static str {
    match code {
        -1 => "BAD ARG",
        -2 => "NO CLIENT",
        -3 => "HTTP OPEN",
        -4 => "HTTP READ",
        -1300..=-1200 => "HTTP 2XX?",
        -1404 => "HTTP 404",
        -1500..=-1000 => "HTTP STATUS",
        _ => "HTTP FAIL",
    }
}

pub(crate) fn weather_body_sample(body: &str, max_chars: usize) -> String {
    body.chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .take(max_chars)
        .collect()
}

pub(crate) fn draw_weather_tile(model: &AppState, frame: &mut [u16]) {
    // Option D large-strip layout:
    // main summary is compact; the lower third has large pill cards.
    // main icon y=34..64, temperature y=70..118, condition y=138,
    // large timeline cards y=176..278, sample y=306.
    let temp = if model.weather.temperature_label() == "--" {
        "72F"
    } else {
        model.weather.temperature_label()
    };
    let condition = compact_condition(model.weather.condition_label());

    match condition {
        "CLEAR" | "SUNNY" => draw_sun_icon(frame, 180, 40, 11, ACCENT_WEATHER),
        "CLOUDY" => draw_cloud_icon_medium(frame, 180, 42, WHITE),
        "BREEZY" | "LOCAL" => draw_wind_icon_compact(frame, 180, 42, WHITE),
        _ => draw_wind_icon_compact(frame, 180, 42, WHITE),
    }

    draw_numeric_value_centered(frame, 70, temp, 24, 4, WHITE);
    draw_text_centered(frame, 124, "WEATHER LOCATION", ACCENT_WEATHER, 1);
    draw_text_centered(frame, 138, model.weather.location_label(), WHITE, 2);
    draw_weather_location_nav_buttons(frame, model);
    // RAW-R56-R2-WEATHER-LOCATION-LABEL
    draw_weather_hour_values(frame, &model.weather.hourly_slots, condition);
    draw_text_centered(frame, 306, &model.weather.footer_label(), WHITE, 1);
}

pub(crate) fn draw_weather_hour_values(
    frame: &mut [u16],
    hourly_slots: &[WeatherHourSlot],
    condition: &str,
) {
    let entries = if hourly_slots.len() >= 4 {
        hourly_slots
            .iter()
            .take(4)
            .map(|slot| WeatherHourRender {
                hour: slot.hour_label.clone(),
                temp: slot.temp_label.clone(),
                icon: mini_icon_from_weather_code(slot.weather_code),
            })
            .collect::<Vec<_>>()
    } else {
        fallback_timeline_entries(condition)
    };

    let centers = [74, 145, 216, 287];
    for (index, entry) in entries.iter().take(4).enumerate() {
        draw_weather_timeline_slot(frame, centers[index], entry, index == 0);
    }
}

pub(crate) fn draw_weather_timeline_slot(
    frame: &mut [u16],
    cx: i32,
    entry: &WeatherHourRender,
    highlighted: bool,
) {
    // Large Option D slot contract: x=(cx-30..cx+30), y=176..278.
    // Larger text is allowed because hour, icon, and temperature stay in separate lanes.
    let hour_color = if highlighted { ACCENT_WEATHER } else { WHITE };
    draw_text_centered_at(frame, cx, 190, &entry.hour, hour_color, 2);
    draw_weather_timeline_icon(frame, cx, 226, entry.icon);
    draw_text_centered_at(frame, cx, 262, &entry.temp, WHITE, 2);
}

pub(crate) fn draw_weather_timeline_icon(
    frame: &mut [u16],
    cx: i32,
    cy: i32,
    icon: WeatherMiniIcon,
) {
    match icon {
        WeatherMiniIcon::Sun => draw_timeline_sun_icon(frame, cx, cy, ACCENT_WEATHER),
        WeatherMiniIcon::PartlyCloudy => draw_timeline_partly_cloudy_icon(frame, cx, cy),
        WeatherMiniIcon::Cloud => draw_timeline_cloud_icon(frame, cx, cy, WHITE),
        WeatherMiniIcon::Rain => draw_timeline_rain_icon(frame, cx, cy),
        WeatherMiniIcon::Storm => draw_timeline_storm_icon(frame, cx, cy),
        WeatherMiniIcon::Wind => draw_timeline_wind_icon(frame, cx, cy, WHITE),
    }
}

// RAW-R56-R1-WEATHER-NAV-BUTTONS

pub(crate) fn draw_weather_location_nav_buttons(frame: &mut [u16], _model: &AppState) {
    draw_text_centered(frame, 318, "< LOC        LOC >", WHITE, 1);
}

// RAW-R56-R2-WEATHER-LABEL-REPAIR

// RAW-V1-0-1-WEATHER-NAV-NO-UNITS
