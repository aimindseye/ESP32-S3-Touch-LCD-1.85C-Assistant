# v0.1.3-r5 — Edge-Origin Swipe Contract

## Goal

Replace r4a's strict center-swipe classifier with a deterministic bezel-style contract for the 360x360 circular display.

## Contract

- Left edge swipe: start `x=0..50`, `y=55..305`, commit with `end_x>180` -> PreviousPage.
- Right edge swipe: start `x=310..360`, `y=55..305`, commit with `end_x<180` -> NextPage.
- Center tap: `x=95..265`, `y=95..285`, movement <=20 -> Select.
- Top band: ignored.
- Bottom band: ignored.
- Edge taps without center-cross commit: ignored.
- Center horizontal swipes: ignored.
- CST816 gesture byte remains diagnostic only.

## UI

Adds the gesture hint:

```text
< EDGE SWIPE    EDGE SWIPE >
```

## Preserved

- accepted v0.1.2 hardware baseline
- r3a/r4a visual display baseline
- POWER GPIO6 experimental logging
- BOOT reserved while USB monitor is attached
- build/flash helpers and validator
