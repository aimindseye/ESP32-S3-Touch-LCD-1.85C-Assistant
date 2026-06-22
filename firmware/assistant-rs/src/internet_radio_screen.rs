//! Dedicated Internet Radio screen renderer.
//!
//! v0.1.36-r31-r1: split Internet Radio display from the Music page renderer.
//! The radio backend remains in `internet_radio.rs`; this module owns only the
//! visual screen for the InternetRadio page.

use crate::internet_radio;

const RADIO_BG: u16 = 0x0000;
const RADIO_WHITE: u16 = 0xffff;
const RADIO_CYAN: u16 = 0x07ff;
const RADIO_BLUE: u16 = 0x041f;
const RADIO_DIM: u16 = 0x39e7;

pub(crate) fn draw(frame: &mut [u16], time_label: &str) {
    let radio = internet_radio::snapshot();

    // Dedicated full-frame clear so no Music title/initials/badge residue remains.
    for px in frame.iter_mut() {
        *px = RADIO_BG;
    }

    crate::draw_text_centered_at(frame, 180, 34, time_label, RADIO_WHITE, 2);
    crate::draw_text_centered_at(frame, 180, 72, "INTERNET RADIO", RADIO_CYAN, 1);

    // Full station name from RADIO.TXT. No compact initials renderer is used here.
    // RADIO_R32_RENDER_SOURCE_LABEL_RAW_STATION_NAME
    // RADIO_R34_STATION_NAME_DRAWS_RAW_LABEL_WITH_LOWERCASE_GLYPHS
    crate::draw_text_centered_at(frame, 180, 116, &radio.source_label, RADIO_WHITE, 2);

    // Playback state/buffer status remains separate.
    crate::draw_text_centered_at(frame, 180, 150, &radio.status_label, RADIO_BLUE, 2);

    // Preserve accepted transport and progress primitives; do not reuse Music title renderer.
    crate::screens::music::draw_music_transport_controls(frame, radio.playing);
    crate::screens::music::draw_music_progress_row(
        frame,
        radio.progress_percent,
        &radio.elapsed_label,
        &radio.duration_label,
    );

    crate::draw_text_centered_at(frame, 180, 318, "VOL- PREV PLAY NEXT VOL+", RADIO_DIM, 1);

    // RADIO_R31_R1_DEDICATED_INTERNET_RADIO_SCREEN_MODULE_FULL_NAME
}

// RADIO_R31_R2_ORPHAN_OVERLAY_FRAGMENT_REMOVED

// RAW-R47-R1-RADIO-SCREEN-MUSIC-MODULE-CALLSITE
