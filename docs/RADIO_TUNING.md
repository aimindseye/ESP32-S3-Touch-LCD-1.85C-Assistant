# Internet Radio tuning

Current accepted path: **v1.0.1-r13**, using the stable r11-r1/r11-r2 radio architecture.

## Accepted architecture

```text
HTTP producer task -> PSRAM-backed FreeRTOS StreamBuffer -> MP3 decode/I2S consumer -> PCM5101 I2S
```

Runtime marker:

```text
radio-r36-r32: stream_pacing=STREAMBUFFER_PSRAM_PRODUCER_CONSUMER_R11_R1
```

## Accepted runtime properties

- StreamBuffer storage is allocated from PSRAM.
- HTTP/network reads are isolated from the MP3 decode/I2S write loop.
- Mono radio streams are upmixed to stereo before PCM5101 I2S output.
- I2S runtime config uses the default ESP-IDF channel config after the rejected DMA-headroom experiment.
- Internet Radio UI refresh is conservative and does not reintroduce stutter.

## Rejected experiments retained only as historical context

- r9 custom ring producer/consumer: rejected because it could fill/spin and trigger watchdog failures.
- r10 direct-decode refill tuning: rejected because network reads still blocked the decode/write path.
- r10-r3 active I2S DMA enlargement: rejected because it caused immediate `WRITE_FAILED err=-2`.

## Test note

For normal playback tests, remove `/LOG.TXT` from the SD card. DEBUG mode is useful for short diagnostics but can affect audio timing.

<!-- RAW-V1-0-1-R14-CLEAN-RADIO-TUNING -->
