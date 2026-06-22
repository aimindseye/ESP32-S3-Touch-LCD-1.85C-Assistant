# Architecture


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

This file mirrors the current top-level architecture summary. See [`architecture.md`](architecture.md) for the maintained architecture document.

Current accepted firmware: `v0.1.36-r56-r2`.

Key modules:

```text
screens::*
page_assets.rs
page_orchestration.rs
touch_router.rs
media_action_router.rs
settings_action_router.rs
weather_action_router.rs
```

Important hardware constraints:

- ESP32-S3 target with Wi-Fi and BLE-only Bluetooth.
- No Bluetooth Classic / A2DP, so the device is not a standard phone Bluetooth speaker.
- Raw RGB565 rendering is retained for flash/RAM predictability.
- SD card is used for assets, audio files, radio station list, and calibration/config files.
- PCM5101 I2S output path is shared by Music and Internet Radio and must remain serialized.

See [`docs/HARDWARE.md`](docs/HARDWARE.md) and [`docs/RELEASE_v0.1.36-r56-r2.md`](docs/RELEASE_v0.1.36-r56-r2.md).
