# Assistant Architecture

## Direction

The ESP32-S3-Touch-LCD-1.85C Assistant is a local, touch-first assistant terminal.

It should not try to run a full local LLM on-device. The ESP32 handles local UI, local music, timers, settings, and cached status. Heavy assistant intelligence can be added later through a backend API.

## Layers

```text
firmware/Assistant/
├── Assistant.ino                 # boot + orchestration only
├── board/                        # display, touch, RTC, battery, SD, audio init
├── apps/                         # Home, Music, Weather, Settings, etc.
├── services/                     # weather client, backend client, settings store
├── ui/                           # reusable LVGL widgets and round layout helpers
└── config/                       # defaults and local config examples
```

Phase 0 may begin as an imported single-sketch baseline. The first refactor should split it using the structure above.

## Apps

### Home

Displays:

```text
Time
Date
Battery
Wi-Fi status
Weather summary
Now-playing summary
Next reminder/event placeholder
```

### Music

Native SD-card music player. This is not only a remote control.

Initial scope:

```text
Scan /MUSIC for MP3 files
Now playing screen
Play / pause
Previous / next
Stop
Volume
Progress
Current track title
```

Later scope:

```text
Playlist files
Shuffle / repeat
Last-track persistence
Album art extraction/rendering
Backend/Spotify/Home Assistant remote control
```

### Weather

Weather should be online but cached.

Initial scope:

```text
Weather placeholder screen
Wi-Fi setup placeholder
Last-updated label
```

Later scope:

```text
Open-Meteo or backend-proxied weather
Current temperature
Condition
High / low
Rain chance
3-day forecast
```

### Settings

Initial scope:

```text
Backlight
Volume
Wi-Fi placeholder
Location placeholder
About screen
```

## Round Display Rules

Use a center-safe UI region and large touch targets.

Recommended content bounds:

```text
Display:      360 x 360
Safe panel:   about 216 x 300 centered
Touch target: >= 32 px high where possible
```

Avoid placing required text or controls near the circular corners.

## Hardware Safety Rules

Carry forward the reference repo rules:

```text
Do not casually drive GPIO4; it is touch interrupt path.
Respect shared I2C init order for touch, RTC, and expander.
Do not mix V1 PCM5101 and V2 ES8311 audio code.
Keep the known-good Arduino hardware baseline available.
```
