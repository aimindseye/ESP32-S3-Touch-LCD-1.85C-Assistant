# Changelog



## v1.0.0 — Stable Release

- Promotes accepted `v0.1.36-r56-r2` baseline to stable release.
- Preserves Weather label/nav-row location controls and fetch/cache behavior.
- Preserves Music WAV/MP3 playback, MP3 progress, and media touch zones.
- Preserves Internet Radio HTTP/HTTPS/M3U playback and transport controls.
- Preserves Settings detail navigation.
- Documents hardware-driven architecture and ESP32-S3 Bluetooth LE-only limitation.
- Documents why direct phone Bluetooth speaker/A2DP behavior is not supported on this hardware.
- Packages release source as `dist/ESP32-S3-Touch-LCD-1.85C-Assistant-v1.0.0-source.zip`.

<!-- RAW-V1-0-0-CHANGELOG -->

## v0.1.36-r56-r2 release docs token repair

- Docs-only repair.
- Ensures `architecture.md` includes the validator-required `Hardware-driven architecture` section.
- No firmware behavior changes.

<!-- RAW-R56-R2-RELEASE-DOCS-TOKEN-REPAIR -->

## v0.1.36-r56-r2 — Release Documentation and Repo Cleanup

- Preserves accepted firmware boot/version `v0.1.36-r56-r2`.
- Updates README and architecture documentation for the current module layout.
- Adds hardware notes and hardware-driven design rationale.
- Documents the ESP32-S3 Bluetooth Classic / A2DP limitation and Wi-Fi audio direction.
- Adds clean release packaging scripts.
- Removes stale backup files, old video/MJPEG historical docs, and scratch archive directories from the release tree after backing them up under `.cleanup/`.
- Repairs stale validator expectations after Settings action routing moved to `settings_action_router.rs`.

<!-- RAW-R56-R2-RELEASE-DOCS-CLEANUP-CHANGELOG -->

## v1.0.0 hardware timezone token repair

- Docs-only repair.
- Adds explicit `Asia/Kolkata` and `Asia%2FKolkata` Weather timezone note to `docs/HARDWARE.md`.
- No firmware behavior changes.

<!-- RAW-V1-0-0-HARDWARE-TIMEZONE-TOKEN-REPAIR -->

## v1.0.0 release notes marker repair

- Docs-only repair.
- Adds release marker compatibility comments to `docs/RELEASE_v1.0.0.md`.
- No firmware behavior changes.

<!-- RAW-V1-0-0-RELEASE-NOTES-MARKER-REPAIR-CHANGELOG -->
