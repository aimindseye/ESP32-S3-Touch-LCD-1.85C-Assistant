# v0.1.3 — Circular Assistant UI + Button Navigation Foundation

## Status

Pending hardware validation.

## Baseline preserved

This patch preserves the accepted v0.1.2 Rust Assistant App Shell baseline:

- ESP-IDF v5.3.2 Rust firmware
- 16MB flash custom partition table with 3MB factory app partition
- ST77916 C shim linkage
- PSRAM framebuffer
- I2C probes for TCA9554, CST816, and PCF85063
- RTC read path
- battery ADC path
- Wi-Fi scan path
- SD capacity probe path
- touch interrupt/read path
- build, flash, partition-fix, timestamp-normalization, and validation helpers

## User-visible change

The rectangular diagnostic dashboard is replaced by a circular, smartwatch-inspired tile UI:

- Home
- Weather
- Music
- Settings

The Home tile remains the primary assistant face. Dense diagnostics are moved to Settings > System placeholder content.

## Navigation model

Touch:

- Swipe left: next tile
- Swipe right: previous tile
- Left edge tap: previous tile fallback
- Right edge tap: next tile fallback
- Center tap: select/action placeholder

Buttons:

- BOOT short press: select/action placeholder
- BOOT long press: assistant placeholder
- POWER short press: back/home
- POWER long press: power menu placeholder

## Button pin assumptions for validation

- BOOT: GPIO0
- POWER candidate: GPIO6

The POWER candidate is input-only in this patch. If physical validation shows the power key is not GPIO6, only the input binding should change; the app-level interaction model should remain the same.

## Expected monitor markers

```text
Circular Assistant UI + Button Navigation Foundation
Watch-style Home/Weather/Music/Settings tiles
Touch navigation: swipe left/right plus edge tap fallback
Button navigation: BOOT select/assistant, POWER home/menu
Home page preserves PSRAM framebuffer + real BAT/WIFI/SD behavior
circular home page rendered
button: BOOT short -> select/action
button: BOOT long -> assistant placeholder
button: POWER short -> back/home
button: POWER long -> power menu placeholder
nav: NextPage -> ...
nav: PreviousPage -> ...
repaint ok
```
