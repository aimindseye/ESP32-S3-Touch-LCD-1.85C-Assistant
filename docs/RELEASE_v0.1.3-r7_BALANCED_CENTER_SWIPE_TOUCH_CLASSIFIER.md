# v0.1.3-r7 — Balanced Center-Swipe Touch Classifier

## Goal

r6 responded to some edge gestures, but it made edge navigation too sensitive while many natural center swipes were ignored. r7 makes center horizontal swipe the primary page navigation and keeps edge gestures only as assistive input.

## Contract

- Center swipe next: start `x=60..300`, `y=75..300`, `dx<=-45`, horizontal-dominant.
- Center swipe previous: start `x=60..300`, `y=75..300`, `dx>=45`, horizontal-dominant.
- Right edge assist: start `x>=310` and `gesture=0x03` or `dx<=-45`.
- Left edge assist: start `x<=50` and `gesture=0x04` or `dx>=45`.
- Center tap: end `x=95..265`, `y=95..285`, movement <=18.
- Stationary edge taps ignored.
- Top/bottom bands ignored.

## UI

Hint updated to:

```text
SWIPE CENTER  TAP CENTER
```

## Preserved

- accepted v0.1.2 hardware baseline
- r3a/r4a/r5/r6 visual display baseline
- POWER GPIO6 experimental logging
- BOOT reserved while USB monitor is attached
- build/flash helpers and validator
