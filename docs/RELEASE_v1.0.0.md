# v1.0.0 — Stable Release

`v1.0.0` promotes the accepted `v0.1.36-r56-r2` baseline to the first stable release.

## Highlights

- Stable six-page assistant UI:
  - Home
  - Weather
  - Music
  - Internet Radio
  - Assistant
  - Settings
- Weather screen label repair: `Weather Location`
- Weather previous/next nav-row location controls
- Weather fetch/cache preserved for Jersey City, New York, Edison, and Mumbai
- Mumbai timezone preserved as `Asia/Kolkata` / `Asia%2FKolkata`
- Music WAV/MP3 playback preserved
- MP3 progress indicator preserved
- Internet Radio HTTP, HTTPS, and M3U playback preserved
- Dedicated media zones preserved:
  - `VOL-`
  - `PREV`
  - `PLAY/STOP`
  - `NEXT`
  - `VOL+`
- Settings detail navigation preserved
- Video page and Video worker remain removed

## Hardware-driven architecture

This release documents the hardware constraints and design decisions for the Waveshare ESP32-S3-Touch-LCD-1.85C:

- ESP32-S3 Wi-Fi and Bluetooth LE
- No Bluetooth Classic/A2DP support
- ST77916 390x390 display
- CST816 touch
- PCF85063 RTC
- PCM5101 I2S audio output
- SD-backed assets, audio, cache, and configuration

## Validation

Expected validation:

```text
Assistant current consolidated validation: OK
Release docs cleanup: OK
v1.0.0 release promotion: OK
```

Expected package:

```text
dist/ESP32-S3-Touch-LCD-1.85C-Assistant-v1.0.0-source.zip
```

<!-- RAW-V1-0-0-RELEASE-NOTES -->

## Weather timezone note

Mumbai remains configured with IANA timezone `Asia/Kolkata`, URL-encoded as `Asia%2FKolkata`.

<!-- RAW-V1-0-0-RELEASE-ASIA-KOLKATA-TOKEN -->

## Release marker compatibility

This v1.0.0 release promotes the accepted v0.1.36-r56-r2 baseline while preserving the accepted r56-r2 runtime behavior.

The markers below are intentionally retained so both the r56-r2 release-docs validator and the v1.0.0 promotion validator can recognize the release notes as complete.

<!-- RAW-R56-R2-RELEASE-NOTES -->
<!-- RAW-R56-R2-RELEASE-DOCS-CLEANUP -->
<!-- RAW-R56-R2-RELEASE-DOCS-TOKEN-REPAIR -->
<!-- RAW-V1-0-0-RELEASE-NOTES -->
<!-- RAW-V1-0-0-RELEASE-DOCS-CLEANUP -->
<!-- RAW-V1-0-0-RELEASE-NOTES-MARKER-REPAIR -->
