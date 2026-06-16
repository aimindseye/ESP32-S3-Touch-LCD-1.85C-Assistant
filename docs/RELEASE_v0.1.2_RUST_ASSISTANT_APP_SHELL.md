# v0.1.2 — Rust Assistant App Shell

## Scope

- Preserve accepted v0.1.1 hardware baseline.
- Rename the Onboard page to Home.
- Add an Assistant app/page enum.
- Add Home / Weather / Music / Settings placeholder pages.
- Keep display, touch, RTC, battery, Wi-Fi, and SD behavior unchanged.
- Add build/flash helper scripts.
- Add validator for Rust repo structure, partition table, and ST77916 shim linkage.

## Accepted baseline preserved

v0.1.1 was physically validated on Waveshare ESP32-S3-Touch-LCD-1.85C hardware:

- Rust firmware builds and flashes over COM8.
- Custom 16MB flash partition table works.
- 3MB factory app partition accepts the current firmware image.
- 8MB PSRAM is detected and used for the framebuffer.
- TCA9554, CST816, and PCF85063 I2C probes pass.
- ST77916 panel initializes.
- Backlight turns on.
- Home page renders.
- RTC reads update the UI.
- Touch events select Home cards or switch pages.
- Repaint path works.

## Files

Replacement files:

- `firmware/assistant-rs/src/main.rs`
- `firmware/assistant-rs/src/app/model.rs`
- `firmware/assistant-rs/src/app/mod.rs`
- `firmware/assistant-rs/Cargo.toml`
- `firmware/assistant-rs/build.rs`
- `firmware/assistant-rs/.cargo/config.toml`
- `firmware/assistant-rs/sdkconfig.defaults`
- `firmware/assistant-rs/partitions.csv`

New files:

- `firmware/assistant-rs/src/app/pages.rs`
- `scripts/fix_assistant_partition_path.ps1`
- `scripts/build_assistant_rs.ps1`
- `scripts/flash_assistant_rs.ps1`
- `scripts/validate_rust_assistant_repo.ps1`
- `docs/RELEASE_v0.1.2_RUST_ASSISTANT_APP_SHELL.md`

## Apply

Copy the contents of this ZIP into the repository root:

```powershell
cd C:\projects\ESP32-S3-Touch-LCD-1.85C-Assistant
Expand-Archive -Force .\ESP32-S3-Touch-LCD-1.85C-Assistant-v0.1.2-files.zip .\_v012
Copy-Item -Recurse -Force .\_v012\ESP32-S3-Touch-LCD-1.85C-Assistant-v0.1.2-files\* .
```

Fix the local absolute partition path and validate:

```powershell
.\scripts\fix_assistant_partition_path.ps1
.\scripts\validate_rust_assistant_repo.ps1
```

Build:

```powershell
.\scripts\build_assistant_rs.ps1
```

Flash and monitor:

```powershell
.\scripts\flash_assistant_rs.ps1 -Port COM8
```

## Expected smoke test

Serial monitor should show:

```text
Hybrid Rust + ESP-IDF display backend
Assistant app shell with Home/Weather/Music/Settings pages
Home page preserves PSRAM framebuffer + real BAT/WIFI/SD behavior
I2C probes:
  0x20 TCA9554  => Ok(())
  0x15 CST816   => Ok(())
  0x51 PCF85063 => Ok(())
panel init ok
backlight on
home page rendered
```

Touching the top tabs should print `page: Weather`, `page: Music`, or `page: Settings` and repaint the placeholder page.
