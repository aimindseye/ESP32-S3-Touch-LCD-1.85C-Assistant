// RAW-R55-SETTINGS-ACTION-CLEANUP
// Focused Settings action/detail helpers extracted after r54.
// This module intentionally imports crate-root helpers to preserve accepted Settings behavior.
use crate::*;

pub(crate) fn is_settings_detail_header_tap(summary: &TouchSummary) -> bool {
    summary.span_x <= CENTER_TAP_MAX_MOVE_PX
        && summary.span_y <= CENTER_TAP_MAX_MOVE_PX
        && summary.duration_ms <= TOUCH_TAP_MAX_MS as u128
        && summary.end_y <= SETTINGS_DETAIL_HEADER_TAP_Y_MAX
}

// RAW-R55-MOVED-SETTINGS-ACTION-FUNCTIONS: is_settings_detail_header_tap
