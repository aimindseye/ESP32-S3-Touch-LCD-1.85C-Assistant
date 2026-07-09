# Changelog

## v1.0.1-r14 — Source Directory Cleanup

- Removes historical `_archive_*` component directories from the active source package.
- Removes old firmware porting notes and stale release documents.
- Removes generated `tmp/` files and duplicate documentation.
- Removes inactive Video/MJPEG C shim code and the unused `esp_jpeg` dependency from the active build path.
- Replaces the long historical validator with a concise current-state validator.
- Refreshes README, architecture, manifest, and release documentation for the accepted v1.0.1-r13 baseline.

## v1.0.1-r13 — Radio Station-Select Idle UI Repair

- After STOP, PREV, or NEXT, Internet Radio shows the selected station as ready with the `PLAY` button.
- Preserves stable r11-r1 PSRAM StreamBuffer playback and r11-r2 live UI refresh.

## v1.0.1-r12 — Main UI Extraction + Dead Archive Cleanup

- Extracts shared RGB565 drawing primitives into `ui_primitives.rs`.
- Extracts reusable high-level widgets into `ui_widgets.rs`.
- Keeps `main.rs` focused on runtime orchestration.

## v1.0.1-r11-r2 — Radio Live UI Refresh Repair

- Keeps stable r11-r1 radio playback.
- Restores conservative live UI refresh for Internet Radio status and clock.

## v1.0.1-r11-r1 — Stable Internet Radio StreamBuffer Playback

- Uses PSRAM-backed FreeRTOS StreamBuffer producer/consumer architecture.
- Separates HTTP reads from MP3 decode/I2S writes.
- Fixes producer startup by using PSRAM-backed static StreamBuffer storage.

## v1.0.1 — Touch Ghost Guard + Weather Fahrenheit Default

- Adds conservative touch ghost guard.
- Keeps Weather Fahrenheit default and previous/next location controls.
- Preserves accepted Music, Internet Radio, Settings, and sleep/wake behavior.

<!-- RAW-V1-0-1-R14-CLEAN-CHANGELOG -->

## v1.0.1-r14-r2 — Generated Components Lock Validator Repair

- Treats `firmware/assistant-rs/components_esp32s3.lock` as generated ESP-IDF component-manager build metadata.
- Keeps the file ignored/excluded from source control.
- Allows validation after `cargo build`, because ESP-IDF recreates the lock locally when `IDF_COMPONENT_MANAGER=1`.
- Preserves the r13 accepted Internet Radio and r14 clean-source baseline.

<!-- RAW-V1-0-1-R14-R2-CHANGELOG -->
