# v0.1.3-r8 — Universal Horizontal Swipe Navigation

## Goal

r7 still mixed center, edge, top, bottom, and gesture-assisted handling. r8 deliberately simplifies the touch contract:

- any horizontal swipe from anywhere changes page
- direction is based only on `dx`
- CST816 gesture byte remains diagnostic only
- center tap remains Select

## Contract

- Swipe left anywhere: `dx < 0`, `abs(dx)>=35`, horizontal-dominant -> NextPage.
- Swipe right anywhere: `dx > 0`, `abs(dx)>=35`, horizontal-dominant -> PreviousPage.
- Center tap: end `x=95..265`, `y=95..285`, movement <=18 -> Select.
- Vertical swipes ignored.
- Small movement below 35px ignored.

## UI

Hint updated to:

```text
SWIPE ANYWHERE
```

## Preserved

- accepted v0.1.2 hardware baseline
- r3a/r4a/r5/r6/r7 visual display baseline
- POWER GPIO6 experimental logging
- BOOT reserved while USB monitor is attached
- build/flash helpers and validator
