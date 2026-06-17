# v0.1.3-r2a — Circular Layout Alignment and Responsive Touch Loop Build Fix

## Fix

v0.1.3-r2 introduced a helper with an invalid explicit type:

```text
esp_idf_hal::adc::ADCI1
```

`esp-idf-hal 0.45.2` exposes `ADC1`, not `ADCI1`. The safest fix is to remove the explicit helper type and restore the accepted baseline pattern: inline battery ADC reads with inferred types.

## Preserved

- accepted v0.1.2 hardware baseline
- circular UI direction
- fixed top status/header band
- consistent title band
- fixed bottom page-dot band
- centered Settings tile/list
- non-blocking dirty-render loop
- 25ms touch polling
- coordinate-based touch down/up recognition
- swipe threshold plus left/center/right tap fallback
- POWER GPIO6 experimental logging
- BOOT reserved while USB monitor is attached
