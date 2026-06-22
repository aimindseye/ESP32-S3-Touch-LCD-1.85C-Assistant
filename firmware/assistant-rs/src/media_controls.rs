//! Shared touch-zone classifier for Music and Internet Radio controls.
//!
//! v0.1.36-r35: keep media controls visually separated and classify taps
//! against the same zones that are drawn on screen.  This module is pure
//! coordinate logic so future screen/layout changes can update one place.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MediaControlAction {
    VolumeDown,
    Previous,
    PlayStop,
    Next,
    VolumeUp,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MediaTouch {
    pub(crate) end_x: u16,
    pub(crate) end_y: u16,
    pub(crate) span_x: i16,
    pub(crate) span_y: i16,
    pub(crate) dx: i16,
    pub(crate) dy: i16,
    pub(crate) duration_ms: u128,
    pub(crate) gesture: u8,
}

pub(crate) const CONTROL_Y: i32 = 214;
pub(crate) const CENTER_CONTROL_Y: i32 = 216;
pub(crate) const SIDE_BUTTON_R: i32 = 23;
pub(crate) const CENTER_BUTTON_R: i32 = 40;
pub(crate) const VOL_DOWN_X: i32 = 35;
pub(crate) const PREV_X: i32 = 92;
pub(crate) const PLAY_X: i32 = 180;
pub(crate) const NEXT_X: i32 = 268;
pub(crate) const VOL_UP_X: i32 = 325;

const TAP_MAX_MS: u128 = 650;
const TAP_MAX_SPAN_PX: i16 = 28;
const CLEAR_SWIPE_DX_PX: i16 = 38;
const CONTROL_Y_MIN: u16 = 172;
const CONTROL_Y_MAX: u16 = 266;

// Non-overlapping hit bands. These correspond to the displayed circles.
const VOL_DOWN_X_MIN: u16 = 0;
const VOL_DOWN_X_MAX: u16 = 64;
const PREV_X_MIN: u16 = 66;
const PREV_X_MAX: u16 = 126;
const PLAY_X_MIN: u16 = 136;
const PLAY_X_MAX: u16 = 224;
const NEXT_X_MIN: u16 = 234;
const NEXT_X_MAX: u16 = 294;
const VOL_UP_X_MIN: u16 = 296;
const VOL_UP_X_MAX: u16 = 359;

pub(crate) fn action_from_touch(touch: MediaTouch) -> Option<MediaControlAction> {
    if !is_media_control_tap(touch) || !(CONTROL_Y_MIN..=CONTROL_Y_MAX).contains(&touch.end_y) {
        return None;
    }

    match touch.end_x {
        VOL_DOWN_X_MIN..=VOL_DOWN_X_MAX => Some(MediaControlAction::VolumeDown),
        PREV_X_MIN..=PREV_X_MAX => Some(MediaControlAction::Previous),
        PLAY_X_MIN..=PLAY_X_MAX => Some(MediaControlAction::PlayStop),
        NEXT_X_MIN..=NEXT_X_MAX => Some(MediaControlAction::Next),
        VOL_UP_X_MIN..=VOL_UP_X_MAX => Some(MediaControlAction::VolumeUp),
        _ => None,
    }
}

pub(crate) fn action_label(action: MediaControlAction) -> &'static str {
    match action {
        MediaControlAction::VolumeDown => "VOL_DOWN",
        MediaControlAction::Previous => "PREV",
        MediaControlAction::PlayStop => "PLAY_STOP",
        MediaControlAction::Next => "NEXT",
        MediaControlAction::VolumeUp => "VOL_UP",
    }
}

fn is_media_control_tap(touch: MediaTouch) -> bool {
    if touch.duration_ms > TAP_MAX_MS {
        return false;
    }

    if touch.span_x.abs() > TAP_MAX_SPAN_PX || touch.span_y.abs() > TAP_MAX_SPAN_PX {
        return false;
    }

    let abs_dx = touch.dx.abs();
    let abs_dy = touch.dy.abs();
    let clear_horizontal_swipe = abs_dx >= CLEAR_SWIPE_DX_PX && abs_dx > abs_dy.saturating_mul(2);
    if clear_horizontal_swipe {
        return false;
    }

    // CST816 left/right gesture IDs.  Ignore gesture-only swipe reports unless
    // the measured movement stayed within the relaxed tap window.
    if matches!(touch.gesture, 0x03 | 0x04) && abs_dx >= 20 {
        return false;
    }

    true
}

// RAW-R35-MEDIA-CONTROL-ZONES
