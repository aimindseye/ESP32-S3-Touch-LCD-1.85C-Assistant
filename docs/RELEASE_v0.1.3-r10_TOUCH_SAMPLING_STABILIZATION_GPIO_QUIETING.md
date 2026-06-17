# v0.1.3-r10 — Touch Sampling Stabilization and GPIO Quieting

## Goal

r9 improved touch response but still ignored some physical swipes near the threshold and periodic GPIO reconfiguration logs appeared during validation. r10 keeps the universal swipe model and stabilizes sampling.

## Touch changes

- Universal swipe threshold lowered from 25px to 20px.
- Center tap movement threshold reduced from 18px to 12px.
- Touch tracker records start, last, min, and max coordinates while the finger is down.
- Swipe classification uses largest horizontal travel span rather than only final up-start delta.
- Touch-up logs include start/end/min/max/span/dx/dy/gesture.
- CST816 gesture fallback remains only below the coordinate span threshold.

## GPIO/status changes

- Startup SD/GPIO status probe is kept.
- Periodic SD status refresh is disabled to avoid repeated GPIO14/GPIO17/GPIO16 reconfiguration logs.
- Non-touch status polling is skipped while a touch is active.
- Touch polling runs before button/status/render work.
- Rendering remains suppressed during active touch.

## Preserved

- accepted v0.1.2 hardware baseline
- r3a/r4a/r5/r6/r7/r8/r9 visual display baseline
- POWER GPIO6 experimental logging
- BOOT reserved while USB monitor is attached
- build/flash helpers and validator
