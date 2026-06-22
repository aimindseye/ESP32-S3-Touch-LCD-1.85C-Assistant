# Release v0.1.36-r56-r2

<!-- RAW-R56-R2-RELEASE-NOTES -->

## Status

`v0.1.36-r56-r2` is the current accepted release baseline.

## Accepted behavior

- Boots as `firmware: v0.1.36-r56-r2`.
- Page order remains `Home -> Weather -> Music -> InternetRadio -> Assistant -> Settings`.
- Video page/worker/source paths remain removed from the accepted runtime path.
- Music WAV/MP3 playback remains accepted.
- MP3 progress and stop serialization remain accepted.
- Dedicated Music and Internet Radio touch zones remain accepted.
- Internet Radio supports direct HTTP MP3 streams and HTTPS M3U playlist resolve.
- Settings detail navigation and header-back behavior remain accepted.
- Weather body tap location cycle/fetch/cache remains accepted.
- Weather label now says `Weather Location`.
- Weather nav row previous/next location controls are accepted.
- Mumbai weather fetch/cache is accepted and uses `Asia/Kolkata` / `Asia%2FKolkata`.

## Module cleanup included

- `screens::*` true modules.
- `page_orchestration.rs` for page orchestration.
- `touch_router.rs` for touch classification.
- `page_assets.rs` for cached page base helpers.
- `media_action_router.rs` for Music/Radio actions.
- `settings_action_router.rs` for Settings actions.
- `weather_action_router.rs` for Weather actions/nav row.

## Build

```bash
cd ~/projects/ESP32-S3-Touch-LCD-1.85C-Assistant
./scripts/validate_assistant_current.sh
cd firmware/assistant-rs
cargo build --release
```

## Flash

```bash
cd ~/projects/ESP32-S3-Touch-LCD-1.85C-Assistant/firmware/assistant-rs
cargo espflash flash --release --monitor --port /dev/cu.usbmodem2101
```

## Release package

```bash
cd ~/projects/ESP32-S3-Touch-LCD-1.85C-Assistant
./scripts/package_release.sh
```

## GitHub release

```bash
git tag -a v0.1.36-r56-r2 -m "v0.1.36-r56-r2 Weather Action Cleanup and Repo Documentation Release"
git push origin v0.1.36-r56-r2
gh release create v0.1.36-r56-r2 \
  dist/ESP32-S3-Touch-LCD-1.85C-Assistant-v0.1.36-r56-r2-source.zip \
  --title "v0.1.36-r56-r2 Weather Action Cleanup" \
  --notes-file docs/RELEASE_v0.1.36-r56-r2.md
```

## Hardware-driven architecture

The firmware architecture is intentionally shaped by the Waveshare ESP32-S3-Touch-LCD-1.85C hardware.

The ESP32-S3 target provides Wi-Fi and Bluetooth LE, but it does not provide Bluetooth Classic/A2DP. For this reason, the design does not treat the device as a standard phone Bluetooth speaker. Music playback, Internet Radio, SD-backed assets, and future phone-to-device audio features should use Wi-Fi, SD card storage, HTTP endpoints, or an external Bluetooth Classic audio receiver module.

The 390x390 ST77916 display, CST816 touch controller, SD card, PCM5101 I2S audio path, PCF85063 RTC, 16MB flash, and 8MB PSRAM drive the current architecture:

- raw RGB565 rendering instead of a heavy graphics stack
- SD-backed assets, audio, configuration, and cache files
- serialized MP3 and Internet Radio stop/start ownership around the shared PCM5101 I2S output
- focused modules for screens, page assets, page orchestration, touch routing, media actions, settings actions, and weather actions
- no Video page or Video worker in the accepted release baseline

<!-- RAW-R56-R2-HARDWARE-DRIVEN-ARCHITECTURE-TOKEN -->

## Superseded by v1.0.0

This release note is retained as the engineering checkpoint that was promoted to v1.0.0.

<!-- RAW-R56-R2-RELEASE-NOTES -->
<!-- RAW-R56-R2-RELEASE-DOCS-CLEANUP -->
