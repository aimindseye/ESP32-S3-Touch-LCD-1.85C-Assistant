# Architecture

Current accepted firmware: **v1.0.1-r13**

## Page model

The product page order is:

```text
Home -> Weather -> Music -> InternetRadio -> Assistant -> Settings -> Home
```

The Video page and MJPEG worker path have been removed from the active product architecture.

## Runtime ownership model

The firmware stays intentionally single-owner for UI state, touch handling, and display rendering. Screens are render modules, not independent threads. Background work is limited to subsystems that require it, such as Wi-Fi/time/weather/audio/radio streaming.

## UI/rendering

The UI uses raw RGB565 drawing for predictable RAM/flash behavior on the round ST77916 display. Active drawing code is split into:

- `ui_primitives.rs`: colors, low-level primitives, glyph/text helpers, and numeric drawing
- `ui_widgets.rs`: reusable watch/weather/music/settings widgets and page dots
- `page_assets.rs`: SD/embedded page-base rendering cache
- `screens/*`: page-specific renderers

`main.rs` now focuses on boot, hardware setup, event polling, page orchestration, and render scheduling.

## Touch routing

Touch input is classified by `touch_router.rs` and routed through focused action modules:

- `media_action_router.rs` for Music and Internet Radio
- `settings_action_router.rs` for Settings
- `weather_action_router.rs` for Weather

The accepted touch ghost guard remains active. Weather center-unit toggling remains disabled; Weather defaults to Fahrenheit.

## Audio architecture

Music and Internet Radio share the PCM5101 I2S output. Stop/start ownership is serialized to prevent concurrent use of the DAC path.

Local Music uses the accepted WAV/MP3 playback path with HELIX MP3 progress.

Internet Radio uses the accepted r11-r1/r11-r2 architecture:

```text
HTTP producer task -> PSRAM-backed FreeRTOS StreamBuffer -> MP3 decode/I2S consumer
```

This keeps network reads out of the decode/write timing path and avoids the rejected r9 custom ring-buffer watchdog failure. The accepted radio UI refresh is conservative and updates status/time without reintroducing stutter.

## Storage

The SD card holds user content and optional configuration:

- `/AUDIO/*.WAV` and `/AUDIO/*.MP3`
- `/AUDIO/RADIO~1.TXT`
- `/WIFI.TXT`
- `/BATTERY.TXT`
- `/ASSETS/*.rgb565`
- `/LOG.TXT` for DEBUG profile

## C shim boundaries

ESP-IDF C shims are kept only for hardware/subsystem operations that are safer or already validated in C:

- ST77916 panel and SD helpers
- PCM5101 I2S output
- HELIX MP3 decode glue
- Internet Radio HTTP/HTTPS/playlist streaming

Dead MJPEG/video C shim code, historical `_archive_*` directories, and stale validator compatibility blocks have been removed from the cleaned source tree.

<!-- RAW-V1-0-1-R14-CLEAN-ARCHITECTURE -->
