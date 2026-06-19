# ESP32-S3 Touch LCD 1.85C Assistant

Version: `v0.1.14-r2-weather-guard-marker-repair`

This is a validator-only repair for the v0.1.14 UI baseline freeze.

## Issue fixed

`v0.1.14-r1` correctly removed the stale extra asset:

```text
Removing stale non-frozen RGB565 asset: home_default_base.rgb565
```

Then the freeze guard failed because it checked the old Weather timeline temperature marker:

```text
draw_text_centered_at(frame, cx, 258, entry.temp, WHITE, 2)
```

The accepted Weather timeline strip currently uses the final repaired baseline:

```text
draw_text_centered_at(frame, cx, 262, entry.temp, WHITE, 2)
```

## Repair

This package updates only the regression guard marker from `y=258` to `y=262`.

It preserves the r1 asset cleanup guard.

## Preserved accepted baseline

```text
Home      v0.1.10-r3
Weather   v0.1.9-r8-r2 final timeline strip, temp baseline y=262
Music     v0.1.11
Assistant v0.1.12
Settings  v0.1.13-r3
Touch     r12 gesture-first baseline
```

## Deploy

```powershell
cd C:\projects\ESP32-S3-Touch-LCD-1.85C-Assistant

Expand-Archive -Force .\ESP32-S3-Touch-LCD-1.85C-Assistant-v0.1.14-r2-weather-guard-marker-repair-files.zip .\_v0114r2baseline
Copy-Item -Recurse -Force .\_v0114r2baseline\ESP32-S3-Touch-LCD-1.85C-Assistant-v0.1.14-r2-weather-guard-marker-repair-files\* .

.\scripts\normalize_assistant_timestamps.ps1
.\scripts\fix_assistant_partition_path.ps1
.\scripts\validate_rust_assistant_repo.ps1
.\scripts\build_assistant_rs.ps1 -Clean
.\scripts\flash_assistant_rs.ps1 -Port COM8
```

## Expected validator output

```text
UI baseline freeze validation: OK
Rust assistant repo validation: OK
```

If the stale asset is still present, the validator may first print:

```text
Removing stale non-frozen RGB565 asset: home_default_base.rgb565
```

## Expected monitor markers

```text
v0.1.14-r2 Weather Baseline Guard Marker Repair
Screens frozen: Home r3 | Weather r8-r2 | Music v0.1.11 | Assistant v0.1.12 | Settings Option A
Renderer: hybrid RGB565 five page assets + dynamic overlays
UI baseline: frozen five-screen layout with regression guards
Asset guard: stale non-frozen RGB565 files are cleaned before validation
Weather guard: timeline temp marker aligned with accepted y=262 layout
```
