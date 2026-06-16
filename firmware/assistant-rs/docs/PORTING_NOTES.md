# Porting notes

## Scope

This project started as a Rust-first attempt to mirror the Waveshare vendor demos, but the active implementation strategy changed during bring-up.

The repo now uses a **hybrid porting strategy**:

- let **ESP-IDF** handle the hardest vendor-aligned platform pieces
- keep **Rust** responsible for board orchestration and application logic

## Original vendor demo scope

The vendor demo validates onboard device functionality such as:

- SD card
- flash size
- battery voltage
- RTC time
- Wi-Fi scan
- backlight brightness
- page switching / UI behavior
- music / MP3 playback
- speech-related demo behavior

## Current interpretation of that scope

This repo now treats the vendor demo in phases.

### Phase 1: hardware/platform parity
Completed or substantially wired:
- EXIO
- touch
- RTC
- panel init
- backlight
- PSRAM
- battery ADC path
- Wi-Fi scan count
- SD presence/capacity path
- diagnostic Onboard page

### Phase 2: UX/layout completion
Current focus:
- finish the centered safe-content panel layout for the round display
- finalize the Onboard page enough to close the porting effort cleanly

### Phase 3: optional vendor-demo parity extensions
Future:
- music page
- audio playback
- microphone / speech
- product polish

## Why the design changed

### What proved expensive
The biggest risk was the display path:

- ST77916 on QSPI
- vendor-specific init behavior
- round display layout expectations
- memory pressure once Wi-Fi and storage were added

### What worked better
A hybrid model proved much more effective:

- ESP-IDF-native display path through a minimal shim
- Rust for:
  - EXIO
  - touch
  - RTC
  - battery
  - Wi-Fi scan orchestration
  - SD probe orchestration
  - page model
  - redraw logic

This kept the project moving once the display path became the bottleneck.

## Important implementation lessons

### 1. Full framebuffer + Wi-Fi requires memory strategy
A full `360x360` RGB565 page buffer is large enough to matter on this board.

The current project direction therefore uses:
- PSRAM
- PSRAM-backed framebuffer allocation
- ESP-IDF memory preferences for Wi-Fi/LWIP and FATFS

### 2. Main-task stack size matters
Running too much logic directly in the ESP-IDF main task caused a stack overflow during the bring-up iterations.

The current shape therefore keeps:
- a lighter main task
- the real app loop on a larger Rust thread stack

### 3. Round screens should not be treated like rectangles
Edge-anchored header/footer bars were repeatedly clipped by the physical bezel.

The chosen UI strategy is now:
- render the full `360x360` screen
- keep the real content inside a centered safe-content rectangle
- let the bezel hide the corners

### 4. Battery / SD / Wi-Fi are now app-level diagnostics fields
These fields are now part of the practical Onboard page, not theoretical parity goals.

## Build / toolchain notes

### Active build path
- Espressif Rust toolchain
- managed ESP-IDF through `esp-idf-sys`
- extra component under `components/st77916_shim`

### Important project-level settings
- PSRAM enabled
- Wi-Fi/LWIP prefer SPIRAM
- FATFS prefer external RAM
- main-task stack increase as a safety margin

### Historical build friction
During bring-up, common issues included:
- missing linker/tool install pieces
- certificate/download failures while managed ESP-IDF tools were being fetched
- symbol/linking failures before the shim path settled
- stack overflow before the thread/stack approach stabilized

These are no longer the architectural direction of the project, but they are worth remembering if the build environment is recreated from scratch.

## Current UI approach

The current Onboard page is intentionally pragmatic:
- it is not the final OS UI
- it is meant to close the porting effort
- it validates that the board can support a future UI/runtime stack cleanly

The current final-direction layout is:
- one centered content panel
- compact header inside the panel
- 2-column diagnostics card grid
- compact footer/status row inside the panel

## What still needs work before closing the port

1. final panel sizing / spacing on the round display
2. header/footer polish
3. battery reading validation
4. SD value presentation polish


## Recommendation going forward

Do not reopen the old pure-display/full-port direction unless absolutely necessary.

The better path is:
1. finish the current hybrid Onboard page cleanly
2. freeze a stable board-support baseline
