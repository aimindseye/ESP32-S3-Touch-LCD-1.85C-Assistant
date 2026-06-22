$ErrorActionPreference = "Stop"
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$firmwareDir = Join-Path $repoRoot "firmware\assistant-rs"

& (Join-Path $PSScriptRoot "clean_repo_stale_artifacts.ps1")

function Require-Text([string]$content, [string]$pattern, [string]$label) {
    if ($content -notmatch [regex]::Escape($pattern)) { throw "$label missing: $pattern" }
}
function Reject-Text([string]$content, [string]$pattern, [string]$label) {
    if ($content -match [regex]::Escape($pattern)) { throw "$label contains stale marker: $pattern" }
}

$main = Get-Content (Join-Path $firmwareDir "src\main.rs") -Raw
$model = Get-Content (Join-Path $firmwareDir "src\app\model.rs") -Raw
$ffi = Get-Content (Join-Path $firmwareDir "src\ffi.rs") -Raw
$shimC = Get-Content (Join-Path $firmwareDir "components\st77916_shim\st77916_shim.c") -Raw
$shimH = Get-Content (Join-Path $firmwareDir "components\st77916_shim\include\st77916_shim.h") -Raw

foreach ($pattern in @(
    'v0.1.31-r2 Stream Playback Polish + Log Quieting',
    'v0.1.31-r1 Streaming FPS + Source Frame Skip Runtime Tuning',
    'v0.1.31 Sequential MJPEG Streaming Worker',
    'v0.1.30-r2 MJPEG Indexed Frame Scanner Repair',
    'v0.1.30-r1 Video Worker Frame Advance Request Latch',
    'v0.1.30 Persistent SD Mount Foundation',
    'v0.1.29-r3 Video Worker Status Quieting + Version Marker Repair',
    'v0.1.29-r2 Video Worker SD Ownership Repair',
    'v0.1.29-r1 Video Page Compile Repair',
    'v0.1.29 Dedicated Video Player Screen + Worker Task Foundation',
    'v0.1.28-r3 MJPEG Playback Responsiveness Repair',
    'v0.1.28-r2 MJPEG Playback Frame Advance Visibility Repair',
    'v0.1.28-r1 Playback Frame Index Compile Repair',
    'v0.1.28 MJPEG Playback Loop, No Audio',
    'v0.1.27-r2 MJPEG Preview Visibility Repair',
    'v0.1.27-r1 MJPEG JPEG Format Enum Compile Repair',
    'v0.1.27 MJPEG First Frame Decode + Display',
    'v0.1.26 MJPEG Video Foundation',
    'v0.1.25-r1 SD Asset Cache Scope Compile Repair',
    'v0.1.25 SD-Backed UI Asset Loader + App Partition Relief',
    'v0.1.24-r7 Wi-Fi Scan Quieting + Touch Finish Normalization',
    'v0.1.24-r6-r1 Log Macro Scope Compile Repair',
    'v0.1.24-r6 Monitor Log Cleanup + Runtime Verbosity Profiles',
    'v0.1.24-r5-r1 Software Sleep Compile Repair',
    'v0.1.24-r5 Software Sleep Control',
    'v0.1.24-r4 EXIO Safe Input-Only Discovery for Unused Bits',
    'v0.1.24-r3 Button Discovery Noise Gate + EXIO Input Matrix',
    'v0.1.24-r2 Button Pin Discovery Matrix',
    'v0.1.24-r1 Power Button GPIO Input Diagnostics + Sleep Trigger Repair',
    'v0.1.24 Screen Sleep / Wake Guard',
    'v0.1.23-r12 Vendor ESP-IDF Battery ADC Parity Alignment',
    'v0.1.23-r11 V1 Schematic Battery ADC Confirmation + Physical Probe Guide',
    'v0.1.23-r10 Battery Enable Path Probe',
    'v0.1.23-r9-r1 C-Shim First ADC Ownership Probe',
    'v0.1.23-r9 ESP-IDF ADC OneShot C-Shim Parity Probe',
    'v0.1.23-r8 Battery Init Order Diagnostics Repo Alignment',
    'v0.1.23-r7 Battery ADC Pin/Enable Probe Matrix',
    'v0.1.23-r6 Vendor Battery ADC Path Alignment',
    'v0.1.23-r5-r2 Battery First-Sample Before Any SD Access Repair',
    'v0.1.23-r5-r1 Battery Isolation Compile Repair',
    'v0.1.23-r5 Battery Calibration SD Read Isolation Repair',
    'v0.1.23-r4 Battery Calibration Config',
    'v0.1.23-r3 Battery ADC Diagnostics + Calibration',
    'v0.1.23-r2 Home Battery + Settings Detail Repair',
    'v0.1.23-r1 Battery Badge Placement Repair',
    'v0.1.23 Battery Status Across Screens',
    'v0.1.22-r5 Settings Detail Clean Base Repair',
    'v0.1.22-r4 Settings Missing Icon Helper Compile Repair',
    'v0.1.22-r3 Settings SD Space Compile Repair',
    'v0.1.22-r2 Settings Detail Visual Alignment Repair',
    'v0.1.22-r1 Settings Icon Helper Compile Repair',
    'v0.1.22 Settings Details Hub + Home Simplification',
    'Home simplification: center tap refreshes weather without detail mode',
    'Settings detail visual repair: shared template, aligned rows, clipped values',
    'Settings detail clean base: old baked row/card area cleared before detail draw',
    'Battery status: non-Home screens show compact battery badge',
    'Battery badge repair: Home-only battery badge with percent text',
    'Battery settings: Device detail shows percent, USB/charging state, and voltage',
    'Battery diagnostics: raw ADC, ADC mV, calculated battery mV, estimated percent',
    'Battery calibration: /BATTERY.TXT adc_multiplier empty_mv full_mv',
    'Battery calibration: Settings Device shows CAL status',
    'Battery isolation: sample before SD calibration, then sample after calibration',
    'Battery isolation: raw=0 ignored once, last valid ADC sample retained',
    'Compile repair: battery raw/mV label helpers restored',
    'Battery first-sample: ADC sampled before weather-cache and BATTERY.TXT SD reads',
    'Battery first-sample: raw=0 never stored as valid battery sample',
    'Battery ADC path: vendor-aligned ADC1 GPIO8 DB_11 multi-sample probe',
    'Battery ADC batch: min/max/avg with ADC read error markers',
    'Battery probe matrix: GPIO1/GPIO3/GPIO7/GPIO8/GPIO9 ADC1 candidates',
    'Battery probe matrix: probe values are diagnostics only, not UI source',
    'Battery probe enable: no confirmed vendor enable GPIO, matrix uses enable=NONE',
    'Battery init order: GPIO8 ADC sampled immediately after Peripherals::take',
    'Battery init phases: pre-i2c pre-wifi pre-exio post-exio post-display',
    'Battery diagnostics repo alignment: BAT_Init before I2C/Wi-Fi/EXIO/SD/display',
    'Battery C-shim parity: native ESP-IDF adc_oneshot ADC1_CH7 GPIO8 probe',
    'Battery C-shim parity: Rust and C raw/mV logged side-by-side, UI source unchanged',
    'Battery C-first ownership: native adc_oneshot probe runs before Rust AdcDriver',
    'Battery C-first ownership: C-first and Rust-after results are compared, UI source unchanged',
    'Battery enable path: rust-diagnostics inspected, no confirmed BAT enable pin found',
    'Battery enable probe: C-first NONE baseline plus EXIO before/after init markers',
    'Battery enable probe: enable values are diagnostics only, UI source unchanged',
    'Board profile: WAVESHARE_ESP32_S3_TOUCH_LCD_1_85C_V1 schematic locked',
    'Battery schematic: BAT_ADC(GPIO8) confirmed on V1 schematic',
    'Battery divider: schematic R ladder implies multiplier about 3.0',
    'Battery UI: Device shows BAT ADC GPIO8 CONFIRMED / SIGNAL MISSING',
    'Battery C-shim parity: post-boot polling disabled to avoid ADC1-in-use spam',
    'Battery vendor ADC: GPIO8 ADC1_CH7 uses ESP-IDF ADC_ATTEN_DB_12',
    'Battery vendor ADC source: C-SHIM GPIO8 VENDOR',
    'Battery vendor ADC: Measurement_offset=0.994500 default multiplier active',
    'Battery vendor ADC: valid C-shim calibrated mV promoted as UI source',
    'Battery Rust ADC: retained as diagnostic-only comparison',
    'Screen sleep: software Sleep Now, top-left long touch, or idle timeout turns backlight off',
    'Screen wake: touch interrupt turns backlight on',
    'Screen wake guard: first wake touch is swallowed, navigation blocked',
    'Screen sleep policy: background RTC/Wi-Fi/weather/battery services continue',
    'Power GPIO diagnostics: GPIO6 configured with ESP-IDF gpio_config input pull-up',
    'Power GPIO diagnostics: raw level and active-low duration logged',
    'Power GPIO diagnostics: report-only, not a sleep trigger',
    'Software sleep: Settings > Display > Sleep Now active',
    'Software sleep: optional top-left long-touch gesture active',
    'Compile repair: Settings Display Sleep Now match arm comma restored',
    'Monitor log cleanup: NORMAL default, DEBUG via compile constant or /LOG.TXT',
    'compile-repair: debug_println macro defined before TouchTracker use',
    'wifi-scan: NORMAL profile skips scans; DEBUG scans only after station is connected',
    'touch-normalization: NORMAL profile emits compact touch finish lines; DEBUG keeps full geometry',
    'asset-loader: SD-backed UI assets path=/ASSETS names=HOME.RGB,WEATHER.RGB,MUSIC.RGB,AI.RGB,SETTINGS.RGB',
    'compile-repair: UiAssetCache allocated in run_app before first render',
    'video-foundation: path=/VIDEO formats=MJPG,MJPEG,MJP first-frame-boundary-scan=ENABLED playback=DEFERRED audio=DEFERRED',
    'video-decode: first-frame JPEG decode enabled on Settings > Storage preview; playback=DEFERRED audio=DEFERRED',
    'compile-repair: esp_jpeg output format enum uses JPEG_IMAGE_FORMAT_RGB565',
    'video-preview: dedicated visible zone enabled; opaque Storage card skipped when decoded; RGB565 swap via /VIDEO/SWAP.TXT',
    'video-playback: dedicated Video page worker playback fps={} audio=DEFERRED stop=TOUCH_OR_SWIPE',
    'compile-repair: VIDEO_PLAYBACK_FRAME_INDEX static allocated for MJPEG frame loop',
    'video-playback-visibility: requested_frame and decoded_frame logged separately; overlay shows tick/requested/decoded; frame_skip={} audio=DEFERRED',
    'video-playback-responsiveness: touch-priority=ENABLED touch-stop=IMMEDIATE fps={} state=STOPPED,PLAYING,PAUSED,TOUCH_STOP config=FPS_TXT_DEFERRED,SKIP_TXT_DEFERRED',
    'video-worker: dedicated Video page enabled; worker task decodes into PSRAM; UI displays latest frame; settings-storage=STATUS_PREVIEW_ONLY audio=DEFERRED',
    'video-worker-sd-owner: worker SD mount/unmount disabled; single SD owner mutex foundation enabled; decode deferred until persistent SD mount; ui_status=WORKER_SD_OWNER',
    'video-worker-quiet: repeated worker status logs throttled log_every={} stable_ui_status=WORKER_SD_OWNER no_crash_expected=true',
    'sd-persistent-foundation: single_mount_session=ENABLED owner_mutex=REAL legacy_mount_calls=COMPAT_REUSE worker_reads=ALREADY_MOUNTED_PATH settings_storage_preview=PRESERVED',
    'video-worker-request-latch: next-frame requests latch while BUSY pending=ONE requested_frame_and_decoded_frame_logged=YES fps=2 sd_reinit=NO',
    'mjpeg-indexed-scanner: requested_frame_maps_to_actual_frame=YES eof_loop_to_zero=YES offset_size_logged=YES fps=2 sd_reinit=NO',
    'mjpeg-stream-worker: sequential=ENABLED open_once=YES rolling_buffer=64KB scan_from_start=NO publish_latest_rgb565=YES fps=2 audio=DEFERRED',
    'video-stream-runtime-tuning: fps_txt=/VIDEO/FPS.TXT default_fps=5 skip_txt=/VIDEO/SKIP.TXT default_skip=6 source_fps=30 decode_selected_only=YES no_audio=YES',
    'video-stream-polish: normal_log_every=30 debug_per_frame=YES eof_wrap_actual_frame_0=YES timing=read_ms,skip_ms,decode_ms,publish_ms fps_tests=6x5,8x4',
    'compile-repair: Video page added to ALL_PAGES length=6 and select-action match',
    'partition-relief: before_r7_app_bytes={} partition_bytes={} embedded_rgb565_removed_bytes={} estimated_free_gain_kib={}',
    'Button discovery matrix: GPIO0 BOOT, GPIO6 POWER, GPIO7, GPIO9 monitored',
    'Button discovery matrix: report-only, no candidate bound to sleep yet',
    'Button discovery matrix: GPIO0 BOOT never used for sleep while USB flashing is attached',
    'Button discovery noise gate: boot level is idle baseline for every candidate',
    'Button discovery noise gate: boot-active candidates marked noisy and not hold-spammed',
    'EXIO input matrix: TCA9554 input register is polled read-only, no output toggles',
    'EXIO safe discovery: preserve EXIO0..2 outputs and configure EXIO3..7 inputs',
    'EXIO safe discovery: input mask 0xF8, protected output mask 0x07',
    'EXIO safe discovery: report-only, no sleep binding until physical transition confirmed',
    'Battery power: USB/UNKNOWN until charger GPIO is validated',
    'Battery refresh: visible page redraws on battery change',
    'Storage detail: SD free/total space shown when available'
)) { Require-Text $main $pattern "v0.1.22-r2 marker" }

foreach ($pattern in @(
    'struct SettingsDetailRow',
    'draw_settings_detail_template',
    'draw_settings_detail_clean_base',
    'draw_settings_detail_row',
    'settings_clip',
    'draw_settings_network_detail',
    'draw_settings_weather_detail',
    'draw_settings_time_detail',
    'draw_settings_storage_detail',
    'draw_settings_device_detail',
    'draw_settings_diagnostics_detail',
    'fn draw_settings_time_icon',
    'fn draw_settings_storage_icon',
    'fn draw_settings_diag_icon',
    'row x=64 width=232 height=30',
    'SD FREE / TOTAL'
)) { Require-Text $main $pattern "shared settings detail template" }

foreach ($pattern in @(
    'pub sd_free_mb: Option<u32>',
    'sd_total_text',
    'sd_free_text',
    'sd_free_total_text'
)) { Require-Text $model $pattern "SD free total model" }

foreach ($pattern in @('st77916_probe_sd_space_mb', 'out_total_mb', 'out_free_mb')) {
    Require-Text $ffi $pattern "SD space ffi"
    Require-Text $shimH $pattern "SD space header"
    Require-Text $shimC $pattern "SD space c"
}

Require-Text $shimC 'f_getfree' "SD FatFs free-space probe"
Require-Text $shimC '#include "ff.h"' "SD FatFs header"
Reject-Text $shimC 'sys/statvfs.h' "unsupported statvfs header"
Require-Text $main 'ffi::st77916_probe_sd_space_mb' "refresh SD free/total"
Reject-Text $main 'draw_text_centered_at(frame, 180, 292, &model.weather.hourly_summary()' "long weather detail summary"
Reject-Text $main 'draw_settings_key_value(frame, 56' "old unaligned settings rows"

Write-Host "Settings detail visual alignment validation: OK"
