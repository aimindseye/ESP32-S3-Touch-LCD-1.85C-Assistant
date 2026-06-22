// RAW-R52-TOUCH-HANDLER-CLEANUP
// Touch classification and action routing glue extracted after r51-r1.
// This module intentionally imports crate-root helpers to preserve accepted runtime behavior.
use crate::*;

pub(crate) fn process_touch_summary(
    model: &mut AppState,
    providers: &LocalProviders,
    wifi: &mut EspWifi<'static>,
    wifi_connect_deadline: &mut Option<Instant>,
    summary: TouchSummary,
    now: Instant,
    last_navigation: &mut Instant,
) -> bool {
    // RAW-R56-R2-WEATHER-NAV-ROW-CALLSITE
    if crate::weather_action_router::maybe_handle_weather_nav_row_touch(model, providers) {
        return true;
    }

    if log_debug_enabled() {
        debug_println!(
            "touch-track: finish id={} reason={} samples={} start=({}, {}) end=({}, {}) minmax=({}, {})..({}, {}) span={} dx={} dy={} ms={} gesture=0x{:02X}",
            summary.touch_id,
            summary.finish_reason,
            summary.sample_count,
            summary.start_x,
            summary.start_y,
            summary.end_x,
            summary.end_y,
            summary.min_x,
            summary.min_y,
            summary.max_x,
            summary.max_y,
            summary.span_x,
            summary.dx,
            summary.dy,
            summary.duration_ms,
            summary.gesture
        );
    } else {
        println!(
            "touch: id={} kind={} page={:?} end=({}, {}) ms={} gesture=0x{:02X}",
            summary.touch_id,
            compact_touch_kind(&summary),
            model.current_page,
            summary.end_x,
            summary.end_y,
            summary.duration_ms,
            summary.gesture
        );
    }

    if let Some(intent) =
        screens::settings::settings_detail_intent_from_touch_summary(model, &summary)
    {
        crate::page_orchestration::handle_intent(
            model,
            providers,
            wifi,
            wifi_connect_deadline,
            intent,
        );
        return true;
    }

    if crate::media_action_router::audio_r33_handle_music_touch(model, &summary) {
        return true;
    }

    if let Some(intent) = intent_from_touch_summary(&summary) {
        if matches!(intent, UiIntent::NextPage | UiIntent::PreviousPage) {
            if now.duration_since(*last_navigation) < Duration::from_millis(TOUCH_NAV_COOLDOWN_MS) {
                debug_println!("touch: navigation ignored during cooldown");
                return false;
            }
            *last_navigation = now;
        }

        crate::page_orchestration::handle_intent(
            model,
            providers,
            wifi,
            wifi_connect_deadline,
            intent,
        );
        true
    } else {
        debug_println!("touch: ignored movement/tap outside classifier thresholds");
        false
    }
}

pub(crate) fn compact_touch_kind(summary: &TouchSummary) -> &'static str {
    if summary.span_x <= CENTER_TAP_MAX_MOVE_PX
        && summary.span_y <= CENTER_TAP_MAX_MOVE_PX
        && summary.duration_ms <= TOUCH_TAP_MAX_MS as u128
    {
        return "tap";
    }

    let abs_span_x = summary.span_x.abs();
    let abs_span_y = summary.span_y.abs();

    if abs_span_x >= UNIVERSAL_SWIPE_MIN_DX && abs_span_x >= abs_span_y {
        if summary.dx < 0 {
            "swipe-left"
        } else {
            "swipe-right"
        }
    } else if abs_span_y >= UNIVERSAL_SWIPE_MIN_DX {
        if summary.dy < 0 {
            "swipe-up"
        } else {
            "swipe-down"
        }
    } else {
        "touch"
    }
}

pub(crate) fn intent_from_touch_summary(summary: &TouchSummary) -> Option<UiIntent> {
    let left_travel = summary.start_x as i16 - summary.min_x as i16;
    let right_travel = summary.max_x as i16 - summary.start_x as i16;
    let signed_span_dx = if left_travel >= right_travel {
        -left_travel
    } else {
        right_travel
    };
    let abs_span_dx = signed_span_dx.abs();
    let abs_span_y = summary.span_y.abs();
    let horizontal_dominant = (abs_span_dx as i32 * 2) > (abs_span_y as i32 * 3);

    let gesture_intent = match summary.gesture {
        CST816_GESTURE_LEFT => Some(UiIntent::NextPage),
        CST816_GESTURE_RIGHT => Some(UiIntent::PreviousPage),
        _ => None,
    };

    let span_intent = if abs_span_dx >= UNIVERSAL_SWIPE_MIN_DX && horizontal_dominant {
        if signed_span_dx < 0 {
            Some(UiIntent::NextPage)
        } else {
            Some(UiIntent::PreviousPage)
        }
    } else {
        None
    };

    if let Some(gesture_intent) = gesture_intent {
        if let Some(span_intent) = span_intent {
            if gesture_intent != span_intent {
                println!(
                    "touch-class: gesture/span disagree gesture=0x{:02X} span_dx={} span={} prefer={}",
                    summary.gesture,
                    signed_span_dx,
                    summary.span_x,
                    if abs_span_dx < TOUCH_GESTURE_SPAN_PREFER_PX {
                        "gesture"
                    } else {
                        "span"
                    }
                );

                if abs_span_dx >= TOUCH_GESTURE_SPAN_PREFER_PX {
                    return log_span_intent(span_intent);
                }
            }
        }

        return log_gesture_intent(gesture_intent);
    }

    if let Some(span_intent) = span_intent {
        return log_span_intent(span_intent);
    }

    let center_tap = summary.span_x <= CENTER_TAP_MAX_MOVE_PX
        && summary.span_y <= CENTER_TAP_MAX_MOVE_PX
        && summary.duration_ms <= TOUCH_TAP_MAX_MS as u128
        && (CENTER_TAP_X_MIN..=CENTER_TAP_X_MAX).contains(&summary.end_x)
        && (CENTER_TAP_Y_MIN..=CENTER_TAP_Y_MAX).contains(&summary.end_y);

    if center_tap {
        debug_println!("touch-class: center-tap accepted");
        return Some(UiIntent::Select);
    }

    if summary.sample_count < 2 {
        debug_println!(
            "touch-class: ignored insufficient samples samples={} dx={} dy={} span={} gesture=0x{:02X}",
            summary.sample_count, summary.dx, summary.dy, summary.span_x, summary.gesture
        );
        return None;
    }

    if abs_span_y >= UNIVERSAL_SWIPE_MIN_DX && abs_span_y > abs_span_dx {
        debug_println!(
            "touch-class: ignored vertical swipe dx={} dy={} span={} gesture=0x{:02X}",
            summary.dx,
            summary.dy,
            summary.span_x,
            summary.gesture
        );
        return None;
    }

    debug_println!(
        "touch-class: ignored below-threshold movement dx={} dy={} span={} gesture=0x{:02X}",
        summary.dx,
        summary.dy,
        summary.span_x,
        summary.gesture
    );
    None
}

pub(crate) fn log_gesture_intent(intent: UiIntent) -> Option<UiIntent> {
    match intent {
        UiIntent::NextPage => println!("touch-class: gesture-left accepted next"),
        UiIntent::PreviousPage => println!("touch-class: gesture-right accepted previous"),
        _ => {}
    }

    Some(intent)
}

// v0.1.33-r1 source-marker: existing_music_screen_audio_controls screen=Music path=/AUDIO controls=LEFT_PREV,CENTER_PLAY_STOP,RIGHT_NEXT wav_pcm=SCREEN_STATE_ONLY mp3_decode=DISABLED output=HARDWARE_GATED_OR_I2S_PENDING video_audio=DEFERRED
pub(crate) fn audio_r33_is_tap(summary: &TouchSummary) -> bool {
    summary.span_x <= CENTER_TAP_MAX_MOVE_PX
        && summary.span_y <= CENTER_TAP_MAX_MOVE_PX
        && summary.duration_ms <= TOUCH_TAP_MAX_MS as u128
}

// RAW-R52-MOVED-TOUCH-FUNCTIONS: process_touch_summary, compact_touch_kind, is_settings_detail_header_tap, intent_from_touch_summary, log_gesture_intent, audio_r33_is_tap, audio_r33_radio_center_action, audio_r33_music_center_action, media_touch_from_summary, audio_r33_handle_music_touch

// RAW-R52-R1-TOUCH-HELPERS-EXPORTED

// RAW-R56-R1-WEATHER-ACTION-FALLBACK-NO-TOUCH-BLOCKS

// RAW-R56-R2-WEATHER-NAV-ROW-CALLSITE-MARKER
