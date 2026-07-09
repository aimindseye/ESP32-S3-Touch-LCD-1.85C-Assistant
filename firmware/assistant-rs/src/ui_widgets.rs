// RAW-V1-0-1-R12-UI-WIDGETS-MODULE
// Higher-level screen widgets/icons extracted from main.rs.
use crate::*;

pub(crate) fn draw_calendar_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    stroke_rect(frame, cx - 7, cy - 7, 14, 14, color);
    draw_line(frame, cx - 7, cy - 3, cx + 7, cy - 3, color);
    draw_line(frame, cx - 4, cy - 10, cx - 4, cy - 6, color);
    draw_line(frame, cx + 4, cy - 10, cx + 4, cy - 6, color);
    fill_rect(frame, cx - 3, cy + 1, 2, 2, color);
    fill_rect(frame, cx + 2, cy + 1, 2, 2, color);
}

// Internet Radio screen now dispatches directly from screens/assistant.rs.

pub(crate) fn draw_watch_outer(frame: &mut [u16], accent: u16) {
    fill_circle(frame, CX, CY, R_OUTER - 6, BG);
    stroke_circle(frame, CX, CY, R_OUTER, RING_DIM);
    draw_arc_segment(frame, CX, CY, 170, 1, 224, 316, accent);
}

pub(crate) fn draw_complication_battery(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    fill_circle(frame, cx, cy, 18, BG_DARK);
    stroke_circle(frame, cx, cy, 18, color);
    stroke_rect(frame, cx - 8, cy - 5, 15, 10, WHITE);
    fill_rect(frame, cx + 8, cy - 2, 2, 4, WHITE);
    fill_rect(frame, cx - 6, cy - 3, 9, 6, color);
}

pub(crate) fn draw_complication_wifi(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    fill_circle(frame, cx, cy, 18, BG_DARK);
    stroke_circle(frame, cx, cy, 18, color);
    draw_arc_segment(frame, cx, cy + 8, 13, 1, 220, 320, WHITE);
    draw_arc_segment(frame, cx, cy + 8, 8, 1, 230, 310, WHITE);
    fill_circle(frame, cx, cy + 8, 2, color);
}

pub(crate) fn draw_complication_sd(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    fill_circle(frame, cx, cy, 18, BG_DARK);
    stroke_circle(frame, cx, cy, 18, color);
    stroke_rect(frame, cx - 7, cy - 10, 14, 18, WHITE);
    fill_rect(frame, cx + 3, cy - 10, 4, 5, BG_DARK);
}

pub(crate) fn draw_sun_cloud_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    draw_sun_icon(frame, cx - 16, cy - 6, 16, color);
    draw_cloud_icon(frame, cx + 16, cy + 8, WHITE);
}

pub(crate) fn draw_sun_icon(frame: &mut [u16], cx: i32, cy: i32, r: i32, color: u16) {
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

pub(crate) fn draw_cloud_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    fill_circle(frame, cx - 22, cy + 4, 16, color);
    fill_circle(frame, cx, cy - 5, 21, color);
    fill_circle(frame, cx + 22, cy + 6, 15, color);
    fill_rounded_rect(frame, cx - 40, cy + 3, 80, 24, 12, color);
}

pub(crate) fn draw_cloud_icon_medium(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    fill_circle(frame, cx - 18, cy + 4, 13, color);
    fill_circle(frame, cx, cy - 4, 17, color);
    fill_circle(frame, cx + 18, cy + 5, 12, color);
    fill_rounded_rect(frame, cx - 33, cy + 3, 66, 20, 10, color);
}

pub(crate) fn draw_wind_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    draw_line(frame, cx - 54, cy - 16, cx + 54, cy - 16, color);
    draw_line(frame, cx - 36, cy, cx + 42, cy, color);
    draw_line(frame, cx - 50, cy + 16, cx + 30, cy + 16, color);
}

pub(crate) fn draw_wind_icon_compact(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    draw_line(frame, cx - 34, cy - 10, cx + 34, cy - 10, color);
    draw_line(frame, cx - 26, cy, cx + 28, cy, color);
    draw_line(frame, cx - 32, cy + 10, cx + 22, cy + 10, color);
}

pub(crate) fn draw_hour_chip(frame: &mut [u16], cx: i32, cy: i32, hour: &str, temp: &str) {
    fill_rounded_rect(frame, cx - 26, cy - 18, 52, 38, 12, 0x0354);
    stroke_round_chip(frame, cx - 26, cy - 18, 52, 38, 12, ACCENT_WEATHER_BLUE);
    draw_text_centered_at(frame, cx, cy - 5, hour, WHITE, 1);
    draw_text_centered_at(frame, cx, cy + 12, temp, WHITE, 1);
}

#[derive(Clone, Copy)]
pub(crate) enum WeatherMiniIcon {
    Sun,
    PartlyCloudy,
    Cloud,
    Rain,
    Storm,
    Wind,
}

#[derive(Clone)]
pub(crate) struct WeatherHourRender {
    pub(crate) hour: String,
    pub(crate) temp: String,
    pub(crate) icon: WeatherMiniIcon,
}

pub(crate) fn fallback_timeline_entries(condition: &str) -> Vec<WeatherHourRender> {
    let raw = match condition {
        "CLEAR" | "LOCAL" => [
            ("11A", "72", WeatherMiniIcon::PartlyCloudy),
            ("12P", "73", WeatherMiniIcon::Sun),
            ("1P", "72", WeatherMiniIcon::Cloud),
            ("2P", "70", WeatherMiniIcon::Wind),
        ],
        "SUNNY" => [
            ("11A", "75", WeatherMiniIcon::Sun),
            ("12P", "77", WeatherMiniIcon::Sun),
            ("1P", "79", WeatherMiniIcon::PartlyCloudy),
            ("2P", "76", WeatherMiniIcon::Wind),
        ],
        "CLOUDY" => [
            ("11A", "68", WeatherMiniIcon::Cloud),
            ("12P", "68", WeatherMiniIcon::Cloud),
            ("1P", "70", WeatherMiniIcon::PartlyCloudy),
            ("2P", "67", WeatherMiniIcon::Rain),
        ],
        "BREEZY" => [
            ("11A", "70", WeatherMiniIcon::Wind),
            ("12P", "71", WeatherMiniIcon::PartlyCloudy),
            ("1P", "69", WeatherMiniIcon::Wind),
            ("2P", "68", WeatherMiniIcon::Storm),
        ],
        _ => [
            ("11A", "72", WeatherMiniIcon::PartlyCloudy),
            ("12P", "73", WeatherMiniIcon::Sun),
            ("1P", "72", WeatherMiniIcon::Cloud),
            ("2P", "70", WeatherMiniIcon::Wind),
        ],
    };

    raw.iter()
        .map(|(hour, temp, icon)| WeatherHourRender {
            hour: (*hour).to_string(),
            temp: (*temp).to_string(),
            icon: *icon,
        })
        .collect()
}

pub(crate) fn mini_icon_from_weather_code(code: i32) -> WeatherMiniIcon {
    match condition_from_weather_code(code) {
        "CLEAR" | "SUNNY" => WeatherMiniIcon::Sun,
        "CLOUDY" => WeatherMiniIcon::Cloud,
        "RAIN" => WeatherMiniIcon::Rain,
        "SNOW" => WeatherMiniIcon::Cloud,
        "STORM" => WeatherMiniIcon::Storm,
        "BREEZY" => WeatherMiniIcon::Wind,
        _ => WeatherMiniIcon::PartlyCloudy,
    }
}

pub(crate) fn draw_tiny_temp_value(frame: &mut [u16], cx: i32, y: i32, text: &str, color: u16) {
    // Kept for future numeric experiments. Large-strip Option D uses scale-2
    // text because the cards are now tall enough for readable hourly temps.
    draw_text_centered_at(frame, cx, y, text, color, 2);
}

pub(crate) fn draw_timeline_sun_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
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

pub(crate) fn draw_timeline_cloud_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    fill_circle(frame, cx - 9, cy + 3, 7, color);
    fill_circle(frame, cx, cy - 4, 9, color);
    fill_circle(frame, cx + 9, cy + 3, 7, color);
    fill_rounded_rect(frame, cx - 16, cy + 3, 32, 10, 5, color);
}

pub(crate) fn draw_timeline_partly_cloudy_icon(frame: &mut [u16], cx: i32, cy: i32) {
    fill_circle(frame, cx - 8, cy - 6, 6, ACCENT_WEATHER);
    draw_line(frame, cx - 8, cy - 15, cx - 8, cy - 13, ACCENT_WEATHER);
    draw_line(frame, cx - 17, cy - 6, cx - 15, cy - 6, ACCENT_WEATHER);
    draw_line(frame, cx + 1, cy - 6, cx + 3, cy - 6, ACCENT_WEATHER);
    draw_timeline_cloud_icon(frame, cx + 5, cy + 2, WHITE);
}

pub(crate) fn draw_timeline_rain_icon(frame: &mut [u16], cx: i32, cy: i32) {
    draw_timeline_cloud_icon(frame, cx, cy - 5, WHITE);
    draw_line(frame, cx - 8, cy + 9, cx - 9, cy + 13, ACCENT_WEATHER_BLUE);
    draw_line(frame, cx, cy + 9, cx - 1, cy + 13, ACCENT_WEATHER_BLUE);
    draw_line(frame, cx + 8, cy + 9, cx + 7, cy + 13, ACCENT_WEATHER_BLUE);
}

pub(crate) fn draw_timeline_storm_icon(frame: &mut [u16], cx: i32, cy: i32) {
    draw_timeline_cloud_icon(frame, cx, cy - 5, WHITE);
    draw_line(frame, cx + 3, cy + 8, cx - 2, cy + 12, ACCENT_WEATHER);
    draw_line(frame, cx - 2, cy + 12, cx + 2, cy + 12, ACCENT_WEATHER);
    draw_line(frame, cx + 2, cy + 12, cx - 1, cy + 17, ACCENT_WEATHER);
}

pub(crate) fn draw_timeline_wind_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    draw_line(frame, cx - 15, cy - 8, cx + 15, cy - 8, color);
    draw_line(frame, cx - 12, cy, cx + 13, cy, color);
    draw_line(frame, cx - 14, cy + 8, cx + 10, cy + 8, color);
}

pub(crate) fn compact_condition(condition: &str) -> &'static str {
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
    } else if condition.contains("SNOW") {
        "CLOUDY"
    } else {
        "LOCAL"
    }
}

pub(crate) fn draw_album_tile(frame: &mut [u16], cx: i32, cy: i32) {
    fill_rounded_rect(frame, cx - 25, cy - 25, 50, 50, 7, 0x2108);
    fill_rect(frame, cx - 21, cy - 21, 42, 17, ACCENT_MUSIC);
    fill_rect(frame, cx - 21, cy - 4, 42, 22, ACCENT_MUSIC_BLUE);
    fill_rect(frame, cx - 21, cy + 18, 42, 3, ACCENT_WEATHER);
    draw_text_centered_at(frame, cx, cy + 3, "AI", WHITE, 2);
}

pub(crate) fn mock_music_progress(toggle_count: u32) -> u8 {
    match toggle_count % 4 {
        0 => 15,
        1 => 35,
        2 => 60,
        _ => 85,
    }
}

pub(crate) fn draw_media_button(frame: &mut [u16], cx: i32, cy: i32, playing: bool) {
    fill_circle(frame, cx, cy, 42, BG_DARK);
    stroke_circle(frame, cx, cy, 44, ACCENT_MUSIC);
    if playing {
        fill_rounded_rect(frame, cx - 13, cy - 21, 8, 42, 4, WHITE);
        fill_rounded_rect(frame, cx + 5, cy - 21, 8, 42, 4, WHITE);
    } else {
        fill_play_triangle(frame, cx + 2, cy, 44, WHITE);
    }
}

pub(crate) fn draw_skip_icon(frame: &mut [u16], cx: i32, cy: i32, next: bool, color: u16) {
    if next {
        fill_play_triangle(frame, cx + 5, cy, 18, color);
        fill_rect(frame, cx + 12, cy - 10, 2, 20, color);
    } else {
        fill_left_triangle(frame, cx - 5, cy, 18, color);
        fill_rect(frame, cx - 14, cy - 10, 2, 20, color);
    }
}

pub(crate) fn draw_track_label(frame: &mut [u16], y: i32, label: &str) {
    // Keep music title as one line to avoid source-pill overlap.
    draw_text_centered(frame, y, label, WHITE, 2);
}

pub(crate) fn draw_waveform(frame: &mut [u16], cx: i32, cy: i32, amp: i32, color: u16) {
    let xs = [-70, -54, -38, -22, -8, 8, 22, 38, 54, 70];
    let heights = [6, 16, 28, 14, amp, amp, 14, 28, 16, 6];
    for i in 0..xs.len() - 1 {
        let x0 = cx + xs[i];
        let y0 = cy
            + if i % 2 == 0 {
                -heights[i] / 2
            } else {
                heights[i] / 2
            };
        let x1 = cx + xs[i + 1];
        let y1 = cy
            + if (i + 1) % 2 == 0 {
                -heights[i + 1] / 2
            } else {
                heights[i + 1] / 2
            };
        draw_line(frame, x0, y0, x1, y1, color);
    }
    fill_circle(frame, cx, cy, 4, WHITE);
}

pub(crate) fn draw_microphone_button(frame: &mut [u16], cx: i32, cy: i32, listening: bool) {
    let color = if listening {
        ACCENT_ASSISTANT
    } else {
        ACCENT_ASSISTANT_BLUE
    };
    fill_circle(frame, cx, cy, 24, color);
    fill_rounded_rect(frame, cx - 6, cy - 13, 12, 20, 6, WHITE);
    draw_line(frame, cx, cy + 7, cx, cy + 16, WHITE);
    draw_arc_segment(frame, cx, cy + 4, 12, 1, 40, 140, WHITE);
}

pub(crate) fn draw_cancel_glyph(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    draw_line(frame, cx - 7, cy - 7, cx + 7, cy + 7, color);
    draw_line(frame, cx - 7, cy + 7, cx + 7, cy - 7, color);
}

pub(crate) fn draw_scroll_arc(frame: &mut [u16]) {
    draw_arc_segment(frame, CX, CY, 168, 2, 332, 28, MUTED);
}

pub(crate) fn draw_toggle_ring(frame: &mut [u16], cx: i32, cy: i32, on: bool) {
    let color = if on { ACCENT_SETTINGS } else { MUTED };
    draw_ring_meter(
        frame,
        cx,
        cy,
        54,
        3,
        if on { 82 } else { 25 },
        RING_DIM,
        color,
    );
    fill_circle(frame, cx, cy, 39, BG_DARK);
    fill_circle(frame, cx, if on { cy - 54 } else { cy + 54 }, 3, color);
}

pub(crate) fn draw_wifi_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    draw_arc_segment(frame, cx, cy + 6, 13, 1, 220, 320, color);
    draw_arc_segment(frame, cx, cy + 6, 8, 1, 230, 310, color);
    fill_circle(frame, cx, cy + 8, 2, color);
}

pub(crate) fn draw_bell_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    stroke_circle(frame, cx, cy, 9, color);
    fill_rect(frame, cx - 7, cy + 6, 14, 3, color);
    fill_circle(frame, cx, cy + 12, 2, color);
}

pub(crate) fn draw_mini_gear(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
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

pub(crate) fn draw_page_dots(model: &AppState, frame: &mut [u16]) {
    let count = ALL_PAGES.len() as i32;
    let spacing = 12;
    let start_x = CX - ((count - 1) * spacing / 2);

    for (idx, page) in ALL_PAGES.iter().copied().enumerate() {
        let selected = page == model.current_page;
        let r = if selected { 3 } else { 2 };
        let color = if selected {
            page_orchestration::accent_for_page(page)
        } else {
            SOFT
        };
        fill_circle(
            frame,
            start_x + idx as i32 * spacing,
            FOOTER_DOTS_Y,
            r,
            color,
        );
    }
}

// RAW-V1-0-1-R12-UI-WIDGETS-MODULE-END
