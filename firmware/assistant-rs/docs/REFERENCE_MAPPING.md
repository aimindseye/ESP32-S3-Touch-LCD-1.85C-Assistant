# Reference mapping

This project is now best understood as a **hybrid Rust + ESP-IDF implementation**, not a direct pure-Rust rewrite of the vendor demos.

## How to read this mapping

### Primary references
These are the references that still actively shape the current code path.

### Historical references
These were important earlier, but they no longer define the active architecture.

---

## Primary external references

### Uploaded vendor trees

#### Arduino references
- `Arduino/examples/LVGL_Arduino/LVGL_Arduino.ino`
  - overall init order
  - page switching behavior
- `Arduino/examples/LVGL_Arduino/Display_ST77916.cpp`
  - display behavior
  - backlight behavior
- `Arduino/examples/LVGL_Arduino/Display_ST77916.h`
  - QSPI pin map
  - display size
- `Arduino/examples/LVGL_Arduino/LVGL_Example.cpp`
  - Onboard/music page structure
- `Arduino/examples/LVGL_Arduino/SD_Card.h`
  - SD card pins
- `Arduino/examples/LVGL_Arduino/BAT_Driver.*`
  - battery conversion assumptions

#### ESP-IDF references
- `main/main.c`
  - vendor init order
- `main/LCD_Driver/ST77916.c`
  - LCD init flow
  - EXIO2 reset behavior
  - backlight assumptions
- `main/LCD_Driver/esp_lcd_st77916/esp_lcd_st77916.c`
  - ESP-IDF-side ST77916 handling
  - QSPI packing model
- `main/Touch_Driver/CST816.c` / `.h`
  - touch register usage
  - EXIO1 reset
- `main/EXIO/TCA9554PWR.*`
  - expander semantics
- `main/PCF85063/PCF85063.*`
  - RTC flow
- `main/SD_Card/SD_MMC.*`
  - SD / capacity reporting model
- `main/Wireless/Wireless.*`
  - Wi-Fi scan parity target
- `main/LVGL_UI/LVGL_Example.c`
  - vendor page fields
  - tab/page intent
  - music-page reference behavior

### Supplemental external reference
- Home Assistant YAML:
  - full-screen `360x360` sanity check
  - consistent pin usage for display/touch/backlight/battery
  - useful as a layout/config sanity reference, not as the active runtime model

Reference:
`https://github.com/nishad2m8/Home-Assistant-YAML/blob/main/WaveShare/01-Waveshare%20Speaker%20Box%20%E2%80%93%20Home%20Assistant%20Voice%20Assistant%20YAML/waveshare-speaker-box.yaml`

---

## Primary in-repo ownership

### `components/st77916_shim/`
Active owner of the display bridge.

Files here are responsible for:
- ESP-IDF panel init
- ST77916 display bridge
- SD presence / capacity probe bridge used by Rust

This is now the most important display/storage boundary in the project.

### `src/main.rs`
Active application entry and orchestrator.

Current responsibilities:
- runtime bootstrap
- PSRAM-backed framebuffer ownership
- Rust thread / app loop shape
- board peripheral orchestration
- redraw cadence
- touch-driven repaint flow
- centered safe-content panel layout

### `src/ffi.rs`
Rust-to-C boundary for the shim.

Current responsibilities:
- panel init entrypoint
- RGB565 draw entrypoint
- SD probe bridge entrypoint

### `src/board.rs`
Central board constants.

Current responsibilities:
- pin mapping
- display dimensions
- battery constants
- board-level naming/constants used by the app and drivers

### `src/drivers/cst816.rs`
Touch driver used by the active app path.

### `src/drivers/pcf85063.rs`
RTC driver used by the active app path.

### `src/drivers/tca9554.rs`
EXIO driver used by the active app path.

### `src/app/model.rs`
Onboard page model and field formatting.

Current responsibilities:
- app-facing data model for the diagnostics page
- state carried between probes and repaints

---

## Historical / secondary in-repo ownership

These files or concepts may still exist in the tree, but they do not describe the primary implementation direction anymore.

### Older Rust display path
Historical:
- `src/drivers/st77916.rs`
- `src/drivers/backlight.rs`
- feature-gated `esp-hal` display / LVGL flow

These were useful during the earlier bring-up attempts, but the active implementation is now the hybrid ESP-IDF shim path.

### Earlier UI direction
Historical:
- full-screen edge bars
- circular clip experiments
- earlier LVGL-first assumptions

Current replacement:
- centered safe-content rectangle inside the round display

---

## Reference hierarchy for future changes

When deciding what to trust next:

1. **current working hybrid code path**
   - `components/st77916_shim`
   - `src/main.rs`
   - `src/ffi.rs`
   - active Rust drivers
2. **uploaded ESP-IDF vendor tree**
   - especially display, touch, EXIO, RTC, SD, wireless references
3. **uploaded Arduino vendor tree**
   - especially UI/page behavior and battery assumptions
4. **supplemental YAML**
   - sanity check only
5. **older pure-Rust/full-port experiments**
   - historical only

## Practical guidance

If a future change touches:
- display init: prefer the shim/vendor ESP-IDF path
- touch / RTC / EXIO: prefer the existing Rust drivers
- Onboard page fields: prefer `src/app/model.rs` + `src/main.rs`
- layout: follow the centered safe-content panel direction
