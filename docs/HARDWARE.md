# Hardware Notes

Current accepted firmware: `v0.1.36-r56-r2`

## Target board

Project target: Waveshare ESP32-S3 Touch LCD 1.85C / 1.85C-BOX class hardware.

The firmware has been validated on an ESP32-S3 device reporting:

```text
chip: ESP32-S3 rev v0.2
flash: 16 MB
PSRAM: 8 MB
Wi-Fi: 2.4 GHz station mode used by firmware
Bluetooth: BLE-only on ESP32-S3; no Bluetooth Classic / A2DP
```

## Main components used by the firmware

```text
MCU              ESP32-S3 / ESP32-S3R8 class
Display          ST77916 round LCD, 390 x 390 firmware UI target
Touch            CST816 capacitive touch controller
RTC              PCF85063
Audio output     PCM5101 I2S path in the accepted firmware baseline
Storage          SD / TF card
GPIO expander    TCA9554 family where present
Battery source   C-SHIM GPIO8 VENDOR reporting path in accepted logs
```

Some Waveshare variants document ES8311/ES7210 audio codec hardware. This firmware baseline uses the accepted PCM5101 I2S output path that has been physically validated in Music and Internet Radio tests.

## Design consequences

### Display

The firmware uses raw RGB565 page bases plus lightweight dynamic overlays. This avoids pulling in large UI dependencies and keeps the app partition under control.

### Touch

Touch gestures/taps are centrally summarized and routed through feature-specific modules. This keeps page navigation independent from Weather, Settings, Music, and Internet Radio action handling.

### Audio

The I2S output path is shared. MP3 decode and Internet Radio stream workers serialize stop/disable ownership so user stop/next/play actions do not race the audio thread.

### SD card

The SD card is part of the product architecture, not just a debug aid. It carries user media, radio station lists, Wi-Fi credentials import, battery calibration, optional assets, and debug/profile files.

### Bluetooth

ESP32-S3 Bluetooth is BLE-only. Standard phone Bluetooth speaker behavior requires Bluetooth Classic A2DP sink support, which this target does not provide. Use Wi-Fi audio/HTTP streaming for a software-only future receiver feature, or add external Bluetooth Classic hardware for true A2DP speaker behavior.

## Time zones

The Weather provider uses fixed configured locations. Mumbai is configured as:

```text
label: MUMBAI
timezone: Asia/Kolkata
Open-Meteo URL value: Asia%2FKolkata
```
