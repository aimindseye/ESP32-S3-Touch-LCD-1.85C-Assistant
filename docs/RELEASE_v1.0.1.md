# v1.0.1-r13 — Internet Radio Stable + Main UI Cleanup

This is the accepted firmware baseline after the Internet Radio stabilization and source cleanup work.

## Highlights

- Stable Internet Radio playback using a PSRAM-backed FreeRTOS StreamBuffer producer/consumer path.
- Internet Radio supports HTTP, HTTPS playlist/M3U resolution, full station names, station cache, live UI refresh, and corrected station-select PLAY/STOP UX.
- Music WAV/MP3 playback remains accepted with HELIX MP3 progress.
- Weather remains Fahrenheit by default with `Weather Location` previous/next controls.
- Touch ghost guard, Settings detail pages, and software sleep/wake remain accepted.
- Main UI drawing helpers are split out of `main.rs` into focused modules.
- Video/MJPEG code and historical source archives are removed from the cleaned source package.

## Runtime marker

```text
radio-r36-r32: stream_pacing=STREAMBUFFER_PSRAM_PRODUCER_CONSUMER_R11_R1
```

## Validation

```bash
./scripts/validate_assistant_current.sh
cd firmware/assistant-rs
cargo build --release
```

## Flash

```bash
cd firmware/assistant-rs
cargo espflash flash --release --monitor --port /dev/cu.usbmodem2101
```

## Normal-mode audio note

Keep `/LOG.TXT` removed for normal audio-quality validation. DEBUG logging is available for short diagnostics but can affect audio timing.

<!-- RAW-V1-0-1-R14-CLEAN-RELEASE-DOC -->

## v1.0.1-r14-r2 generated components lock validator repair

`v1.0.1-r14-r2` fixes the clean-source validator after local builds. ESP-IDF component manager recreates `firmware/assistant-rs/components_esp32s3.lock` during build; validation now allows that generated local file while ensuring it remains ignored/untracked.

<!-- RAW-V1-0-1-R14-R2-RELEASE-NOTE -->
