//! Radio-style Music screen renderer.
//!
//! v0.1.36-r36: Music now uses the same clean black visual language as
//! Internet Radio instead of relying on the older baked MUSIC.RGB background.
//! Playback, progress, volume, and dedicated media touch zones remain owned by
//! the accepted audio and shared-control paths.

pub(crate) fn draw(model: &crate::AppState, frame: &mut [u16]) {
    let audio = crate::audio_foundation::music_screen_snapshot();

    // Full-frame clear removes the older Music RGB visual shell while keeping
    // the accepted runtime transport/progress overlays.
    // RAW-R36-MUSIC-RADIO-STYLE-FULL-CLEAR
    frame.fill(crate::BLACK);

    crate::draw_text_centered_at(frame, 180, 34, &model.rtc_hms(), crate::WHITE, 2);
    crate::draw_text_centered_at(frame, 180, 72, "MUSIC", crate::ACCENT_MUSIC_BLUE, 1);

    // Keep file/track information where the Internet Radio station name lives.
    crate::draw_text_centered_at(frame, 180, 116, &audio.track_label, crate::WHITE, 2);

    // Playback/format status row mirrors the Internet Radio status row.
    crate::draw_text_centered_at(
        frame,
        180,
        150,
        &audio.subtitle_label,
        crate::ACCENT_MUSIC_BLUE,
        2,
    );

    // Reuse the accepted r35 visually separated transport/volume controls.
    crate::draw_music_transport_controls(frame, audio.playing);
    crate::draw_music_progress_row(
        frame,
        audio.progress_percent,
        &audio.elapsed_label,
        &audio.duration_label,
    );

    // Match Internet Radio footer wording so visible controls and touch zones
    // describe the same layout on both screens.
    crate::draw_text_centered_at(frame, 180, 318, "VOL- PREV PLAY NEXT VOL+", crate::MUTED, 1);

    // RAW-R36-MUSIC-RADIO-STYLE-LAYOUT
}
