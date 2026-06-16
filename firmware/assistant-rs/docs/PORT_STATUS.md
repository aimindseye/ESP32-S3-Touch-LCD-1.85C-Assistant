# Port status

## Status summary

This repo has moved from an earlier best-effort `esp-hal` full-port concept to a **working hybrid Rust + ESP-IDF port**.

## Current working state

### Boot / runtime
- stable boot achieved
- no current reboot loop in the active design path
- PSRAM enabled and available
- app loop runs continuously

### Display
- ST77916 init works through the ESP-IDF shim
- full-screen panel rendering works
- backlight works
- touch-triggered repaints work

### Input / board peripherals
- TCA9554 EXIO probe works
- CST816 touch probe works
- CST816 config read works
- touch coordinates are received at runtime
- PCF85063 RTC probe and reads work

### Data fields now wired into the Onboard page
- flash size
- PSRAM size
- RTC time/date
- touch count / last touch
- battery ADC path
- Wi-Fi scan count
- SD presence / capacity path
- backlight percentage

## Current limitations

### UI / layout
- the remaining major issue is UI polish, not board bring-up
- header/footer clipping was observed when using edge-anchored bars
- current chosen direction is a **centered safe-content rectangle** inside the round display
- final typography / spacing still needs adjustment

### Battery
- ADC path is wired
- displayed voltage still needs validation / calibration polish

### SD
- SD probe path is wired through the ESP-IDF shim
- output formatting / presentation still needs polish
- runtime behavior should still be treated as bring-up-level rather than final product quality

### Wi-Fi
- scan count is wired and usable for diagnostics
- station networking is not yet being used for a user-facing product flow

### Audio / music / speech
- not completed
- vendor music page and speech-demo parity are still future work

## Architecture status

### Active architecture
- **display:** ESP-IDF C shim + `esp_lcd_st77916`
- **app logic:** Rust
- **framebuffer:** PSRAM-backed
- **UI:** software-rendered diagnostics / Onboard page
- **memory strategy:** prefer PSRAM for large allocations and reduce pressure on internal RAM

### Retired / secondary architecture
- pure Rust `esp-hal` display path is no longer the active direction for this repo
- older LVGL-first/full-port assumptions are now historical context, not the recommended implementation path

## Milestones completed

- working display init
- working backlight control
- working touch and touch-driven repaint
- working RTC read loop
- working hybrid Rust + ESP-IDF integration
- PSRAM-enabled framebuffer path
- Wi-Fi count integration
- SD probe integration
- stable runtime after stack / heap fixes

## Milestones remaining

### To finish this effort
1. complete centered safe-content layout
2. polish Onboard page visuals
3. validate battery reading
4. clean up SD formatting

### To move toward vendor parity
1. add music page shell
2. add audio playback
3. add microphone / speech path


## Practical readiness

### Ready now
- continued UI/layout iteration
- board validation
- touch / RTC / Wi-Fi / SD diagnostics


### Not ready yet
- full vendor demo parity
