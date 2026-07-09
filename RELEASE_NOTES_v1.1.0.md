# v1.1.0 — Clean Source + Stable Internet Radio

## Highlights

- Stabilized Internet Radio playback using the PSRAM-backed StreamBuffer producer/consumer path.
- Confirmed Internet Radio playback without stutter.
- Preserved live Internet Radio UI refresh for clock/status updates.
- Fixed station-select UX so STOP/NEXT/PREV show the selected station as ready with PLAY instead of stale STOP.
- Completed main.rs/UI refactor by extracting UI primitives/widgets and reducing active main.rs size.
- Cleaned source tree by removing old archive directories, stale Video/MJPEG paths, obsolete scripts, and obsolete documents.
- Updated README, architecture, changelog, release docs, and radio tuning docs.
- Fixed clean-source validation after local builds.

## Accepted baseline

- Pages: Home, Weather, Music, InternetRadio, Assistant, Settings.
- Music WAV/MP3 playback, MP3 progress, and Music volume controls are preserved.
- Internet Radio HTTP/HTTPS/M3U playback is stable without stutter.
- Internet Radio live time/status refresh is preserved.
- Weather Fahrenheit default, Weather location controls, touch ghost guard, Settings detail clean-base, and software sleep/wake are preserved.

## Runtime marker

radio-r36-r32: stream_pacing=STREAMBUFFER_PSRAM_PRODUCER_CONSUMER_R11_R1

## Validation marker

v1.0.1-r14-r2 generated components lock validator repair: OK

## Notes

Keep /LOG.TXT removed from the SD card for normal audio testing. DEBUG logging is still available through /LOG.TXT, but it can affect audio timing.
