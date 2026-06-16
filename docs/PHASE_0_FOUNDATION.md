# Phase 0 Foundation

## Goal

Create the first Assistant firmware baseline from the known-good `ESP32-S3-Touch-LCD-1.85C` Arduino/LVGL demo.

## Accepted Starting Features

The imported reference baseline should preserve:

```text
Display init
LVGL 9 UI
Touch
Backlight
Battery readout
RTC label
SD_MMC mount
MP3 playback from SD
V1 PCM5101 audio path
Home and Player tabs
Round-screen-safe layout
```

## Phase 0 Changes

After import:

```text
Rename sketch folder from LVGL_Arduino to Assistant
Rename LVGL_Arduino.ino to Assistant.ino
Update boot title to Assistant
Rename Player tab to Music
Add Weather tab placeholder
Add Settings/About placeholder
Document SD card layout
Add validation script
```

## Phase 0 Validation

Run:

```powershell
.\scripts\validate_assistant_repo.ps1
```

Then flash through Arduino IDE or Arduino CLI and check serial output at 115200 baud.

Expected serial markers:

```text
Booting ESP32-S3 Assistant...
SD: mounted
Audio: V1 PCM5101 path initialized
```

## Out of Scope for Phase 0

```text
Full assistant backend
Voice recognition
Wi-Fi provisioning
Weather API integration
Playlist editor
Album art rendering
Persistent settings
Rust port
```
