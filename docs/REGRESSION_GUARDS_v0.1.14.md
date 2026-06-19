# v0.1.14 Regression Guards

The main regression guard is:

```powershell
.\scripts\validate_ui_baseline_freeze.ps1
```

It is also invoked by:

```powershell
.\scripts\validate_rust_assistant_repo.ps1
```

## Guard coverage

```text
- exactly five RGB565 assets
- each asset size is exactly 259200 bytes
- each accepted asset SHA256 is frozen
- Home accepted layout markers
- Weather accepted timeline-strip markers
- Music Option C markers
- Assistant Option B markers
- Settings Option A/r3 alignment markers
- five-page order
- r12 touch constants and classifier markers
- concise boot banner markers
- forbidden stale boot/log markers
- forbidden LVGL / esp_lvgl_port markers
- forbidden periodic SD/GPIO refresh markers
```

## Intent

This release should not change visuals. It protects the currently accepted UI before future feature work begins.
