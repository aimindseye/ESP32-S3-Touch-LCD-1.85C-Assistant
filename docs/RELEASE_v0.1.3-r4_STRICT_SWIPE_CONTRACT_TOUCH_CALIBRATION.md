# v0.1.3-r4 — Strict Swipe Contract and Touch Calibration

## Goal

Make the circular assistant touch model predictable by removing edge-tap page navigation and using a strict swipe-only contract for page movement.

## Rules

- Swipe left: `dx <= -100`, start `y=80..285`, horizontal-dominant, duration `50..700ms` -> NextPage.
- Swipe right: `dx >= 100`, start `y=80..285`, horizontal-dominant, duration `50..700ms` -> PreviousPage.
- Center tap: movement <=20px and end point inside `x=95..265`, `y=95..285` -> Select.
- Top status band: ignored.
- Bottom page-dot band: ignored.
- Left/right edge taps: ignored.
- CST816 gesture byte: logged as diagnostic only.

## Preserved

- accepted v0.1.2 hardware baseline
- r3a UI visual polish
- POWER GPIO6 experimental logging
- BOOT reserved while USB monitor is attached
- build/flash helpers and validator
