# v0.1.3-r6 — Hybrid Gesture-Aware Touch Classifier

## Goal

r5 proved that a center-cross edge contract is too strict for the CST816 behavior on this round display. r6 keeps the accepted visual UI and makes touch classification hybrid:

- CST816 gesture bytes are used as classifier input.
- Edge flicks do not need to cross the center line.
- Natural center swipes are accepted as a fallback.
- Stationary edge taps remain ignored.

## Contract

- Right edge next: start `x>=310`, `y=55..305`, and `gesture=0x03` or `dx<=-20`.
- Left edge previous: start `x<=50`, `y=55..305`, and `gesture=0x04` or `dx>=20`.
- Center swipe fallback: start `x=70..290`, `y=80..285`, `abs(dx)>=80`, horizontal-dominant.
- Center tap: end `x=95..265`, `y=95..285`, movement <=20.
- Top/bottom bands ignored.
- Stationary edge taps ignored.

## Preserved

- accepted v0.1.2 hardware baseline
- r3a/r4a/r5 visual display baseline
- POWER GPIO6 experimental logging
- BOOT reserved while USB monitor is attached
- build/flash helpers and validator
