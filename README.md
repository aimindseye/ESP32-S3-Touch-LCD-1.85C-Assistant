# ESP32-S3 Touch LCD 1.85C Assistant

Current accepted firmware/release: **v1.1.0 — Clean Source + Stable Internet Radio**

Rust + ESP-IDF firmware for the Waveshare ESP32-S3-Touch-LCD-1.85C / 1.85C-BOX. The accepted product path is a compact round-screen assistant with Home, Weather, Music, Internet Radio, Assistant, and Settings pages.

## v1.1.0 highlights

- Stabilized Internet Radio playback with a PSRAM-backed FreeRTOS StreamBuffer producer/consumer path.
- Preserved live Internet Radio UI refresh for time/status updates without reintroducing audio stutter.
- Fixed Internet Radio station-select UX so STOP, NEXT, and PREV leave the selected station ready with a PLAY button instead of stale STOP state.
- Refactored UI source:
  - extracted low-level RGB565 drawing helpers to `ui_primitives.rs`;
  - extracted reusable widgets/icons to `ui_widgets.rs`;
  - reduced active `main.rs` orchestration scope.
- Cleaned source tree:
  - removed historical archive directories;
  - removed stale Video/MJPEG active paths;
  - removed obsolete scripts/documents from the current source package;
  - replaced the historical validator chain with a concise current-state validator.
- Updated docs for the accepted v1.1.0 baseline.
- Treats `firmware/assistant-rs/components_esp32s3.lock` as generated ESP-IDF component-manager metadata: it is ignored/untracked, but validation allows it after local builds because ESP-IDF can recreate it.

## Accepted feature set

- Home status screen with time, Wi-Fi, battery, SD, and runtime status.
- Weather screen with `Weather Location`, previous/next location controls, Open-Meteo cache/fetch behavior, and Fahrenheit default.
- Music screen with SD-backed WAV/MP3 playback, HELIX MP3 decode, progress display, and volume controls.
- Internet Radio with HTTP/HTTPS/M3U support, station list cache, PSRAM StreamBuffer producer/consumer playback, live UI refresh, and corrected station-select PLAY/STOP UX.
- Assistant screen placeholder/card.
- Settings overview and detail screens for Network, Time, Display, Sound, Storage, Device, and Diagnostics.
- Software sleep/wake with touch interrupt wake guard.

Video/MJPEG code and historical source archives have been removed from the active source tree.

## Hardware

- MCU: ESP32-S3R8
- Display: 1.85-inch ST77916 round LCD, 390x390 class panel with 360x360 safe framebuffer use
- Touch: CST816 capacitive touch controller
- RTC: PCF85063
- Audio: PCM5101 I2S DAC path
- Storage: TF / microSD card
- Flash: 16 MB
- PSRAM: 8 MB
- Connectivity: Wi-Fi and Bluetooth LE

The ESP32-S3 does not support Bluetooth Classic/A2DP, so this device is not a standard phone Bluetooth speaker target. Use SD audio, Internet Radio URLs, Wi-Fi transfer/HTTP designs, or an external Bluetooth Classic receiver module for phone-audio workflows.

## Source layout

```text
firmware/assistant-rs/src/main.rs                   boot/runtime orchestration
firmware/assistant-rs/src/screens/*                 page render modules
firmware/assistant-rs/src/ui_primitives.rs          RGB565 primitives and text drawing
firmware/assistant-rs/src/ui_widgets.rs             reusable high-level UI widgets
firmware/assistant-rs/src/page_assets.rs            page-base asset cache and rendering
firmware/assistant-rs/src/page_orchestration.rs     page dispatch/navigation
firmware/assistant-rs/src/touch_router.rs           touch classification/routing
firmware/assistant-rs/src/media_action_router.rs    Music and Internet Radio actions
firmware/assistant-rs/src/settings_action_router.rs Settings actions
firmware/assistant-rs/src/weather_action_router.rs  Weather actions
firmware/assistant-rs/src/audio_foundation.rs       local WAV/MP3 playback
firmware/assistant-rs/src/internet_radio.rs         stations and radio state
firmware/assistant-rs/components/*                  ESP-IDF C shims and HELIX MP3
```

More detail is in [`architecture.md`](architecture.md). User-facing notes are in [`userguide.md`](userguide.md).

## SD card layout

```text
/WIFI.TXT                 optional Wi-Fi credentials import
/BATTERY.TXT              optional battery calibration
/AUDIO/*.WAV              local WAV files
/AUDIO/*.MP3              local MP3 files
/AUDIO/RADIO~1.TXT        Internet Radio station list
/ASSETS/*.rgb565          optional SD-backed UI page assets
/LOG.TXT                  optional DEBUG log profile/config
```

Keep `/LOG.TXT` removed for normal audio-quality testing. DEBUG logging is for short diagnostics only and can affect radio playback timing.

## Validate

```bash
./scripts/validate_assistant_current.sh
```

Expected current validation includes:

```text
Assistant current consolidated validation: OK
v1.0.1-r13 accepted Internet Radio baseline: OK
v1.0.1-r14 source directory cleanup: OK
v1.0.1-r14-r2 generated components lock validator repair: OK
```

## Build

```bash
./scripts/build_assistant_rs.sh
```

Or manually:

```bash
./scripts/fix_assistant_partition_path.sh
cd firmware/assistant-rs
cargo build --release
```

## Flash

```bash
./scripts/flash_assistant_rs.sh --port /dev/cu.usbmodem2101
```

Or manually:

```bash
cd firmware/assistant-rs
cargo espflash flash --release --monitor --port /dev/cu.usbmodem2101
```

## Package release

```bash
./scripts/package_release.sh
```

## Git/release hygiene

`firmware/assistant-rs/components_esp32s3.lock` is generated by ESP-IDF component manager during local builds. It should remain ignored/untracked and should not be committed as source.

Before committing a release, verify generated files are not staged:

```bash
git status --short | grep -E 'target/|components_esp32s3.lock|\.cleanup/' || true
```

That command should print nothing.

## Runtime marker

Expected Internet Radio runtime marker:

```text
radio-r36-r32: stream_pacing=STREAMBUFFER_PSRAM_PRODUCER_CONSUMER_R11_R1
```

<!-- RAW-V1-1-0-CLEAN-STABLE-RADIO-README -->
