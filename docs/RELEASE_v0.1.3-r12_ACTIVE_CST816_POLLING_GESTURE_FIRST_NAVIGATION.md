# v0.1.3-r12 — Active CST816 Polling and Gesture-First Navigation

## Goal

r11 showed that CST816 INT should not be treated as a reliable continuous finger-down level. The bad log section was dominated by one-sample, zero-span touches followed by delayed success once multiple samples were collected. r12 changes INT to a start trigger only.

## Touch loop

- INT only starts a possible touch.
- Once active, the firmware polls CST816 every 8ms regardless of INT state.
- Active polling continues until 3 consecutive no-touch reads or 180ms window expiry.
- Classification happens only after active polling finishes.
- Rendering is skipped while touch polling is active.

## Navigation

- Gesture-first:
  - `0x03` -> NextPage
  - `0x04` -> PreviousPage
- If gesture is zero, use span direction.
- If gesture and span disagree, log both and prefer gesture only when span is less than 35px.
- Span threshold remains 20px.
- Center tap threshold remains 12px.

## Preserved

- accepted v0.1.2 hardware baseline
- r3a/r4a/r5/r6/r7/r8/r9/r10 UI display baseline
- GPIO14/GPIO17/GPIO16 presence boot-only
- no SD refresh during UI validation
- POWER GPIO6 experimental logging only
- BOOT reserved while USB monitor is attached
