// RAW-R48-ASSISTANT-TRUE-SCREEN-MODULE
// True Rust module for the Assistant screen renderer.
// Uses crate-root helpers/constants while preserving r47-r1 behavior.
use crate::*;

// RAW-R44-SCREEN-ASSISTANT-RENDERER-SPLIT
// Included from main.rs; kept in crate-root item namespace intentionally.

pub(crate) fn draw_assistant_page(
    model: &AppState,
    frame: &mut [u16],
    asset_cache: &mut UiAssetCache,
) -> Result<()> {
    let page = model.current_page;
    crate::page_assets::draw_cached_page_base(asset_cache, frame, page);

    match page {
        AssistantPage::Home => crate::screens::home::draw_home_tile(model, frame),
        AssistantPage::Weather => crate::screens::weather::draw_weather_tile(model, frame),
        AssistantPage::Music => crate::screens::music::draw_music_tile(model, frame),
        AssistantPage::InternetRadio => draw_internet_radio_screen(model, frame),
        AssistantPage::Assistant => draw_ai_assistant_tile(model, frame),
        AssistantPage::Settings => crate::screens::settings::draw_settings_tile(model, frame),
    }

    draw_page_dots(model, frame);

    if !unsafe { ffi::st77916_panel_draw_rgb565(0, 0, 359, 359, frame.as_mut_ptr()) } {
        bail!("st77916_panel_draw_rgb565 returned false");
    }

    Ok(())
}

pub(crate) fn draw_ai_assistant_tile(model: &AppState, frame: &mut [u16]) {
    // v0.1.12 AI Assistant Option B Conversation Card.
    // Base asset provides the clean card shell, lower control layout, and subtle screen frame.
    // Runtime overlays keep listening state, message, timestamp, and mic state live.
    draw_waveform(
        frame,
        180,
        70,
        if model.assistant.listening { 22 } else { 14 },
        ACCENT_ASSISTANT,
    );
    draw_text_centered_at(frame, 180, 112, model.assistant.title_label(), WHITE, 3);
    draw_text_centered_at(frame, 180, 140, model.assistant.subtitle_label(), MUTED, 2);

    draw_assistant_robot_badge(frame, 91, 190, model.assistant.listening);
    draw_text(frame, 126, 178, model.assistant.card_label(), WHITE, 2);
    draw_text(frame, 126, 204, model.assistant.card_aux_label(), MUTED, 1);

    draw_microphone_button(frame, 180, 272, model.assistant.listening);
    draw_cancel_glyph(frame, 116, 272, MUTED);
}

pub(crate) fn draw_assistant_robot_badge(frame: &mut [u16], cx: i32, cy: i32, listening: bool) {
    let outer = if listening {
        ACCENT_ASSISTANT
    } else {
        RING_DIM
    };
    fill_circle(frame, cx, cy, 25, 0x192A);
    stroke_circle(frame, cx, cy, 25, outer);

    fill_rounded_rect(frame, cx - 15, cy - 11, 30, 22, 8, BG_DARK);
    stroke_rect(frame, cx - 15, cy - 11, 30, 22, WHITE);
    fill_circle(frame, cx - 7, cy, 3, ACCENT_ASSISTANT);
    fill_circle(frame, cx + 7, cy, 3, ACCENT_ASSISTANT);
    draw_line(frame, cx, cy - 16, cx, cy - 12, WHITE);
    fill_circle(frame, cx, cy - 18, 2, WHITE);
}

pub(crate) fn draw_assistant_orb(frame: &mut [u16], cx: i32, cy: i32, r: i32, color: u16) {
    fill_circle(frame, cx, cy, r, BG_DARK);
    stroke_circle(frame, cx, cy, r, color);
    stroke_circle(frame, cx, cy, r - 8, SOFT);
    fill_circle(frame, cx, cy, 4, color);
}

// RAW-R45-R1-ASSISTANT-HOME-MODULE-CALLSITE

// RAW-R46-R1-ASSISTANT-WEATHER-MODULE-CALLSITE

// RAW-R47-ASSISTANT-MUSIC-MODULE-CALLSITE

// RAW-R48-R1-ASSISTANT-SHARED-HELPERS-EXPORTED

// RAW-R51-R1-ASSISTANT-CACHED-BASE-CALLSITE

// RAW-R53-ASSISTANT-PAGE-ASSETS-CALLSITE
