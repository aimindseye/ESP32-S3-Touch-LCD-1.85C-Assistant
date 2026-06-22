# ESP32-S3 Touch LCD 1.85C Assistant


## User guide and screenshots

The v1.0.0 user guide is available in [`userguide.md`](userguide.md).

It includes screen-by-screen usage notes and screenshots for:

- Home
- Weather
- Music
- Internet Radio
- Assistant
- Settings
- Network
- Time
- Display
- Sound
- Storage
- Device
- Diagnostics

Screenshots are stored under [`docs/screenshots/`](docs/screenshots/).

<!-- RAW-V1-0-0-README-USERGUIDE-LINK -->

Current stable release: `v1.0.0`

This project is a Rust firmware application for the Waveshare ESP32-S3-Touch-LCD-1.85C / 1.85C-BOX. It provides a compact touch assistant UI with Home, Weather, Music, Internet Radio, Assistant, and Settings pages.

## v1.0.0 accepted baseline

`v1.0.0` promotes the accepted `v0.1.36-r56-r2` firmware baseline to a stable public release.

Accepted runtime behavior includes:

- Home, Weather, Music, Internet Radio, Assistant, and Settings pages
- Weather location cycling, previous/next nav-row controls, fetch/cache behavior, and `Weather Location` label
- SD-backed Music playback for WAV and MP3
- HELIX MP3 progress reporting
- Dedicated media touch zones: `VOL- | PREV | PLAY/STOP | NEXT | VOL+`
- Internet Radio playback from HTTP, HTTPS, and M3U stations
- Internet Radio station names and transport controls
- Settings detail navigation and back behavior
- Software sleep / wake behavior
- No Video page, no Video worker, and no Video source path in the accepted release

## Hardware summary

Target hardware:

- MCU: ESP32-S3R8
- Display: 1.85-inch ST77916 round LCD, 390x390
- Touch: CST816 capacitive touch controller
- RTC: PCF85063
- Audio output: PCM5101 I2S DAC path
- Storage: TF / microSD card
- Flash: 16MB
- PSRAM: 8MB
- Connectivity: Wi-Fi and Bluetooth LE

## Hardware-driven design notes

The ESP32-S3 supports Wi-Fi and Bluetooth LE, but it does not support Bluetooth Classic/A2DP. Therefore this firmware should not be treated as a normal phone Bluetooth speaker target. Phone-to-device audio should use Wi-Fi, SD card transfer, Internet Radio URLs, an HTTP receiver design, or an external Bluetooth Classic audio receiver module.

The current design uses raw RGB565 drawing and SD-backed assets to fit the display, memory, and flash constraints while keeping the UI responsive. Music and Internet Radio share the PCM5101 I2S output path, so playback actions are serialized to preserve stable audio ownership.

## Build

```bash
cd firmware/assistant-rs
cargo build --release
```

## Flash

Flash only after validation and build succeed:

```bash
cd firmware/assistant-rs
cargo espflash flash --release --monitor --port /dev/cu.usbmodem2101
```

## Validation

```bash
./scripts/validate_assistant_current.sh
```

## Package release

```bash
./scripts/package_release.sh
```

<!-- RAW-V1-0-0-README -->

---

## Historical notes

# ESP32-S3 Touch LCD 1.85C Assistant

Current accepted firmware: `v0.1.36-r56-r2`

This repository contains the Rust/ESP-IDF assistant firmware for the Waveshare ESP32-S3 Touch LCD 1.85C / 1.85C-BOX class device. The current accepted baseline is a compact round-screen assistant with Weather, Music, Internet Radio, Assistant, and Settings pages.

## Current accepted UI

```text
Home -> Weather -> Music -> InternetRadio -> Assistant -> Settings -> Home
```

Video has been removed from the accepted product path. The firmware and validators intentionally preserve the post-Video cleanup baseline.

<!-- RAW-R50-DOCS-NO-SCREEN-INCLUDES -->

## Current accepted features

- SD-backed RGB565 page assets with dynamic overlays.
- Home screen with status, RTC/time, Wi-Fi, battery, and storage reporting.
- Weather screen with `Weather Location`, live Open-Meteo fetch/cache, previous/next location nav row, units support, and Mumbai configured as `Asia/Kolkata`.
- Music screen with accepted WAV/MP3 playback, HELIX MP3 decode, progress display, stop serialization, and dedicated media touch zones.
- Internet Radio with HTTP MP3, HTTPS playlist/M3U resolve, station names, volume, next/previous, and stop serialization.
- Settings overview and detail pages with accepted detail navigation and header-back behavior.
- Software sleep policy with touch wake guard.
- NORMAL/DEBUG monitor log profiles.

## Hardware target

The firmware is designed for an ESP32-S3 board with a small round LCD, capacitive touch, SD storage, RTC, Wi-Fi, BLE, PSRAM, flash, and I2S audio output. The tested project target is the Waveshare ESP32-S3 Touch LCD 1.85C / 1.85C-BOX class hardware.

Important hardware design constraints are documented in [`docs/HARDWARE.md`](docs/HARDWARE.md).

## Bluetooth speaker limitation

This device should not be treated as a normal phone Bluetooth speaker target. Phones normally stream speaker audio using Bluetooth Classic A2DP. The ESP32-S3 supports Bluetooth LE but does not support Bluetooth Classic / A2DP. For phone-to-device audio, the preferred software-only path is Wi-Fi audio or URL/stream playback over the existing Wi-Fi + MP3/Internet Radio pipeline.

## Architecture overview

The current firmware is intentionally split into focused modules:

```text
src/screens/*                 page renderers
src/page_assets.rs            cached page base / asset rendering helpers
src/page_orchestration.rs     page dispatch and navigation support
src/touch_router.rs           general touch classification and routing
src/media_action_router.rs    Music and Internet Radio action routing
src/settings_action_router.rs Settings detail/action routing
src/weather_action_router.rs  Weather select/action routing and nav row handling
src/app/*                     app state, providers, actions, model boundaries
src/audio_foundation.rs       WAV/MP3 playback and PCM5101 I2S output path
src/internet_radio.rs         station list, stream/playlist playback state
```

More detail is in [`architecture.md`](architecture.md).

## SD card layout

Typical SD files:

```text
/WIFI.TXT                 optional Wi-Fi credentials import
/BATTERY.TXT              optional battery calibration
/AUDIO/*.WAV              local WAV files
/AUDIO/*.MP3              local MP3 files
/AUDIO/RADIO~1.TXT        Internet Radio station list
/assets/*.rgb565          UI page base assets when SD-backed assets are used
/LOG.TXT                  optional DEBUG log profile/config
```

## Build and validate on macOS

```bash
cd ~/projects/ESP32-S3-Touch-LCD-1.85C-Assistant

./scripts/validate_assistant_current.sh

cd firmware/assistant-rs
cargo build --release
```

## Flash

Only flash after validation and release build succeed.

```bash
cd ~/projects/ESP32-S3-Touch-LCD-1.85C-Assistant/firmware/assistant-rs
cargo espflash flash \
  --release \
  --monitor \
  --port /dev/cu.usbmodem2101
```

Expected boot banner:

```text
firmware: v0.1.36-r56-r2 ... weather_action_cleanup,weather_nav_buttons,weather_nav_row_touch
ui: pages=Home,Weather,Music,InternetRadio,Assistant,Settings controls=DEDICATED_MEDIA_ZONES
```

## Release packaging

```bash
cd ~/projects/ESP32-S3-Touch-LCD-1.85C-Assistant
./scripts/package_release.sh
```

This creates a clean release zip under `dist/` and excludes build output, cleanup backups, old patch backups, SD-card contents, and historical scratch archives.

## Release notes

See [`docs/RELEASE_v0.1.36-r56-r2.md`](docs/RELEASE_v0.1.36-r56-r2.md).
