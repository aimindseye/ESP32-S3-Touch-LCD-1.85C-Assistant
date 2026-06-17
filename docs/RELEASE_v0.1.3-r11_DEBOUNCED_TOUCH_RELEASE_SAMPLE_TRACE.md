# v0.1.3-r11 — Debounced Touch Release and Sample-Trace Stabilization

## Goal

r10 proved span-based universal swipe is better, but same-position physical swipes can still be split into stationary or short-movement events. r11 changes the touch tracker lifecycle so a gesture is not finalized on the first INT-high/release indication.

## Touch tracking

- Touch polling reduced to 10ms.
- Release is debounced for 60ms.
- First INT-high does not immediately finish the gesture.
- Tracker records touch_id, sample_count, start, last, min, max, span, dx, dy, and gesture.
- Tracker explicitly resets before each begin and after each finish.
- Monitor logs include begin/sample/release-pending/finish/reset traces.

## Classifier

- Universal horizontal swipe from anywhere remains the primary navigation.
- Span-based threshold remains 20px.
- Center tap threshold remains 12px.
- CST816 gesture fallback remains below threshold only:
  - 0x03 -> NextPage
  - 0x04 -> PreviousPage

## GPIO quieting

- GPIO14/GPIO17/GPIO16 presence probing remains boot-only.
- No periodic SD refresh during UI validation.
- POWER GPIO6 remains experimental logging only.
- BOOT remains reserved while USB monitor is attached.
