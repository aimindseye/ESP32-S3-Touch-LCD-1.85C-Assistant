# ESP32-S3-Touch-LCD-1.85C Assistant

Personal assistant firmware for the Waveshare ESP32-S3-Touch-LCD-1.85C round display board.

This repository is app-focused. It uses the known-good hardware bring-up from `aimindseye/ESP32-S3-Touch-LCD-1.85C` as the reference baseline, then evolves it into a dashboard-style assistant.

## Phase 0 Goal

Create a stable Assistant baseline from the existing Arduino/LVGL V1 firmware:

- round-screen-safe UI
- Home screen
- native SD-card MP3 player
- RTC time display
- battery readout
- backlight control
- touch navigation
- placeholder Weather app
- placeholder Assistant/Settings app

## Hardware Baseline

Initial target:

- Waveshare ESP32-S3 Touch LCD 1.85C V1
- PCM5101 audio path
- LVGL 9.5.x
- ESP32 Arduino core 3.2.x
- ESP32-audioI2S
- SD_MMC

Known V1 pins from the reference repo:

```text
I2S_BCLK = GPIO48
I2S_LRCK = GPIO38
I2S_DOUT = GPIO47
SD_CLK   = GPIO14
SD_CMD   = GPIO17
SD_D0    = GPIO16
```

Do not use the V2 ES8311 audio path when targeting V1 hardware.

## Assistant App Set

Planned top-level apps:

```text
Home
Music
Weather
Timers
Reminders
Calendar
Devices
Settings
```

Phase 0 starts with Home + Music + Weather placeholder.

## Repository Layout

```text
ESP32-S3-Touch-LCD-1.85C-Assistant/
├── firmware/
│   └── Assistant/               # Arduino/LVGL assistant sketch
├── docs/
│   ├── ASSISTANT_ARCHITECTURE.md
│   ├── PHASE_0_FOUNDATION.md
│   └── SD_CARD_LAYOUT.md
├── scripts/
│   ├── bootstrap_from_reference_repo.ps1
│   └── validate_assistant_repo.ps1
└── README.md
```

## Bootstrap

From PowerShell:

```powershell
cd C:\project\ESP32-S3-Touch-LCD-1.85C-Assistant
.\scripts\bootstrap_from_reference_repo.ps1
.\scripts\validate_assistant_repo.ps1
```

The bootstrap script imports the current working Arduino/LVGL hardware baseline from the reference repo, renames the sketch folder to `Assistant`, and creates the initial app-oriented documentation.

## First Flash Smoke Test

After bootstrapping, open:

```text
firmware/Assistant/Assistant.ino
```

Use the same Arduino settings proven in the reference repo:

```text
Partition scheme: 16M Flash (3MB APP/9.9MB FATFS)
Serial monitor:   115200 baud
Upload speed:     460800 or 115200 if unreliable
```

Expected first smoke result:

```text
Display works
Touch works
RTC label appears
Battery voltage appears
SD card mounts
MP3 playback works from SD
Home/Player UI works
```
