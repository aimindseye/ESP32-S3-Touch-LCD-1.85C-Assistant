# v1.1.0 — Clean Source + Stable Internet Radio

## Highlights

- Stabilized Internet Radio playback using the PSRAM-backed StreamBuffer producer/consumer path.
- Confirmed Internet Radio playback without stutter.
- Preserved live Internet Radio UI refresh for clock/status updates.
- Fixed station-select UX so STOP/NEXT/PREV show the selected station as ready with PLAY instead of stale STOP.
- Completed main.rs/UI refactor and source-directory cleanup.
- Removed old archive directories, stale Video/MJPEG paths, obsolete scripts, and obsolete documents.
- Updated README, architecture, changelog, release docs, and radio tuning docs.
- Fixed validation after local ESP-IDF builds regenerate components_esp32s3.lock.

## Runtime marker

radio-r36-r32: stream_pacing=STREAMBUFFER_PSRAM_PRODUCER_CONSUMER_R11_R1

## Notes

Keep /LOG.TXT removed from the SD card for normal audio testing. DEBUG logging remains available but can affect audio timing.
