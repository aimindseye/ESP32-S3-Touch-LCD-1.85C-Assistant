# Assistant Rust Firmware

This is the canonical firmware for ESP32-S3-Touch-LCD-1.85C-Assistant.

Base reference:

- `ESP32-S3-Touch-LCD-1.85C/demo/rust/rust-full-port`

Direction:

- Rust-owned assistant application shell
- ESP-IDF-backed board integration where needed
- ST77916 display path through the reference shim
- centered safe-content UI for the 360x360 round display
- Home / Weather / Music / Settings assistant apps

The old Arduino import is preserved only as a hardware reference under:

- `firmware/reference/arduino-v1-baseline`

Initial Rust scope:

- boot
- display
- touch
- RTC
- battery
- SD probe
- Wi-Fi scan/status
- Home screen
- Weather placeholder
- Music placeholder

Native SD music playback will be ported after the Rust shell is physically validated.
