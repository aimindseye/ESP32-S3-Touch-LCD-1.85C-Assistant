# v0.1.14 Five-Screen UI Baseline Freeze

This document freezes the accepted visual/UI baseline for the ESP32-S3 Touch LCD 1.85C Assistant.

## Frozen screen order

```text
Home -> Weather -> Music -> Assistant -> Settings
```

## Accepted screen baselines

```text
Home      v0.1.10-r3  Home Date and Weather Alignment Repair
Weather   v0.1.9-r8-r2 Weather Large Timeline Strip
Music     v0.1.11     Music Screen Option C Minimal Equalizer
Assistant v0.1.12     AI Assistant Option B Conversation Card
Settings  v0.1.13-r3  Settings Baseline Alignment and Log Cleanup
```

## Frozen RGB565 base assets

| Asset | SHA256 |
|---|---|
| `home_base.rgb565` | `e08c25e66648989237b310728f0048593aad54dbdb2444397d8eede64b4d7744` |
| `weather_base.rgb565` | `1d1e82c3936e281c7e3affc9cbe099c81c80d213aacd2d73a5e972b1033424ce` |
| `music_base.rgb565` | `5757144b865cce59569be9504da650d77113d9bdfc4e026e626c3ee82a4c9a3a` |
| `assistant_base.rgb565` | `7e24c36c419d2832fc7b3df09f5d0d191849efc1dac2c6098b676af3591003fa` |
| `settings_base.rgb565` | `54bafea9208ad49c57d34dac40865988b4967bd775aaad9a0a5c06bba9fe58e5` |

Each asset must remain exactly `259200` bytes: `360 * 360 * 2`.

## Frozen runtime policies

```text
- r12 gesture-first touch baseline is preserved
- five-page order is preserved
- hybrid RGB565 renderer remains primary
- raw renderer fallback remains available
- mocked/local integrations only
- no LVGL path
- no periodic SD/GPIO refresh
- concise boot banner only
```
