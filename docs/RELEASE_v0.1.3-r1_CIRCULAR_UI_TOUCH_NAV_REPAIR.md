# v0.1.3-r1 — Circular UI Layout and Touch Navigation Repair

## Goal

Repair the first circular UI pass after physical validation showed that the watch-style direction is correct, but the layout still used too little of the circular display and touch navigation did not work reliably.

## Preserved baseline

This patch preserves the accepted v0.1.2 hardware baseline:

- ESP-IDF v5.3.2 Rust firmware
- 16MB flash custom partition table
- 3MB factory app partition
- ST77916 C shim linkage
- PSRAM framebuffer rendering
- I2C probes for TCA9554, CST816, and PCF85063
- RTC, touch, battery, Wi-Fi scan, and SD behavior
- Build, flash, timestamp-normalization, partition-path, and validator scripts

## UI changes

- Keeps Home / Weather / Music / Settings circular tile direction.
- Expands content into more of the circular face.
- Moves status/header indicators to the top arc.
- Moves page dots to the bottom arc.
- Removes the heavy inner ring from the previous pass.
- Increases primary text size on Home, Weather, and Music.
- Keeps dense diagnostics only inside Settings > System.

## Touch behavior

The CST816 controller reported `gesture=0x00` during the failed test, so this revision makes coordinate zones the primary navigation path:

- Left third: previous page
- Right third: next page
- Center area: select / assistant placeholder

CST816 gesture-code support remains present for real swipes, but the UI no longer depends on gesture codes.

## Button behavior

- BOOT / GPIO0 is reserved in this revision because it is a flash strap and pressing it while USB monitor is attached can confuse host-side serial tooling.
- Center tap is the assistant/select action.
- POWER / GPIO6 remains an experimental input candidate and is logged as such.

## Expected markers

```text
Circular UI Layout and Touch Navigation Repair
Touch navigation: left/right thirds plus center select
BOOT runtime control reserved while USB monitor is attached
POWER candidate logging: GPIO6 experimental home/menu
expanded circular home page rendered
```
