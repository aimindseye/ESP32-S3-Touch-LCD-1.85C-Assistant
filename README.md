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
