# Hardware

Target: Waveshare ESP32-S3-Touch-LCD-1.85C / 1.85C-BOX

## Board details

- MCU: ESP32-S3R8
- Flash: 16MB
- PSRAM: 8MB
- Display: ST77916 390x390 LCD
- Touch: CST816 capacitive touch
- RTC: PCF85063
- Audio: PCM5101 I2S output path
- Storage: TF / microSD card
- Connectivity: Wi-Fi and Bluetooth LE

## Hardware limitations reflected in firmware design

The ESP32-S3 supports Bluetooth LE, but Bluetooth Classic/A2DP is not supported. A normal phone Bluetooth speaker requires Bluetooth Classic A2DP sink support, so this firmware does not implement direct phone Bluetooth speaker behavior.

Recommended audio paths for this hardware are:

- local WAV/MP3 playback from SD
- Internet Radio over Wi-Fi
- future Wi-Fi audio receiver endpoint
- external Bluetooth Classic receiver module if true phone Bluetooth speaker behavior is required

## Display and memory design

The display path uses raw RGB565 rendering and SD-backed assets. This keeps the firmware small enough for the partition layout while preserving a responsive UI on the 390x390 display.

## Audio design

The PCM5101 I2S output is shared by Music and Internet Radio. The accepted design serializes stop/start transitions to prevent overlapping ownership of the audio path.

<!-- RAW-V1-0-0-HARDWARE -->

## Weather timezone configuration

The accepted Weather configuration includes Mumbai as a selectable Weather location.

Mumbai uses the IANA timezone `Asia/Kolkata`. For the Open-Meteo request URL, this is encoded as `Asia%2FKolkata`.

This timezone is intentionally documented as part of the hardware/release baseline because Weather fetch/cache behavior is validated on-device for Mumbai along with Jersey City, New York, and Edison.

<!-- RAW-V1-0-0-HARDWARE-ASIA-KOLKATA-TOKEN -->
