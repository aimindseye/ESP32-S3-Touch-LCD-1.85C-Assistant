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

    // v1.0.1-r7: Internet Radio has its own centered volume + transport layout.
    // Do not reuse the Music side-edge volume controls here.
    draw_internet_radio_r7_controls(frame, radio.playing);
    crate::screens::music::draw_music_progress_row(
        frame,
        radio.progress_percent,
        &radio.elapsed_label,
        &radio.duration_label,
    );

    crate::draw_text_centered_at(
        frame,
        180,
        318,
        "VOL-/VOL+ ABOVE PREV/PLAY/NEXT",
        RADIO_DIM,
        1,
    );

    // RADIO_R31_R1_DEDICATED_INTERNET_RADIO_SCREEN_MODULE_FULL_NAME
}

fn draw_internet_radio_r7_controls(frame: &mut [u16], playing: bool) {
    let play_label = if playing { "STOP" } else { "PLAY" };

    // v1.0.1-r7-r1: high-contrast centered controls for the round LCD.
    // Keep VOL-/VOL+ above transport, but make PREV/NEXT readable.
    crate::draw_text_centered_at(frame, 102, 190, "VOL-", RADIO_WHITE, 2);
    crate::draw_text_centered_at(frame, 278, 190, "VOL+", RADIO_WHITE, 2);

    // Transport row: PREV/NEXT are intentionally white and size 2.
    crate::draw_text_centered_at(frame, 82, 252, "PREV", RADIO_WHITE, 2);
    crate::draw_text_centered_at(frame, 190, 252, play_label, RADIO_CYAN, 2);
    crate::draw_text_centered_at(frame, 302, 252, "NEXT", RADIO_WHITE, 2);
}

// RADIO_R31_R2_ORPHAN_OVERLAY_FRAGMENT_REMOVED

// RAW-R47-R1-RADIO-SCREEN-MUSIC-MODULE-CALLSITE

// RAW-V1-0-1-R4-RADIO-STREAM-HEADROOM

// RAW-V1-0-1-R7-INTERNET-RADIO-CENTER-VOLUME-LAYOUT

// RAW-V1-0-1-R7-R1-INTERNET-RADIO-READABILITY-REPAIR

// RAW-V1-0-1-R7-R1-READABILITY-COMPAT: RADIO CONTROLS CENTERED
// RAW-V1-0-1-R10-R4-RADIO-STATIC-UI-COMPAT

// RAW-V1-0-1-R11-R2-RADIO-LIVE-UI-REFRESH-SCREEN

// RAW-V1-0-1-R13-RADIO-STATION-IDLE-UI-SCREEN
