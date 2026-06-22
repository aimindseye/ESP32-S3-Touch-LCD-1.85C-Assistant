// RAW-R47-MUSIC-TRUE-SCREEN-MODULE
// True Rust module for the Music screen renderer.
// Uses crate-root helpers/constants while preserving r46-r1 behavior.
use crate::*;

// RAW-R44-SCREEN-MUSIC-RENDERER-SPLIT
// Included from main.rs; kept in crate-root item namespace intentionally.

pub(crate) fn draw_music_tile(model: &AppState, frame: &mut [u16]) {
    // v0.1.36-r36: module-based radio-style Music renderer.
    // The module clears the old MUSIC.RGB visual shell and keeps accepted
    // playback/progress/control behavior intact.
    music_screen::draw(model, frame);
}

pub(crate) fn draw_music_transport_controls(frame: &mut [u16], playing: bool) {
    // v0.1.36-r35: five visibly separated media controls.  The touch zones
    // live in media_controls.rs and match these displayed button centers.
    // RAW-R35-DRAW-FIVE-MEDIA-CONTROL-ZONES
    let side_y = media_controls::CONTROL_Y;

    fill_circle(
        frame,
        media_controls::VOL_DOWN_X,
        side_y,
        media_controls::SIDE_BUTTON_R,
        BG_DARK,
    );
    stroke_circle(
        frame,
        media_controls::VOL_DOWN_X,
        side_y,
        media_controls::SIDE_BUTTON_R,
        ACCENT_MUSIC_BLUE,
    );
    draw_text_centered_at(
        frame,
        media_controls::VOL_DOWN_X,
        side_y + 3,
        "VOL-",
        WHITE,
        1,
    );

    fill_circle(
        frame,
        media_controls::PREV_X,
        side_y,
        media_controls::SIDE_BUTTON_R,
        BG_DARK,
    );
    stroke_circle(
        frame,
        media_controls::PREV_X,
        side_y,
        media_controls::SIDE_BUTTON_R,
        ACCENT_MUSIC_BLUE,
    );
    draw_skip_icon(frame, media_controls::PREV_X, side_y, false, WHITE);

    fill_circle(
        frame,
        media_controls::NEXT_X,
        side_y,
        media_controls::SIDE_BUTTON_R,
        BG_DARK,
    );
    stroke_circle(
        frame,
        media_controls::NEXT_X,
        side_y,
        media_controls::SIDE_BUTTON_R,
        ACCENT_MUSIC_BLUE,
    );
    draw_skip_icon(frame, media_controls::NEXT_X, side_y, true, WHITE);

    fill_circle(
        frame,
        media_controls::VOL_UP_X,
        side_y,
        media_controls::SIDE_BUTTON_R,
        BG_DARK,
    );
    stroke_circle(
        frame,
        media_controls::VOL_UP_X,
        side_y,
        media_controls::SIDE_BUTTON_R,
        ACCENT_MUSIC_BLUE,
    );
    draw_text_centered_at(
        frame,
        media_controls::VOL_UP_X,
        side_y + 3,
        "VOL+",
        WHITE,
        1,
    );

    // Center button: primary play/stop control.
    fill_circle(
        frame,
        media_controls::PLAY_X,
        media_controls::CENTER_CONTROL_Y,
        media_controls::CENTER_BUTTON_R,
        BG_DARK,
    );
    stroke_circle(
        frame,
        media_controls::PLAY_X,
        media_controls::CENTER_CONTROL_Y,
        media_controls::CENTER_BUTTON_R + 1,
        ACCENT_MUSIC_BLUE,
    );
    stroke_circle(
        frame,
        media_controls::PLAY_X,
        media_controls::CENTER_CONTROL_Y,
        media_controls::CENTER_BUTTON_R - 5,
        RING_DIM,
    );

    if playing {
        fill_rounded_rect(
            frame,
            media_controls::PLAY_X - 10,
            media_controls::CENTER_CONTROL_Y - 21,
            8,
            42,
            4,
            WHITE,
        );
        fill_rounded_rect(
            frame,
            media_controls::PLAY_X + 6,
            media_controls::CENTER_CONTROL_Y - 21,
            8,
            42,
            4,
            WHITE,
        );
    } else {
        fill_play_triangle(
            frame,
            media_controls::PLAY_X + 3,
            media_controls::CENTER_CONTROL_Y,
            40,
            WHITE,
        );
    }
}

pub(crate) fn draw_music_progress_row(
    frame: &mut [u16],
    progress: u8,
    elapsed: &str,
    duration: &str,
) {
    draw_text_centered_at(frame, 77, 288, elapsed, ACCENT_MUSIC_BLUE, 2);
    draw_text_centered_at(frame, 285, 288, duration, WHITE, 2);

    let track_x = 112;
    let track_y = 281;
    let track_w = 136;
    fill_rounded_rect(frame, track_x, track_y, track_w, 7, 3, RING_DIM);

    let fill_w = ((track_w as u16 * progress.min(100) as u16) / 100) as i32;
    fill_rounded_rect(
        frame,
        track_x,
        track_y,
        fill_w.max(6),
        7,
        3,
        ACCENT_MUSIC_BLUE,
    );
    fill_circle(
        frame,
        track_x + fill_w.max(6),
        track_y + 3,
        7,
        ACCENT_MUSIC_BLUE,
    );
}

// RAW-R47-R1-MUSIC-SHARED-HELPERS-EXPORTED
