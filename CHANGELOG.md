# Changelog


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
