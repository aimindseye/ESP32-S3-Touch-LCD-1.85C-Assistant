// RAW-R45-HOME-TRUE-SCREEN-MODULE
// True Rust module for the Home screen renderer.
// Uses crate-root helpers/constants while preserving r44 rendering behavior.
use crate::*;

// RAW-R44-SCREEN-HOME-RENDERER-SPLIT
// Included from main.rs; kept in crate-root item namespace intentionally.

pub(crate) fn draw_home_tile(model: &AppState, frame: &mut [u16]) {
    if model.home.status_detail_open {
        draw_watch_outer(frame, ACCENT_HOME);
        draw_arc_segment(frame, CX, CY, 166, 2, 204, 258, ACCENT_HOME_BLUE);
        draw_arc_segment(frame, CX, CY, 166, 2, 282, 336, ACCENT_HOME);
        draw_arc_segment(frame, CX, CY, 166, 2, 24, 76, ACCENT_WEATHER);
        draw_arc_segment(frame, CX, CY, 166, 2, 104, 156, ACCENT_HOME_GREEN);
        crate::screens::assistant::draw_assistant_orb(frame, 180, 112, 28, ACCENT_HOME);
        draw_chip(frame, 78, 170, 88, 30, "BAT", ACCENT_HOME_BLUE, true);
        draw_chip(frame, 194, 170, 88, 30, "WIFI", ACCENT_HOME, true);
        draw_chip(
            frame,
            116,
            218,
            128,
            30,
            "SD / RTC",
            ACCENT_HOME_GREEN,
            true,
        );
        draw_text_centered(frame, 270, model.home.detail_label(), ACCENT_HOME, 1);
    } else {
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

pub(crate) fn draw_home_battery_complication(
    frame: &mut [u16],
    cx: i32,
    cy: i32,
    percent: Option<u8>,
) {
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

pub(crate) fn draw_home_weather_icon(frame: &mut [u16], cx: i32, cy: i32, condition: &str) {
    match condition {
        "CLEAR" | "SUNNY" => draw_timeline_sun_icon(frame, cx, cy, ACCENT_WEATHER),
        "CLOUDY" | "LOCAL" => draw_timeline_cloud_icon(frame, cx, cy, WHITE),
        "RAIN" => draw_timeline_rain_icon(frame, cx, cy),
        "STORM" => draw_timeline_storm_icon(frame, cx, cy),
        "BREEZY" => draw_timeline_wind_icon(frame, cx, cy, WHITE),
        _ => draw_timeline_cloud_icon(frame, cx, cy, WHITE),
    }
}

// RAW-R48-R1-HOME-ASSISTANT-ORB-CALLSITE
