// RAW-R49-SETTINGS-TRUE-SCREEN-MODULE
// True Rust module for the Settings screen renderer.
// Uses crate-root helpers/constants while preserving r48-r1 behavior.
use crate::*;

// RAW-R44-SCREEN-SETTINGS-RENDERER-SPLIT
// Included from main.rs; kept in crate-root item namespace intentionally.

pub(crate) fn settings_detail_intent_from_touch_summary(
    model: &AppState,
    summary: &TouchSummary,
) -> Option<UiIntent> {
    if model.current_page != AssistantPage::Settings || !model.settings.detail_open {
        return None;
    }

    if crate::settings_action_router::is_settings_detail_header_tap(summary) {
        println!("touch-class: settings detail header tap accepted back");
        return Some(UiIntent::SettingsBackToOverview);
    }

    let up_travel = summary.start_y as i16 - summary.min_y as i16;
    let down_travel = summary.max_y as i16 - summary.start_y as i16;
    let signed_span_dy = if up_travel >= down_travel {
        -up_travel
    } else {
        down_travel
    };

    let abs_span_dy = signed_span_dy.abs();
    let abs_span_x = summary.span_x.abs();
    let vertical_dominant = abs_span_dy >= SETTINGS_DETAIL_VERTICAL_SWIPE_MIN_DY
        && (abs_span_dy as i32 * 2) > (abs_span_x as i32 * 3);

    // CST816 reports 0x01 for up and 0x02 for down on this family.
    // Only consume vertical gestures inside Settings detail. Horizontal swipes
    // continue to use the global page navigation classifier below.
    let gesture_vertical = match summary.gesture {
        0x01 => Some(UiIntent::SettingsPreviousDetail),
        0x02 => Some(UiIntent::SettingsNextDetail),
        _ => None,
    };

    if vertical_dominant {
        let intent = if signed_span_dy < 0 {
            UiIntent::SettingsPreviousDetail
        } else {
            UiIntent::SettingsNextDetail
        };

        return log_settings_detail_vertical_intent(intent, signed_span_dy, summary);
    }

    if let Some(intent) = gesture_vertical {
        if abs_span_x < TOUCH_GESTURE_SPAN_PREFER_PX {
            return log_settings_detail_vertical_intent(intent, signed_span_dy, summary);
        }
    }

    None
}

pub(crate) fn draw_settings_tile(model: &AppState, frame: &mut [u16]) {
    if !model.settings.detail_open {
        draw_text_centered_at(frame, 180, 32, &model.rtc_hms(), MUTED, 1);
        draw_text_centered_at(frame, 180, 76, "SETTINGS", ACCENT_SETTINGS, 2);

        let rows = if model.settings.overview_page == 0 {
            [
                (SettingsPanel::Network, "NETWORK", SettingsIcon::Wifi),
                (SettingsPanel::Weather, "WEATHER", SettingsIcon::Weather),
                (SettingsPanel::Time, "TIME", SettingsIcon::Time),
                (SettingsPanel::Display, "DISPLAY", SettingsIcon::Display),
            ]
        } else {
            [
                (SettingsPanel::Sound, "SOUND", SettingsIcon::Sound),
                (SettingsPanel::Storage, "STORAGE", SettingsIcon::Storage),
                (SettingsPanel::Device, "DEVICE", SettingsIcon::Device),
                (
                    SettingsPanel::Diagnostics,
                    "DIAG",
                    SettingsIcon::Diagnostics,
                ),
            ]
        };

        for (index, (panel, label, icon)) in rows.iter().enumerate() {
            draw_settings_list_row(
                frame,
                55,
                96 + index as i32 * 50,
                label,
                *icon,
                model.settings.selected == *panel,
            );
        }

        draw_text_centered_at(
            frame,
            180,
            326,
            &format!("PAGE {} TAP MORE", model.settings.overview_page_label()),
            MUTED,
            1,
        );
    } else {
        draw_settings_detail(frame, model);
    }
}

#[derive(Clone, Copy)]
pub(crate) enum SettingsIcon {
    Wifi,
    Weather,
    Time,
    Display,
    Sound,
    Storage,
    Device,
    Diagnostics,
}

pub(crate) fn draw_settings_list_row(
    frame: &mut [u16],
    x: i32,
    y: i32,
    label: &str,
    icon: SettingsIcon,
    selected: bool,
) {
    let outline = if selected { ACCENT_SETTINGS } else { RING_DIM };
    let fill = if selected { 0x1096 } else { BG_DARK };

    fill_rounded_rect(frame, x, y, 250, 38, 16, fill);
    stroke_rounded_rect(frame, x, y, 250, 38, 16, outline);

    let icon_cx = x + 32;
    let icon_cy = y + 19;
    fill_circle(
        frame,
        icon_cx,
        icon_cy,
        15,
        if selected { 0x18F8 } else { BG },
    );
    stroke_circle(frame, icon_cx, icon_cy, 15, outline);
    draw_settings_row_icon(frame, icon, icon_cx, icon_cy, WHITE);

    draw_text(frame, x + 74, y + 26, label, WHITE, 2);
    draw_settings_chevron(frame, x + 226, y + 19, if selected { WHITE } else { MUTED });
}

pub(crate) fn draw_settings_row_icon(
    frame: &mut [u16],
    icon: SettingsIcon,
    cx: i32,
    cy: i32,
    color: u16,
) {
    match icon {
        SettingsIcon::Wifi => draw_wifi_icon(frame, cx, cy - 4, color),
        SettingsIcon::Weather => draw_sun_icon(frame, cx, cy, 7, color),
        SettingsIcon::Time => draw_settings_time_icon(frame, cx, cy, color),
        SettingsIcon::Display => draw_settings_sun_icon(frame, cx, cy, color),
        SettingsIcon::Sound => draw_settings_sound_icon(frame, cx, cy, color),
        SettingsIcon::Storage => draw_settings_storage_icon(frame, cx, cy, color),
        SettingsIcon::Device => draw_settings_info_icon(frame, cx, cy, color),
        SettingsIcon::Diagnostics => draw_settings_diag_icon(frame, cx, cy, color),
    }
}

pub(crate) fn draw_settings_detail(frame: &mut [u16], model: &AppState) {
    match model.settings.selected {
        SettingsPanel::Network => draw_settings_network_detail(frame, model),
        SettingsPanel::Weather => draw_settings_weather_detail(frame, model),
        SettingsPanel::Time => draw_settings_time_detail(frame, model),
        SettingsPanel::Display => draw_settings_display_detail(frame, model),
        SettingsPanel::Sound => draw_settings_sound_detail(frame, model),
        SettingsPanel::Storage => draw_settings_storage_detail(frame, model),
        SettingsPanel::Device => draw_settings_device_detail(frame, model),
        SettingsPanel::Diagnostics => draw_settings_diagnostics_detail(frame, model),
    }
}

pub(crate) struct SettingsDetailRow<'a> {
    label: &'a str,
    value: &'a str,
    color: u16,
}

pub(crate) fn draw_settings_detail_template(
    frame: &mut [u16],
    model: &AppState,
    title: &str,
    icon: SettingsIcon,
    primary: &str,
    rows: &[SettingsDetailRow<'_>],
    footer: &str,
) {
    draw_settings_detail_clean_base(frame);
    draw_text_centered_at(frame, 180, 32, &model.rtc_hms(), MUTED, 1);
    draw_text_centered_at(frame, 180, 72, title, ACCENT_SETTINGS, 2);

    fill_circle(frame, 180, 112, 28, BG_DARK);
    stroke_circle(frame, 180, 112, 28, ACCENT_SETTINGS);
    draw_settings_row_icon(frame, icon, 180, 112, WHITE);

    draw_text_centered_at(frame, 180, 148, &settings_clip(primary, 16), WHITE, 2);

    let row_y = [174, 208, 242, 276];
    for (index, row) in rows.iter().take(4).enumerate() {
        draw_settings_detail_row(
            frame,
            row_y[index],
            row.label,
            row.value,
            row.color,
            index == 0,
        );
    }

    draw_text_centered_at(frame, 180, 324, &settings_clip(footer, 22), MUTED, 1);
}

pub(crate) fn draw_settings_detail_clean_base(frame: &mut [u16]) {
    // v0.1.22-r5: Settings overview still uses the baked settings_base.rgb565.
    // Detail pages use a dynamic template, so clear only the baked row/card area
    // that otherwise shows old left circles and old row outlines behind details.
    // Keep the outer neon arc, top time, and title visual language intact.
    fill_rounded_rect(frame, 62, 88, 236, 66, 24, BG);
    fill_rounded_rect(frame, 48, 150, 264, 42, 18, BG);
    fill_rounded_rect(frame, 48, 184, 264, 42, 18, BG);
    fill_rounded_rect(frame, 48, 218, 264, 42, 18, BG);
    fill_rounded_rect(frame, 48, 252, 264, 42, 18, BG);
    fill_rounded_rect(frame, 76, 300, 208, 34, 14, BG);
}

pub(crate) fn draw_settings_detail_row(
    frame: &mut [u16],
    y: i32,
    label: &str,
    value: &str,
    value_color: u16,
    selected: bool,
) {
    // Standard detail grid: row x=64 width=232 height=30, label x=82, value x=166.
    let fill = if selected { 0x1096 } else { BG_DARK };
    let outline = if selected { ACCENT_SETTINGS } else { RING_DIM };

    fill_rounded_rect(frame, 64, y - 18, 232, 30, 13, fill);
    stroke_rounded_rect(frame, 64, y - 18, 232, 30, 13, outline);

    draw_text(frame, 82, y + 2, &settings_clip(label, 7), WHITE, 1);
    draw_text(frame, 166, y + 2, &settings_clip(value, 15), value_color, 1);
}

pub(crate) fn draw_settings_network_detail(frame: &mut [u16], model: &AppState) {
    let ssid = model.wifi_ssid_label();
    let aps = model.wifi_ap_count_label();
    draw_settings_detail_template(
        frame,
        model,
        "NETWORK",
        SettingsIcon::Wifi,
        model.wifi_status_label(),
        &[
            SettingsDetailRow {
                label: "SSID",
                value: ssid.as_str(),
                color: ACCENT_SETTINGS,
            },
            SettingsDetailRow {
                label: "APS",
                value: aps.as_str(),
                color: ACCENT_SETTINGS,
            },
            SettingsDetailRow {
                label: "STATE",
                value: model.settings.wifi_provisioning_label(),
                color: ACCENT_SETTINGS,
            },
            SettingsDetailRow {
                label: "ERR",
                value: model.settings.wifi_error_label(),
                color: MUTED,
            },
        ],
        "TAP IMPORT WIFI.TXT",
    );
}

pub(crate) fn draw_settings_weather_detail(frame: &mut [u16], model: &AppState) {
    let temp = format!(
        "{} {}",
        model.weather.temperature_label(),
        model.weather.condition_label()
    );
    draw_settings_detail_template(
        frame,
        model,
        "WEATHER",
        SettingsIcon::Weather,
        model.weather.location_label(),
        &[
            SettingsDetailRow {
                label: "TEMP",
                value: temp.as_str(),
                color: ACCENT_WEATHER,
            },
            SettingsDetailRow {
                label: "STATUS",
                value: model.weather.status_label(),
                color: ACCENT_SETTINGS,
            },
            SettingsDetailRow {
                label: "UNITS",
                value: model.weather.units.suffix(),
                color: ACCENT_SETTINGS,
            },
            SettingsDetailRow {
                label: "CACHE",
                value: model.weather.last_error_label(),
                color: MUTED,
            },
        ],
        "TAP CYCLE + FETCH",
    );
}

pub(crate) fn draw_settings_time_detail(frame: &mut [u16], model: &AppState) {
    let time = model.rtc_hms_full();
    let date = model.rtc_ymd();
    draw_settings_detail_template(
        frame,
        model,
        "TIME",
        SettingsIcon::Time,
        time.as_str(),
        &[
            SettingsDetailRow {
                label: "DATE",
                value: date.as_str(),
                color: ACCENT_SETTINGS,
            },
            SettingsDetailRow {
                label: "SNTP",
                value: model.time_sync_label(),
                color: ACCENT_SETTINGS,
            },
            SettingsDetailRow {
                label: "SRC",
                value: model.time_source_label(),
                color: ACCENT_SETTINGS,
            },
            SettingsDetailRow {
                label: "ERR",
                value: model.time_error_label(),
                color: MUTED,
            },
        ],
        "RTC + SNTP",
    );
}

pub(crate) fn draw_settings_display_detail(frame: &mut [u16], model: &AppState) {
    let brightness = model.settings.brightness_label();
    let quiet = if model.settings.quiet_render_enabled {
        "ON"
    } else {
        "OFF"
    };
    draw_settings_detail_template(
        frame,
        model,
        "DISPLAY",
        SettingsIcon::Display,
        "SLEEP NOW",
        &[
            SettingsDetailRow {
                label: "SLEEP",
                value: "NOW",
                color: ACCENT_SETTINGS,
            },
            SettingsDetailRow {
                label: "WAKE",
                value: "TOUCH",
                color: ACCENT_SETTINGS,
            },
            SettingsDetailRow {
                label: "BRIGHT",
                value: brightness.as_str(),
                color: MUTED,
            },
            SettingsDetailRow {
                label: "QUIET",
                value: quiet,
                color: MUTED,
            },
        ],
        "TAP SLEEP",
    );
}

pub(crate) fn draw_settings_sound_detail(frame: &mut [u16], model: &AppState) {
    let volume = model.settings.volume_label();
    draw_settings_detail_template(
        frame,
        model,
        "SOUND",
        SettingsIcon::Sound,
        volume.as_str(),
        &[
            SettingsDetailRow {
                label: "VOLUME",
                value: volume.as_str(),
                color: ACCENT_SETTINGS,
            },
            SettingsDetailRow {
                label: "PCM",
                value: "I2S",
                color: ACCENT_SETTINGS,
            },
            SettingsDetailRow {
                label: "PINS",
                value: "48/38/47",
                color: MUTED,
            },
            SettingsDetailRow {
                label: "ROUTE",
                value: "SPEAKER",
                color: MUTED,
            },
        ],
        "TAP VOLUME",
    );
}

pub(crate) fn draw_settings_storage_detail(frame: &mut [u16], model: &AppState) {
    // RAW-R42-STORAGE-DETAIL-VIDEO-PREVIEW-REMOVED
    let sd = model.sd_text();
    let free_total = model.sd_free_total_text();

    draw_settings_detail_template(
        frame,
        model,
        "STORAGE",
        SettingsIcon::Storage,
        sd.as_str(),
        &[
            SettingsDetailRow {
                label: "SPACE",
                value: free_total.as_str(),
                color: ACCENT_SETTINGS,
            },
            SettingsDetailRow {
                label: "CARD",
                value: sd.as_str(),
                color: ACCENT_SETTINGS,
            },
        ],
        "SD FREE / TOTAL",
    );
}

pub(crate) fn draw_settings_device_detail(frame: &mut [u16], model: &AppState) {
    let primary = model.battery_adc_path_text();
    let source = model.battery_adc_source_text();
    let voltage = model.battery_voltage_text();
    let adc = model.battery_adc_text();
    let batt = model.battery_percent_detail_text();
    draw_settings_detail_template(
        frame,
        model,
        "DEVICE",
        SettingsIcon::Device,
        primary,
        &[
            SettingsDetailRow {
                label: "BATT",
                value: batt.as_str(),
                color: ACCENT_SETTINGS,
            },
            SettingsDetailRow {
                label: "SOURCE",
                value: source,
                color: ACCENT_SETTINGS,
            },
            SettingsDetailRow {
                label: "ADC",
                value: adc.as_str(),
                color: MUTED,
            },
            SettingsDetailRow {
                label: "VOLT",
                value: voltage.as_str(),
                color: MUTED,
            },
        ],
        &model.battery_cal_text(),
    );
}

pub(crate) fn draw_settings_diagnostics_detail(frame: &mut [u16], model: &AppState) {
    let touch = model.touch_count_text();
    let nav = model.nav_count_text();
    let button = model.button_count_text();
    draw_settings_detail_template(
        frame,
        model,
        "DIAG",
        SettingsIcon::Diagnostics,
        model.last_action,
        &[
            SettingsDetailRow {
                label: "TOUCH",
                value: touch.as_str(),
                color: ACCENT_SETTINGS,
            },
            SettingsDetailRow {
                label: "NAV",
                value: nav.as_str(),
                color: ACCENT_SETTINGS,
            },
            SettingsDetailRow {
                label: "BTN",
                value: button.as_str(),
                color: MUTED,
            },
            SettingsDetailRow {
                label: "I2C",
                value: if model.i2c_ok { "OK" } else { "ERR" },
                color: MUTED,
            },
        ],
        "LAST ACTION",
    );
}

pub(crate) fn settings_clip(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if out.chars().count() >= max_chars {
            break;
        }
        if ch.is_control() {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out
}

pub(crate) fn draw_settings_percent_bar(
    frame: &mut [u16],
    x: i32,
    y: i32,
    w: i32,
    percent: u8,
    accent: u16,
) {
    let pct = percent.min(100) as i32;
    let fill_w = ((w * pct) / 100).clamp(0, w);
    fill_rounded_rect(frame, x, y, w, 8, 4, RING_DIM);
    fill_rounded_rect(frame, x, y, fill_w, 8, 4, accent);
    fill_circle(frame, x + fill_w, y + 4, 8, WHITE);
}

pub(crate) fn draw_settings_row(frame: &mut [u16], y: i32, title: &str, icon: u8) {
    let icon_color = match icon {
        0 => MUTED,
        1 => ACCENT_SETTINGS,
        2 => SOFT,
        _ => RING,
    };
    fill_circle(frame, 90, y, 18, icon_color);
    match icon {
        0 => draw_sun_icon(frame, 90, y, 7, WHITE),
        1 => draw_wifi_icon(frame, 90, y, WHITE),
        2 => draw_bell_icon(frame, 90, y, WHITE),
        _ => draw_mini_gear(frame, 90, y, WHITE),
    }
    draw_text(frame, 124, y + 5, title, WHITE, 2);
}

// v0.1.22-r1 compile repair: Settings Details Hub renderer helpers.
// These were accidentally removed when the v0.1.22 Settings renderer block
// replaced the previous Settings Option A detail implementation.
pub(crate) fn draw_settings_chevron(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    draw_line(frame, cx - 5, cy - 8, cx + 5, cy, color);
    draw_line(frame, cx + 5, cy, cx - 5, cy + 8, color);
}

pub(crate) fn draw_settings_sun_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    stroke_circle(frame, cx, cy, 9, color);
    draw_line(frame, cx, cy - 16, cx, cy - 12, color);
    draw_line(frame, cx, cy + 12, cx, cy + 16, color);
    draw_line(frame, cx - 16, cy, cx - 12, cy, color);
    draw_line(frame, cx + 12, cy, cx + 16, cy, color);
    draw_line(frame, cx - 11, cy - 11, cx - 8, cy - 8, color);
    draw_line(frame, cx + 8, cy + 8, cx + 11, cy + 11, color);
    draw_line(frame, cx + 8, cy - 8, cx + 11, cy - 11, color);
    draw_line(frame, cx - 11, cy + 11, cx - 8, cy + 8, color);
}

pub(crate) fn draw_settings_sound_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    fill_rect(frame, cx - 13, cy - 6, 6, 12, color);
    draw_line(frame, cx - 7, cy - 8, cx + 1, cy - 14, color);
    draw_line(frame, cx - 7, cy + 8, cx + 1, cy + 14, color);
    draw_line(frame, cx + 1, cy - 14, cx + 1, cy + 14, color);
    draw_arc_segment(frame, cx + 3, cy, 8, 1, 312, 48, color);
    draw_arc_segment(frame, cx + 4, cy, 14, 1, 305, 55, color);
}

pub(crate) fn draw_settings_info_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    stroke_circle(frame, cx, cy, 14, color);
    fill_circle(frame, cx, cy - 7, 2, color);
    fill_rect(frame, cx - 1, cy - 2, 2, 11, color);
    fill_rect(frame, cx - 3, cy + 8, 6, 2, color);
}

// v0.1.22-r4 compile repair: remaining Settings detail icon helpers.
pub(crate) fn draw_settings_time_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    stroke_circle(frame, cx, cy, 15, color);
    draw_line(frame, cx, cy, cx, cy - 9, color);
    draw_line(frame, cx, cy, cx + 8, cy + 5, color);
}

pub(crate) fn draw_settings_storage_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    stroke_rect(frame, cx - 8, cy - 11, 16, 22, color);
    fill_rect(frame, cx - 5, cy - 7, 3, 4, color);
    fill_rect(frame, cx, cy - 7, 3, 4, color);
    draw_line(frame, cx - 5, cy + 6, cx + 5, cy + 6, color);
}

pub(crate) fn draw_settings_diag_icon(frame: &mut [u16], cx: i32, cy: i32, color: u16) {
    stroke_rect(frame, cx - 10, cy - 10, 20, 20, color);
    draw_line(frame, cx - 6, cy - 3, cx - 1, cy + 4, color);
    draw_line(frame, cx - 1, cy + 4, cx + 8, cy - 6, color);
}

// RAW-R48-SETTINGS-ASSISTANT-MODULE-CALLSITE

// RAW-R52-R1-SETTINGS-TOUCH-ROUTER-CALLSITE
