# Firmware: Assistant

This folder should contain the Arduino/LVGL Assistant sketch.

Bootstrap it from the reference repo with:

```powershell
.\scripts\bootstrap_from_reference_repo.ps1
```

The imported sketch starts from the known-good V1 baseline with Display, Touch, RTC, SD_MMC, Battery, Backlight, and MP3 playback.

First refactor target:

```text
Assistant.ino
board/
apps/
ui/
services/
config/
```
