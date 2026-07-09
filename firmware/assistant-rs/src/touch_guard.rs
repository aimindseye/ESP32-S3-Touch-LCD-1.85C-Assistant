// RAW-V1-0-1-TOUCH-GHOST-GUARD
// Conservative touch noise guard for ESP32-S3-Touch-LCD-1.85C.
//
// Goals:
// - Preserve accepted v1.0.0 gestures and taps.
// - Reject obvious short phantom taps.
// - Add cooldown on Weather/Settings, where phantom scroll/page actions were observed.
// - Keep DEBUG-only rejection logs through debug_println!.

use crate::log_debug_enabled;
use crate::{AppState, AssistantPage};
use core::sync::atomic::{AtomicU32, Ordering};

const MIN_TOUCH_DURATION_MS: u32 = 45;
const WEATHER_SETTINGS_COOLDOWN_MS: u32 = 360;
const GENERAL_COOLDOWN_MS: u32 = 120;
const EDGE_NOISE_BAND_PX: i16 = 4;

static LAST_ACCEPTED_TOUCH_MS: AtomicU32 = AtomicU32::new(0);

fn now_ms() -> u32 {
    // ESP-IDF monotonic timer, microseconds since boot.
    // The wrap behavior is intentional; wrapping_sub handles it.
    let micros = unsafe { esp_idf_svc::sys::esp_timer_get_time() };
    (micros / 1000) as u32
}

fn page_cooldown_ms(page: &AssistantPage) -> u32 {
    if matches!(page, AssistantPage::Weather | AssistantPage::Settings) {
        WEATHER_SETTINGS_COOLDOWN_MS
    } else {
        GENERAL_COOLDOWN_MS
    }
}

pub(crate) fn reject_touch_ghost(model: &AppState) -> bool {
    let Some(touch) = model.last_touch else {
        return false;
    };

    let x = touch.x as i16;
    let y = touch.y as i16;
    let duration_ms: u32 = 0; // TouchSnapshot has no duration field in v1.0.1-r2

    // RAW-V1-0-1-R2-FINGERS-GHOST-REJECT
    if touch.fingers == 0 {
        debug_println!(
            "touch-guard: reject reason=short-touch page={:?} x={} y={} ms={} fingers={}",
            model.current_page,
            x,
            y,
            duration_ms,
            touch.fingers
        );
        return true;
    }

    let now = now_ms();

    if duration_ms > 0 && duration_ms < MIN_TOUCH_DURATION_MS {
        debug_println!(
            "touch-guard: reject reason=short-touch page={:?} x={} y={} ms={}",
            model.current_page,
            x,
            y,
            duration_ms
        );
        return true;
    }

    if x <= EDGE_NOISE_BAND_PX || y <= EDGE_NOISE_BAND_PX {
        debug_println!(
            "touch-guard: reject reason=edge-noise page={:?} x={} y={} ms={}",
            model.current_page,
            x,
            y,
            duration_ms
        );
        return true;
    }

    let last = LAST_ACCEPTED_TOUCH_MS.load(Ordering::Relaxed);
    let cooldown = page_cooldown_ms(&model.current_page);
    if last != 0 {
        let elapsed = now.wrapping_sub(last);
        if elapsed < cooldown {
            debug_println!(
                "touch-guard: reject reason=cooldown page={:?} x={} y={} ms={} elapsed={} cooldown={}",
                model.current_page,
                x,
                y,
                duration_ms,
                elapsed,
                cooldown
            );
            return true;
        }
    }

    LAST_ACCEPTED_TOUCH_MS.store(now, Ordering::Relaxed);
    false
}
