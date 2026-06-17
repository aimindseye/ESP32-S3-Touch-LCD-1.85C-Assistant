# v0.1.3-r9 — Universal Swipe with Gesture Fallback

## Goal

r8 made touch response much better with universal horizontal swipes, but some physical swipes were still logged as short movements when the sampled coordinate delta was too small. r9 keeps universal swipe as the primary path, lowers the coordinate threshold, and uses the CST816 gesture byte only as a below-threshold fallback.

## Contract

- Coordinate swipe left: `dx < 0`, `abs(dx)>=25`, horizontal-dominant -> NextPage.
- Coordinate swipe right: `dx > 0`, `abs(dx)>=25`, horizontal-dominant -> PreviousPage.
- Gesture fallback only when `abs(dx) < 25`:
  - `gesture=0x03` -> NextPage.
  - `gesture=0x04` -> PreviousPage.
- Center tap: end `x=95..265`, `y=95..285`, movement <=18 -> Select.
- Vertical swipes ignored.
- Stationary touches with no gesture ignored.
- Ignored-event logs include dx/dy/gesture.

## UI

Hint remains:

```text
SWIPE ANYWHERE
```

## Preserved

- accepted v0.1.2 hardware baseline
- r3a/r4a/r5/r6/r7/r8 visual display baseline
- POWER GPIO6 experimental logging
- BOOT reserved while USB monitor is attached
- build/flash helpers and validator
