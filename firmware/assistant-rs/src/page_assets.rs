// RAW-R53-PAGE-ASSETS-CLEANUP
// Cached page base / asset rendering helpers extracted after r52-r1.
// This module intentionally imports crate-root helpers to preserve accepted rendering behavior.
use crate::*;

pub(crate) fn draw_page_fallback_base(frame: &mut [u16], page: AssistantPage) {
    frame.fill(BLACK);
    let accent = match page {
        AssistantPage::Home => ACCENT_HOME,
        AssistantPage::Weather => ACCENT_WEATHER,
        AssistantPage::Music => ACCENT_MUSIC,
        AssistantPage::InternetRadio => ACCENT_MUSIC,
        AssistantPage::Assistant => ACCENT_ASSISTANT,
        AssistantPage::Settings => ACCENT_SETTINGS,
    };
    fill_circle(frame, CX, CY, R_OUTER - 6, BG);
    stroke_circle(frame, CX, CY, R_OUTER, RING_DIM);
    draw_arc_segment(frame, CX, CY, 170, 2, 220, 320, accent);
    draw_text_centered_at(frame, 180, 70, page.title(), accent, 2);
    draw_text_centered_at(frame, 180, 314, "SD ASSET FALLBACK", MUTED, 1);
}

pub(crate) fn draw_cached_page_base(
    asset_cache: &mut UiAssetCache,
    frame: &mut [u16],
    page: AssistantPage,
) {
    // RAW-R39-MEDIA-PAGES-BLACK-BASE
    // Music and InternetRadio now own their clean black media layout, so avoid
    // loading/drawing the old baked MUSIC.RGB background for those pages.
    if matches!(page, AssistantPage::Music | AssistantPage::InternetRadio) {
        frame.fill(BLACK);
        return;
    }

    match asset_cache.ensure_page(page) {
        UiAssetSource::Sd => asset_cache.copy_to(frame),
        UiAssetSource::Fallback => draw_page_fallback_base(frame, page),
    }
}

// RAW-R53-MOVED-PAGE-ASSET-FUNCTIONS: draw_cached_page_base, draw_page_fallback_base
