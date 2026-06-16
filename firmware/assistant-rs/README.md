# esp32s3-touch-lcd-1.85c-rust-full-port

Hybrid Rust + ESP-IDF port for the Waveshare ESP32-S3-Touch-LCD-1.85C / 1.85C-BOX vendor demo.

## Current state

This repository is no longer positioned as a pure `esp-hal` full-port scaffold.

The active implementation path is now:

- **ESP-IDF-native display path** through a minimal C shim and `esp_lcd_st77916`
- **Rust-owned board/application logic** for:
  - EXIO / TCA9554
  - touch / CST816
  - RTC / PCF85063
  - battery ADC sampling
  - Wi-Fi scan count
  - SD card presence / capacity reporting
  - UI model and repaint loop
- **PSRAM-backed framebuffer** for the Onboard page
- **centered safe-content panel** UI approach for the round display

## What works now

- stable boot on the board with managed ESP-IDF
- ST77916 panel init through the ESP-IDF shim
- backlight enable
- EXIO / TCA9554 probe and reset flow
- CST816 touch config read and touch-triggered repaints
- PCF85063 RTC reads
- PSRAM enabled and used for the page framebuffer
- Wi-Fi scan count wired into the Onboard page
- SD probe / capacity path wired through the shim
- battery ADC wired into the Onboard page
- touch, RTC, and page repaint loop running together

## What is still unfinished

- final visual polish for the round display
  - header/footer clipping
  - panel sizing / spacing
  - typography and card balance
- battery calibration still needs real-world validation
- SD presentation formatting still needs polish
- music page parity with the vendor demo
- audio, MP3 playback, microphone, and speech features

## Design approach change

### Old approach

Earlier work in this repo tried to stay close to a pure Rust `esp-hal` full-port with:
- custom QSPI transport
- Rust-side ST77916 handling
- feature-gated LVGL path
- portable Rust-first SD / Wi-Fi abstractions

### Current approach

The active design is now:

- **use ESP-IDF where the vendor stack is already strong**
  - panel init / vendor display path
  - Wi-Fi / storage integration
  - PSRAM integration
- **use Rust for board logic and application structure**
  - hardware orchestration
  - page model
  - input handling
  - redraw policy

This reduced risk on the hardest part of the board bring-up: the ST77916 QSPI display path.

## UI strategy

The current UI direction is:

- render to the full `360x360` panel
- avoid pretending the display is smaller than it is
- place all important content inside a **centered safe-content rectangle**
- let the physical round bezel hide the corners
- keep header, grid, and footer **inside the same content panel**

That is the chosen direction for completing this effort before moving on to OS work.

## Repository focus

This repo currently focuses on:

1. getting the Waveshare vendor-demo hardware scope working from Rust + ESP-IDF
2. completing a stable Onboard page
3. documenting the board and porting decisions clearly
4. creating a clean base for the later OS effort

## Recommended bring-up order

1. verify the toolchain and ESP-IDF integration build cleanly
2. confirm panel init + backlight
3. confirm EXIO / touch / RTC
4. confirm PSRAM-backed Onboard page render
5. confirm Wi-Fi count and SD probe path
6. finish centered safe-content UI layout
7. move to music/audio parity

## Build model

This repo now expects the **Espressif Rust toolchain + ESP-IDF** path rather than the earlier pure `esp-hal` path.

Representative build / run flow:

```bash
rustup override set esp
cargo run --release
```

Representative ESP-IDF-related project settings now include:

- managed ESP-IDF through `esp-idf-sys`
- extra component under `components/st77916_shim`
- `sdkconfig.defaults` for PSRAM-related settings
- PSRAM/Wi-Fi/FATFS memory preference tuning

## Important implementation notes

- the display path is intentionally hybrid, not pure Rust-only
- the current Onboard page is a pragmatic validation UI, not the final product UI
- the current UI is meant to finish hardware parity work quickly so the effort can move on to a true OS shell
- future app/runtime work should build on this repo’s validated board support and display/input/storage/network foundation rather than reopening the display bring-up problem

## Current subsystem summary

- Display: working
- Backlight: working
- Touch: working
- RTC: working
- PSRAM: working
- Wi-Fi scan: wired and showing live counts
- SD probe: wired
- Battery ADC: wired
- UI layout: functional, still needs final polish
- Music/audio: not done
- Speech/assistant: not done
- Runtime-installed apps / OS: future phase

## Vendor / external references used

The implementation has been guided by:

- uploaded Waveshare Arduino and ESP-IDF vendor trees
- the vendor ST77916 / touch / RTC / EXIO flows
- the vendor Onboard + music page field list
- the Home Assistant YAML as a supplemental sanity check for:
  - full-screen `360x360` assumptions
  - battery GPIO
  - touch/backlight/display pin consistency

Supplemental YAML reference:
`https://github.com/nishad2m8/Home-Assistant-YAML/blob/main/WaveShare/01-Waveshare%20Speaker%20Box%20%E2%80%93%20Home%20Assistant%20Voice%20Assistant%20YAML/waveshare-speaker-box.yaml`

## Near-term next steps

1. finish the centered safe-content panel layout
2. clean up Onboard page typography and spacing
3. validate battery scaling
4. improve SD display formatting
5. add a real music page shell


