#!/usr/bin/env bash
set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
firmware_dir="$repo_root/firmware/assistant-rs"
asset_dir="$firmware_dir/assets/rgb565"

"$script_dir/clean_repo_stale_artifacts.sh"

require_text() {
  local file="$1"; local pattern="$2"; local label="$3"
  grep -Fq "$pattern" "$file" || { echo "$label missing: $pattern" >&2; exit 1; }
}

reject_text() {
  local file="$1"; local pattern="$2"; local label="$3"
  if grep -Fq "$pattern" "$file"; then echo "$label contains stale marker: $pattern" >&2; exit 1; fi
}

require_file() {
  local file="$1"; local label="$2"
  [[ -f "$file" ]] || { echo "$label missing: $file" >&2; exit 1; }
}

expected_hash() {
  case "$1" in
    home_base.rgb565) echo "e08c25e66648989237b310728f0048593aad54dbdb2444397d8eede64b4d7744" ;;
    weather_base.rgb565) echo "1d1e82c3936e281c7e3affc9cbe099c81c80d213aacd2d73a5e972b1033424ce" ;;
    music_base.rgb565) echo "5757144b865cce59569be9504da650d77113d9bdfc4e026e626c3ee82a4c9a3a" ;;
    assistant_base.rgb565) echo "7e24c36c419d2832fc7b3df09f5d0d191849efc1dac2c6098b676af3591003fa" ;;
    settings_base.rgb565) echo "54bafea9208ad49c57d34dac40865988b4967bd775aaad9a0a5c06bba9fe58e5" ;;
    *) return 1 ;;
  esac
}

main="$firmware_dir/src/main.rs"
require_text "$firmware_dir/Cargo.toml" 'version = "0.1.23"' "Cargo version"
model="$firmware_dir/src/app/model.rs"
actions="$firmware_dir/src/app/actions.rs"
pages="$firmware_dir/src/app/pages.rs"
model="$firmware_dir/src/app/model.rs"
ffi="$firmware_dir/src/ffi.rs"
shim_c="$firmware_dir/components/st77916_shim/st77916_shim.c"
shim_h="$firmware_dir/components/st77916_shim/include/st77916_shim.h"

asset_count="$(find "$asset_dir" -maxdepth 1 -type f -name '*.rgb565' | wc -l | tr -d ' ')"
[[ "$asset_count" = "5" ]] || { echo "Expected exactly 5 RGB565 assets, found $asset_count" >&2; exit 1; }
for asset in "$asset_dir"/*.rgb565; do
  name="$(basename "$asset")"; expected="$(expected_hash "$name" || true)"
  [[ -n "$expected" ]] || { echo "Unexpected RGB565 asset: $name" >&2; exit 1; }
  actual="$(shasum -a 256 "$asset" | awk '{print $1}')"
  [[ "$actual" = "$expected" ]] || { echo "RGB565 hash changed for $name" >&2; exit 1; }
done

for pattern in \
  'v0.1.31-r2 Stream Playback Polish + Log Quieting' \
  'v0.1.31-r1 Streaming FPS + Source Frame Skip Runtime Tuning' \
  'v0.1.31 Sequential MJPEG Streaming Worker' \
  'v0.1.30-r2 MJPEG Indexed Frame Scanner Repair' \
  'v0.1.30-r1 Video Worker Frame Advance Request Latch' \
  'v0.1.30 Persistent SD Mount Foundation' \
  'v0.1.29-r3 Video Worker Status Quieting + Version Marker Repair' \
  'v0.1.29-r2 Video Worker SD Ownership Repair' \
  'v0.1.29-r1 Video Page Compile Repair' \
  'v0.1.29 Dedicated Video Player Screen + Worker Task Foundation' \
  'v0.1.28-r3 MJPEG Playback Responsiveness Repair' \
  'v0.1.28-r2 MJPEG Playback Frame Advance Visibility Repair' \
  'v0.1.28-r1 Playback Frame Index Compile Repair' \
  'v0.1.28 MJPEG Playback Loop, No Audio' \
  'v0.1.27-r2 MJPEG Preview Visibility Repair' \
  'v0.1.27-r1 MJPEG JPEG Format Enum Compile Repair' \
  'v0.1.27 MJPEG First Frame Decode + Display' \
  'v0.1.26 MJPEG Video Foundation' \
  'v0.1.25-r1 SD Asset Cache Scope Compile Repair' \
  'v0.1.25 SD-Backed UI Asset Loader + App Partition Relief' \
  'v0.1.24-r7 Wi-Fi Scan Quieting + Touch Finish Normalization' \
  'v0.1.24-r6-r1 Log Macro Scope Compile Repair' \
  'v0.1.24-r6 Monitor Log Cleanup + Runtime Verbosity Profiles' \
  'v0.1.24-r5-r1 Software Sleep Compile Repair' \
  'v0.1.24-r5 Software Sleep Control' \
  'v0.1.24-r4 EXIO Safe Input-Only Discovery for Unused Bits' \
  'v0.1.24-r3 Button Discovery Noise Gate + EXIO Input Matrix' \
  'v0.1.24-r2 Button Pin Discovery Matrix' \
  'v0.1.24-r1 Power Button GPIO Input Diagnostics + Sleep Trigger Repair' \
  'v0.1.24 Screen Sleep / Wake Guard' \
  'v0.1.23-r12 Vendor ESP-IDF Battery ADC Parity Alignment' \
  'v0.1.23-r11 V1 Schematic Battery ADC Confirmation + Physical Probe Guide' \
  'v0.1.23-r10 Battery Enable Path Probe' \
  'v0.1.23-r9-r1 C-Shim First ADC Ownership Probe' \
  'v0.1.23-r9 ESP-IDF ADC OneShot C-Shim Parity Probe' \
  'v0.1.23-r8 Battery Init Order Diagnostics Repo Alignment' \
  'v0.1.23-r7 Battery ADC Pin/Enable Probe Matrix' \
  'v0.1.23-r6 Vendor Battery ADC Path Alignment' \
  'v0.1.23-r5-r2 Battery First-Sample Before Any SD Access Repair' \
  'v0.1.23-r5-r1 Battery Isolation Compile Repair' \
  'v0.1.23-r5 Battery Calibration SD Read Isolation Repair' \
  'v0.1.23-r4 Battery Calibration Config' \
  'v0.1.23-r3 Battery ADC Diagnostics + Calibration' \
  'v0.1.23-r2 Home Battery + Settings Detail Repair' \
  'v0.1.23-r1 Battery Badge Placement Repair' \
  'v0.1.23 Battery Status Across Screens' \
  'v0.1.22-r5 Settings Detail Clean Base Repair' \
  'v0.1.22-r4 Settings Missing Icon Helper Compile Repair' \
  'v0.1.22-r3 Settings SD Space Compile Repair' \
  'v0.1.22-r2 Settings Detail Visual Alignment Repair' \
  'v0.1.22-r1 Settings Icon Helper Compile Repair' \
  'v0.1.22 Settings Details Hub + Home Simplification' \
  'Home simplification: center tap refreshes weather without detail mode' \
  'Settings detail visual repair: shared template, aligned rows, clipped values' \
  'Settings detail clean base: old baked row/card area cleared before detail draw' \
  'Battery status: non-Home screens show compact battery badge' \
  'Battery badge repair: Home-only battery badge with percent text' \
  'Battery settings: Device detail shows percent, USB/charging state, and voltage' \
  'Battery diagnostics: raw ADC, ADC mV, calculated battery mV, estimated percent' \
  'Battery calibration: /BATTERY.TXT adc_multiplier empty_mv full_mv' \
  'Battery calibration: Settings Device shows CAL status' \
  'Battery isolation: sample before SD calibration, then sample after calibration' \
  'Battery isolation: raw=0 ignored once, last valid ADC sample retained' \
  'Compile repair: battery raw/mV label helpers restored' \
  'Battery first-sample: ADC sampled before weather-cache and BATTERY.TXT SD reads' \
  'Battery first-sample: raw=0 never stored as valid battery sample' \
  'Battery ADC path: vendor-aligned ADC1 GPIO8 DB_11 multi-sample probe' \
  'Battery ADC batch: min/max/avg with ADC read error markers' \
  'Battery probe matrix: GPIO1/GPIO3/GPIO7/GPIO8/GPIO9 ADC1 candidates' \
  'Battery probe matrix: probe values are diagnostics only, not UI source' \
  'Battery probe enable: no confirmed vendor enable GPIO, matrix uses enable=NONE' \
  'Battery init order: GPIO8 ADC sampled immediately after Peripherals::take' \
  'Battery init phases: pre-i2c pre-wifi pre-exio post-exio post-display' \
  'Battery diagnostics repo alignment: BAT_Init before I2C/Wi-Fi/EXIO/SD/display' \
  'Battery C-shim parity: native ESP-IDF adc_oneshot ADC1_CH7 GPIO8 probe' \
  'Battery C-shim parity: Rust and C raw/mV logged side-by-side, UI source unchanged' \
  'Battery C-first ownership: native adc_oneshot probe runs before Rust AdcDriver' \
  'Battery C-first ownership: C-first and Rust-after results are compared, UI source unchanged' \
  'Battery enable path: rust-diagnostics inspected, no confirmed BAT enable pin found' \
  'Battery enable probe: C-first NONE baseline plus EXIO before/after init markers' \
  'Battery enable probe: enable values are diagnostics only, UI source unchanged' \
  'Board profile: WAVESHARE_ESP32_S3_TOUCH_LCD_1_85C_V1 schematic locked' \
  'Battery schematic: BAT_ADC(GPIO8) confirmed on V1 schematic' \
  'Battery divider: schematic R ladder implies multiplier about 3.0' \
  'Battery UI: Device shows BAT ADC GPIO8 CONFIRMED / SIGNAL MISSING' \
  'Battery C-shim parity: post-boot polling disabled to avoid ADC1-in-use spam' \
  'Battery vendor ADC: GPIO8 ADC1_CH7 uses ESP-IDF ADC_ATTEN_DB_12' \
  'Battery vendor ADC source: C-SHIM GPIO8 VENDOR' \
  'Battery vendor ADC: Measurement_offset=0.994500 default multiplier active' \
  'Battery vendor ADC: valid C-shim calibrated mV promoted as UI source' \
  'Battery Rust ADC: retained as diagnostic-only comparison' \
  'Screen sleep: software Sleep Now, top-left long touch, or idle timeout turns backlight off' \
  'Screen wake: touch interrupt turns backlight on' \
  'Screen wake guard: first wake touch is swallowed, navigation blocked' \
  'Screen sleep policy: background RTC/Wi-Fi/weather/battery services continue' \
  'Power GPIO diagnostics: GPIO6 configured with ESP-IDF gpio_config input pull-up' \
  'Power GPIO diagnostics: raw level and active-low duration logged' \
  'Power GPIO diagnostics: report-only, not a sleep trigger' \
  'Software sleep: Settings > Display > Sleep Now active' \
  'Software sleep: optional top-left long-touch gesture active' \
  'Compile repair: Settings Display Sleep Now match arm comma restored' \
  'Monitor log cleanup: NORMAL default, DEBUG via compile constant or /LOG.TXT' \
  'compile-repair: debug_println macro defined before TouchTracker use' \
  'wifi-scan: NORMAL profile skips scans; DEBUG scans only after station is connected' \
  'touch-normalization: NORMAL profile emits compact touch finish lines; DEBUG keeps full geometry' \
  'asset-loader: SD-backed UI assets path=/ASSETS names=HOME.RGB,WEATHER.RGB,MUSIC.RGB,AI.RGB,SETTINGS.RGB' \
  'compile-repair: UiAssetCache allocated in run_app before first render' \
  'video-foundation: path=/VIDEO formats=MJPG,MJPEG,MJP first-frame-boundary-scan=ENABLED playback=DEFERRED audio=DEFERRED' \
  'video-decode: first-frame JPEG decode enabled on Settings > Storage preview; playback=DEFERRED audio=DEFERRED' \
  'compile-repair: esp_jpeg output format enum uses JPEG_IMAGE_FORMAT_RGB565' \
  'video-preview: dedicated visible zone enabled; opaque Storage card skipped when decoded; RGB565 swap via /VIDEO/SWAP.TXT' \
  'video-playback: dedicated Video page worker playback fps={} audio=DEFERRED stop=TOUCH_OR_SWIPE' \
  'compile-repair: VIDEO_PLAYBACK_FRAME_INDEX static allocated for MJPEG frame loop' \
  'video-playback-visibility: requested_frame and decoded_frame logged separately; overlay shows tick/requested/decoded; frame_skip={} audio=DEFERRED' \
  'video-playback-responsiveness: touch-priority=ENABLED touch-stop=IMMEDIATE fps={} state=STOPPED,PLAYING,PAUSED,TOUCH_STOP config=FPS_TXT_DEFERRED,SKIP_TXT_DEFERRED' \
  'video-worker: dedicated Video page enabled; worker task decodes into PSRAM; UI displays latest frame; settings-storage=STATUS_PREVIEW_ONLY audio=DEFERRED' \
  'video-worker-sd-owner: worker SD mount/unmount disabled; single SD owner mutex foundation enabled; decode deferred until persistent SD mount; ui_status=WORKER_SD_OWNER' \
  'video-worker-quiet: repeated worker status logs throttled log_every={} stable_ui_status=WORKER_SD_OWNER no_crash_expected=true' \
  'sd-persistent-foundation: single_mount_session=ENABLED owner_mutex=REAL legacy_mount_calls=COMPAT_REUSE worker_reads=ALREADY_MOUNTED_PATH settings_storage_preview=PRESERVED' \
  'video-worker-request-latch: next-frame requests latch while BUSY pending=ONE requested_frame_and_decoded_frame_logged=YES fps=2 sd_reinit=NO' \
  'mjpeg-indexed-scanner: requested_frame_maps_to_actual_frame=YES eof_loop_to_zero=YES offset_size_logged=YES fps=2 sd_reinit=NO' \
  'mjpeg-stream-worker: sequential=ENABLED open_once=YES rolling_buffer=64KB scan_from_start=NO publish_latest_rgb565=YES fps=2 audio=DEFERRED' \
  'video-stream-runtime-tuning: fps_txt=/VIDEO/FPS.TXT default_fps=5 skip_txt=/VIDEO/SKIP.TXT default_skip=6 source_fps=30 decode_selected_only=YES no_audio=YES' \
  'video-stream-polish: normal_log_every=30 debug_per_frame=YES eof_wrap_actual_frame_0=YES timing=read_ms,skip_ms,decode_ms,publish_ms fps_tests=6x5,8x4' \
  'compile-repair: Video page added to ALL_PAGES length=6 and select-action match' \
  'partition-relief: before_r7_app_bytes={} partition_bytes={} embedded_rgb565_removed_bytes={} estimated_free_gain_kib={}' \
  'Button discovery matrix: GPIO0 BOOT, GPIO6 POWER, GPIO7, GPIO9 monitored' \
  'Button discovery matrix: report-only, no candidate bound to sleep yet' \
  'Button discovery matrix: GPIO0 BOOT never used for sleep while USB flashing is attached' \
  'Button discovery noise gate: boot level is idle baseline for every candidate' \
  'Button discovery noise gate: boot-active candidates marked noisy and not hold-spammed' \
  'EXIO input matrix: TCA9554 input register is polled read-only, no output toggles' \
  'EXIO safe discovery: preserve EXIO0..2 outputs and configure EXIO3..7 inputs' \
  'EXIO safe discovery: input mask 0xF8, protected output mask 0x07' \
  'EXIO safe discovery: report-only, no sleep binding until physical transition confirmed' \
  'Battery power: USB/UNKNOWN until charger GPIO is validated' \
  'Battery refresh: visible page redraws on battery change' \
  'Storage detail: SD free/total space shown when available'; do
  require_text "$main" "$pattern" "v0.1.22-r2 marker"
done

for pattern in \
  'struct SettingsDetailRow' \
  'draw_settings_detail_template' \
  'draw_settings_detail_clean_base' \
  'draw_settings_detail_row' \
  'settings_clip' \
  'draw_settings_network_detail' \
  'draw_settings_weather_detail' \
  'draw_settings_time_detail' \
  'draw_settings_storage_detail' \
  'draw_settings_device_detail' \
  'draw_settings_diagnostics_detail' \
  'fn draw_settings_time_icon' \
  'fn draw_settings_storage_icon' \
  'fn draw_settings_diag_icon' \
  'row x=64 width=232 height=30' \
  'SD FREE / TOTAL'; do
  require_text "$main" "$pattern" "shared settings detail template"
done

for pattern in \
  'pub sd_free_mb: Option<u32>' \
  'sd_total_text' \
  'sd_free_text' \
  'sd_free_total_text'; do
  require_text "$model" "$pattern" "SD free total model"
done

for pattern in \
  'st77916_probe_sd_space_mb' \
  'out_total_mb' \
  'out_free_mb'; do
  require_text "$ffi" "$pattern" "SD space ffi"
  require_text "$shim_h" "$pattern" "SD space header"
  require_text "$shim_c" "$pattern" "SD space c"
done

require_text "$shim_c" 'f_getfree' "SD FatFs free-space probe"
require_text "$shim_c" '#include "ff.h"' "SD FatFs header"
reject_text "$shim_c" 'sys/statvfs.h' "unsupported statvfs header"
require_text "$main" 'ffi::st77916_probe_sd_space_mb' "refresh SD free/total"
require_text "$actions" 'screen: HomeRefresh weather status={} location={}' "Home refresh accepted baseline"
reject_text "$main" 'draw_global_battery_status(model, frame)' "non-Home battery overlay removed"

for pattern in \
  'weather-fetch: hourly location={} slots={}' \
  'time-rtc: persisted source=NTP' \
  'wifi-connect: connected ssid={}' \
  'touch-class: settings detail header tap accepted back'; do
  require_text "$main" "$pattern" "accepted baseline"
done

reject_text "$main" 'draw_text_centered_at(frame, 180, 292, &model.weather.hourly_summary()' "long weather detail summary"
reject_text "$main" 'draw_settings_key_value(frame, 56' "old unaligned settings rows"

echo "Settings detail visual alignment validation: OK"

require_text "$model" 'battery_power_text' "battery power helper"
require_text "$model" 'battery_voltage_text' "battery voltage detail helper"
require_text "$model" 'format!("{}%", pct)' "battery percent home text"

require_text "$main" 'fn battery_sample_from_raw' "battery ADC conversion helper"
require_text "$main" 'fn note_battery_adc_sample' "battery ADC logging helper"
require_text "$main" 'battery-adc: source={}' "battery ADC diagnostic log"
require_text "$main" 'battery-cal: applied source={}' "battery calibration applied log"
require_text "$main" 'battery-cal: not loaded reason={}' "battery calibration default log"
require_text "$main" 'parse_battery_calibration_text' "battery config parser"
require_text "$main" 'BATTERY_TXT_FALLBACK_PATHS' "battery config fallback paths"
require_text "$main" 'model.battery_cal_text()' "Settings Device CAL footer"
require_text "$model" 'pub battery_adc_raw: Option<u16>' "battery raw field"
require_text "$model" 'pub battery_adc_mv: Option<u16>' "battery ADC mV field"
require_text "$model" 'battery_adc_text' "battery ADC text helper"
require_text "$model" 'battery_percent_detail_text' "battery percent estimate helper"
require_text "$model" '"USB/UNKNOWN"' "USB unknown power status"
reject_text "$model" '"USB CHG"' "no charger status claim"

require_text "$ffi" 'st77916_read_sd_battery_txt' "battery SD read ffi"
require_text "$shim_h" 'st77916_read_sd_battery_txt' "battery SD read header"
require_text "$shim_c" 'st77916_read_sd_battery_txt' "battery SD read shim"
require_text "$model" 'battery_adc_multiplier' "battery calibration multiplier model"
require_text "$model" 'battery_empty_mv' "battery empty threshold model"
require_text "$model" 'battery_full_mv' "battery full threshold model"
require_text "$model" 'battery_cal_text' "battery CAL status text"

require_text "$main" 'boot-precal' "battery precal sample log source"
require_text "$main" 'boot-cal' "battery calibrated sample log source"
require_text "$main" 'action=KEEP_LAST_VALID' "battery zero rejection keep-last marker"
require_text "$main" 'ADC_ZERO_REPEATED' "battery repeated zero marker"
require_text "$model" 'note_battery_zero_sample' "battery zero sample counter"
require_text "$model" 'battery_has_valid_sample' "battery valid sample guard"

require_text "$model" 'battery_adc_raw_label' "battery raw ADC label helper"
require_text "$model" 'battery_adc_mv_label' "battery ADC mV label helper"

require_text "$main" 'ADC_ZERO_NO_VALID_SAMPLE' "battery zero no-valid marker"
require_text "$main" 'action=KEEP_UNAVAILABLE' "battery keep unavailable marker"
reject_text "$main" 'action=ACCEPT_ZERO_OR_NO_VALID_SAMPLE' "raw zero must not be stored as valid"

require_text "$main" 'battery-adc-path: reference=rust-full-port unit=ADC1 gpio=GPIO{} attenuation=DB_11 samples={}' "battery ADC path boot log"
require_text "$main" 'battery-adc-batch: source={} unit=ADC1 gpio=GPIO{} attenuation=DB_11 samples={}' "battery ADC batch log"
require_text "$main" 'marker=ADC_READ_ERROR' "battery ADC read error marker"
require_text "$main" 'marker=ADC_BATCH_NO_VALID' "battery ADC no-valid batch marker"
require_text "$main" 'BATTERY_ADC_SAMPLE_COUNT' "battery ADC sample count"
require_text "$main" 'min={} max={} avg={}' "battery ADC min max avg log"

require_text "$main" 'battery-adc-probe-matrix: start candidates=GPIO1,GPIO3,GPIO7,GPIO8,GPIO9 enable=NONE ui_source=NO' "battery probe matrix start"
require_text "$main" 'battery-adc-probe-enable: candidate=NONE_CONFIRMED action=SKIP_WITH_ENABLE_TEST' "battery probe no confirmed enable marker"
require_text "$main" 'probe-gpio1-adc1-ch0' "battery probe gpio1"
require_text "$main" 'probe-gpio3-adc1-ch2' "battery probe gpio3"
require_text "$main" 'probe-gpio7-adc1-ch6' "battery probe gpio7"
require_text "$main" 'current-gpio8-adc1-ch7' "battery probe current gpio8"
require_text "$main" 'probe-gpio9-adc1-ch8' "battery probe gpio9"
require_text "$main" 'ui_source=NO' "battery probe diagnostic-only marker"
require_text "$main" 'marker=ADC_PROBE_NO_VALID' "battery probe no-valid marker"

require_text "$main" 'battery-init-phase: pre-i2c' "battery pre-i2c phase"
require_text "$main" 'battery-init-phase: pre-wifi' "battery pre-wifi phase"
require_text "$main" 'battery-init-phase: pre-exio' "battery pre-exio phase"
require_text "$main" 'battery-init-phase: post-exio' "battery post-exio phase"
require_text "$main" 'battery-init-phase: post-display' "battery post-display phase"
require_text "$main" '"pre-i2c"' "battery pre-i2c sample source"
require_text "$main" '"pre-wifi"' "battery pre-wifi sample source"
require_text "$main" '"pre-exio"' "battery pre-exio sample source"
require_text "$main" '"post-exio"' "battery post-exio sample source"
require_text "$main" '"post-display"' "battery post-display sample source"
require_text "$main" 'probe_battery_adc_matrix!(adc, bat_pin, pins)' "battery probe matrix preserved"

require_text "$main" 'battery-adc-parity: source={}' "battery ADC Rust/C parity log"
require_text "$main" 'c_unit=ADC1 c_channel=7 c_gpio=GPIO8' "battery C-shim channel log"
require_text "$main" 'c_mv_avg={}' "battery C-shim millivolt log"
require_text "$main" 'c_calibrated={}' "battery C-shim calibrated flag"
require_text "$main" 'ui_source=NO' "battery parity diagnostic-only marker"
require_text "$ffi" 'St77916AdcProbeResult' "battery ADC FFI result struct"
require_text "$ffi" 'st77916_adc1_gpio8_oneshot_probe' "battery ADC FFI function"
require_text "$shim_c" 'adc_oneshot_new_unit' "C-shim native adc oneshot unit"
require_text "$shim_c" 'ADC_CHANNEL_7' "C-shim ADC1 channel 7"
require_text "$shim_c" 'adc_cali_raw_to_voltage' "C-shim calibrated millivolt probe"
require_text "$firmware_dir/components/st77916_shim/CMakeLists.txt" 'esp_adc' "C-shim esp_adc dependency"

require_text "$main" 'battery-adc-cfirst: phase=before-rust-adc-driver' "battery C-first log"
require_text "$main" 'battery-adc-ownership-compare:' "battery ownership compare log"
require_text "$main" 'c_first_mv_avg={}' "battery C-first millivolt compare"
require_text "$main" 'rust_after_source=pre-i2c' "battery Rust-after compare source"
require_text "$main" 'ui_source=NO' "battery ownership diagnostic-only marker"

require_text "$main" 'battery-enable-probe: phase={}' "battery enable probe log helper"
require_text "$main" 'enable_source={}' "battery enable source field"
require_text "$main" 'enable_state={}' "battery enable state field"
require_text "$main" 'mv_avg={}' "battery enable millivolt field"
require_text "$main" 'NO_CONFIRMED_ENABLE' "battery no confirmed enable marker"
require_text "$main" 'SKIP_NO_VENDOR_BAT_ENABLE_IN_RUST_DIAGNOSTICS' "battery diagnostics enable inspection marker"
require_text "$main" 'SKIP_BEFORE_EXIO_INIT' "battery pre-exio enable skip marker"
require_text "$main" 'NO_TOGGLE_EXIO_BITS_0_2_ASSIGNED_TOUCH_LCD_SD' "battery post-exio safe no-toggle marker"
require_text "$main" 'ui_source=NO' "battery enable diagnostic-only marker"

require_text "$firmware_dir/src/board.rs" 'BOARD_PROFILE_V1_SCHEMATIC' "V1 board profile constant"
require_text "$firmware_dir/src/board.rs" 'BATTERY_ADC_SCHEMATIC_LABEL' "V1 BAT_ADC label constant"
require_text "$firmware_dir/src/app/model.rs" 'battery_adc_status_text' "battery ADC status model"
require_text "$firmware_dir/src/app/model.rs" 'battery_adc_source_text' "battery ADC source model"
require_text "$main" 'BAT_ADC(GPIO8)' "Settings Device BAT_ADC source"
require_text "$main" 'SIGNAL MISSING' "Settings Device signal missing status"
require_text "$main" 'SKIP_AFTER_RUST_ADC_OWNED' "post-boot C-shim parity skip"
require_text "$main" 'C_FIRST_ALREADY_LOGGED' "C-first ownership retained marker"
require_text "$repo_root/docs/V1_SCHEMATIC_BATTERY_ADC_CONFIRMATION_v0.1.23-r11.md" 'BAT_ADC ≈ battery voltage / 3' "physical probe checklist"
reject_text "$repo_root/README.md" 'battery ADC is unsupported' "unsupported battery ADC wording"
reject_text "$repo_root/architecture.md" 'battery ADC is unsupported' "unsupported battery ADC architecture wording"

require_text "$firmware_dir/src/board.rs" 'BATTERY_SOURCE_C_SHIM_VENDOR' "vendor C-shim source label"
require_text "$firmware_dir/src/board.rs" 'BATTERY_MEASUREMENT_OFFSET: f32 = 0.9945' "vendor measurement offset"
require_text "$firmware_dir/components/st77916_shim/st77916_shim.c" 'ADC_ATTEN_DB_12' "vendor DB12 attenuation"
require_text "$firmware_dir/src/app/model.rs" 'note_battery_sample_with_source' "source-aware battery model update"
require_text "$main" 'note_vendor_c_shim_battery_sample' "vendor C-shim promotion helper"
require_text "$main" 'action=PROMOTE_TO_UI' "vendor C-shim promotion log"
require_text "$main" 'source=c-shim-gpio8-vendor' "vendor C-shim source log"
require_text "$main" 'note_rust_adc_diagnostic_sample' "Rust diagnostic-only battery function"
require_text "$main" 'VENDOR_C_SHIM_IS_PRODUCTION' "Rust diagnostic no-overwrite marker"
require_text "$main" 'SOURCE' "Settings Device source row"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "vendor source boot marker"
reject_text "$main" 'note_battery_adc_sample(&mut $model, avg_raw' "Rust ADC must not overwrite C-shim source"

require_text "$main" 'SCREEN_SLEEP_IDLE_MS: u64 = 120_000' "screen sleep idle timeout"
require_text "$main" 'SCREEN_WAKE_GUARD_MS: u64 = 700' "screen wake guard timeout"
require_text "$main" 'let mut screen_sleeping = false' "screen sleep runtime state"
require_text "$main" 'let mut wake_guard_until: Option<Instant>' "screen wake guard state"
require_text "$main" 'screen-sleep: source=idle idle_ms={}' "idle sleep log"
require_text "$main" 'screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH' "touch wake guarded log"
require_text "$main" 'screen-wake-guard: touch ignored source=int action=NO_NAVIGATION' "wake touch swallowed log"
require_text "$main" '!touch_tracker.active && !screen_sleeping' "render paused while sleeping"
require_text "$main" 'backlight.set_low()?' "backlight off command"
require_text "$main" 'backlight.set_high()?' "backlight on command"
require_text "$repo_root/docs/SCREEN_SLEEP_WAKE_GUARD_v0.1.24.md" 'POWER long' "screen sleep doc"

require_text "$firmware_dir/src/board.rs" 'POWER_BUTTON_GPIO: u8 = 6' "power GPIO board constant"
require_text "$firmware_dir/src/ffi.rs" 'st77916_gpio_input_pullup' "power GPIO input FFI"
require_text "$firmware_dir/components/st77916_shim/include/st77916_shim.h" 'st77916_gpio_input_pullup' "power GPIO shim header"
require_text "$firmware_dir/components/st77916_shim/st77916_shim.c" 'gpio_config_t io_conf' "power GPIO C gpio_config"
require_text "$firmware_dir/components/st77916_shim/st77916_shim.c" 'GPIO_PULLUP_ENABLE' "power GPIO pull-up enabled"
require_text "$main" 'POWER_GPIO_DIAG_MS: u64 = 250' "power GPIO diagnostic interval"
require_text "$main" 'power-gpio-config: gpio=GPIO{} via=c-shim gpio_config input=ENABLE pullup=ENABLE' "power GPIO config log"
require_text "$main" 'power-gpio-diag: gpio=GPIO{} level={} active_low_down={} duration_ms={} source=state-change' "power GPIO state change log"
require_text "$main" 'power-gpio-diag: gpio=GPIO{} level={} active_low_down=true duration_ms={} source=hold' "power GPIO hold log"
require_text "$main" 'power-gpio-event: gpio=GPIO{} event={:?} level={} active_low_down={} duration_ms={} confirmed=YES' "power GPIO event log"
require_text "$main" 'screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH' "touch wake preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "r12 battery source preserved"

require_text "$main" 'BUTTON_PIN_CANDIDATES: [ButtonPinCandidate; 4]' "button discovery candidate list"
require_text "$main" 'label: "BOOT"' "GPIO0 BOOT candidate"
require_text "$main" 'gpio: 0' "GPIO0 candidate"
require_text "$main" 'label: "POWER_ASSUMED"' "GPIO6 candidate"
require_text "$main" 'gpio: 6' "GPIO6 candidate"
require_text "$main" 'label: "GPIO7_FREE_CANDIDATE"' "GPIO7 candidate"
require_text "$main" 'label: "GPIO9_FREE_CANDIDATE"' "GPIO9 candidate"
require_text "$main" 'USB_FLASHING_GUARD' "BOOT no-sleep guard"
require_text "$main" 'button-discovery-matrix: start candidates={} mode=REPORT_ONLY sleep_binding=DISABLED' "button discovery start log"
require_text "$main" 'button-pin-config: label={} gpio=GPIO{} origin={} input_pullup_ok={}' "button pin config log"
require_text "$main" 'button-pin-event: label={} gpio=GPIO{} event=LongCandidate' "button long candidate log"
require_text "$main" 'action=REPORT_ONLY sleep_bind_allowed={}' "report-only action"
require_text "$main" 'screen-sleep: source=power-long action=SKIP_UNBOUND_CANDIDATE' "power long unbound sleep skip"
require_text "$main" 'screen-sleep: source=idle idle_ms={}' "idle sleep preserved"
require_text "$main" 'screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH' "touch wake preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "r12 battery source preserved"

require_text "$firmware_dir/src/drivers/tca9554.rs" 'pub const INPUT_PORT: u8 = 0x00' "TCA9554 input register const"
require_text "$firmware_dir/src/drivers/tca9554.rs" 'pub fn read_input_port' "TCA9554 read input port"
require_text "$firmware_dir/src/drivers/tca9554.rs" 'i2c.write_read(addr, &[Self::INPUT_PORT]' "TCA9554 input write_read"
require_text "$main" 'v0.1.24-r3 Button Discovery Noise Gate + EXIO Input Matrix' "r3 boot marker"
require_text "$main" 'Button discovery noise gate: boot level is idle baseline for every candidate' "noise gate marker"
require_text "$main" 'EXIO input matrix: TCA9554 input register is polled read-only, no output toggles' "EXIO marker"
require_text "$main" 'boot_level: i32' "button state baseline field"
require_text "$main" 'noisy_at_boot: bool' "button state noisy field"
require_text "$main" 'button_candidate_noisy_at_boot' "button noisy helper"
require_text "$main" 'classification=STUCK_LOW_OR_BOOT_ACTIVE action=HOLD_SPAM_SUPPRESSED' "stuck-low suppression log"
require_text "$main" 'state-change-away-from-baseline' "away-from-baseline state change"
require_text "$main" 'return-to-baseline' "return-to-baseline log"
require_text "$main" 'exio-input-matrix: init addr=0x{:02X}' "EXIO init log"
require_text "$main" 'exio-input-change: addr=0x{:02X}' "EXIO change log"
require_text "$main" 'mode=READ_ONLY no_output_toggle=YES' "EXIO read-only guard"
require_text "$main" 'screen-sleep: source=idle idle_ms={}' "idle sleep preserved"
require_text "$main" 'screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH' "touch wake preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "r12 battery source preserved"

require_text "$firmware_dir/src/board.rs" 'EXIO_SAFE_OUTPUT_MASK: u8 = 0x07' "EXIO protected output mask"
require_text "$firmware_dir/src/board.rs" 'EXIO_DISCOVERY_INPUT_MASK: u8 = 0xF8' "EXIO discovery input mask"
require_text "$firmware_dir/src/board.rs" 'EXIO_DISCOVERY_CONFIG: u8 = 0xF8' "EXIO discovery config"
require_text "$main" 'v0.1.24-r4 EXIO Safe Input-Only Discovery for Unused Bits' "r4 boot marker"
require_text "$main" 'EXIO safe discovery: preserve EXIO0..2 outputs and configure EXIO3..7 inputs' "safe EXIO marker"
require_text "$main" 'exio-safe-input-config: addr=0x{:02X} config=0x{:02X}' "EXIO safe config log"
require_text "$main" 'protected_outputs=EXIO0_TOUCH_RST,EXIO1_LCD_RST,EXIO2_SD_CS' "protected EXIO outputs log"
require_text "$main" 'discovery_inputs=EXIO3,EXIO4,EXIO5,EXIO6,EXIO7' "EXIO discovery inputs log"
require_text "$main" 'let _ = exio.set_output_port(&mut i2c, board::TCA9554_ADDR, 0xFF)' "EXIO outputs set high before config"
require_text "$main" 'exio.set_config(&mut i2c, board::TCA9554_ADDR, board::EXIO_DISCOVERY_CONFIG)' "EXIO safe config apply"
require_text "$main" 'raw_value & board::EXIO_DISCOVERY_INPUT_MASK' "EXIO masked input polling"
require_text "$main" 'input_mask=0x{:02X} protected_output_mask=0x{:02X} mode=READ_ONLY no_output_toggle=YES' "EXIO masked change log"
require_text "$main" 'screen-sleep: source=idle idle_ms={}' "idle sleep preserved"
require_text "$main" 'screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH' "touch wake preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "r12 battery source preserved"

require_text "$firmware_dir/src/app/settings.rs" 'software_sleep_requested: bool' "software sleep request state"
require_text "$firmware_dir/src/app/settings.rs" 'take_software_sleep_request' "software sleep request take method"
require_text "$firmware_dir/src/app/settings.rs" 'SettingsPanel::Display => "SLEEP NOW".to_string()' "Display current value Sleep Now"
require_text "$main" 'v0.1.24-r5 Software Sleep Control' "r5 boot marker"
require_text "$main" 'SOFTWARE_SLEEP_CORNER_HOLD_MS: u64 = 900' "corner sleep hold threshold"
require_text "$main" 'software_sleep_corner_ready' "corner sleep helper"
require_text "$main" 'software-sleep-control: settings_display_sleep_now=ENABLED' "software sleep runtime marker"
require_text "$main" 'screen-sleep: policy idle_ms={} wake_guard_ms={} sleep_sources=SETTINGS_DISPLAY_SLEEP_NOW,TOP_LEFT_LONG_TOUCH,IDLE wake_sources=TOUCH_INT power=REPORT_ONLY' "software sleep policy marker"
require_text "$main" 'screen-sleep: source=settings-display-sleep-now backlight=OFF render=PAUSED services=ACTIVE' "Settings Sleep Now log"
require_text "$main" 'screen-sleep: source=top-left-long-touch hold_ms={}' "corner long-touch sleep log"
require_text "$main" 'screen-sleep: source=software touch released wake=ARMED' "software sleep release guard"
require_text "$main" 'button: POWER long -> report-only no firmware sleep binding' "POWER long report-only"
require_text "$main" 'SettingsDetailRow { label: "SLEEP", value: "NOW"' "Display Sleep Now row"
require_text "$main" 'screen-sleep: source=idle idle_ms={}' "idle sleep preserved"
require_text "$main" 'screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH' "touch wake preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "r12 battery source preserved"

require_text "$firmware_dir/src/app/settings.rs" 'SettingsPanel::Display => "SLEEP NOW".to_string(),' "Display Sleep Now match arm comma"
require_text "$main" 'v0.1.24-r5-r1 Software Sleep Compile Repair' "r5-r1 boot marker"
require_text "$main" 'Compile repair: Settings Display Sleep Now match arm comma restored' "r5-r1 compile repair marker"
require_text "$main" '_providers: &LocalProviders' "unused providers warning repaired"
require_text "$main" '_wifi: &mut EspWifi' "unused wifi warning repaired"
require_text "$main" '_wifi_connect_deadline: &mut Option<Instant>' "unused wifi deadline warning repaired"

require_text "$main" 'LOG_PROFILE_COMPILE_DEBUG: bool = false' "NORMAL default compile log profile"
require_text "$main" 'RuntimeLogProfile' "runtime log profile enum"
require_text "$main" 'debug_println!' "debug print macro"
require_text "$main" 'st77916_configure_runtime_logs' "runtime ESP-IDF log configuration call"
require_text "$main" 'st77916_read_sd_log_txt' "runtime SD LOG.TXT read call"
require_text "$main" 'LOG=DEBUG' "LOG.TXT debug documentation marker"
require_text "$main" 'version: v0.1.31-r2 Stream Playback Polish + Log Quieting' "compact boot version summary"
require_text "$main" 'monitor: essential logs only; add LOG=DEBUG to /LOG.TXT for diagnostics' "compact monitor summary"
require_text "$main" 'runtime: log_profile={} source={} sleep=software+idle wake=touch-int battery_source=C-SHIM_GPIO8_VENDOR' "compact runtime summary"
require_text "$main" 'battery-summary: source={} voltage={} percent={} cal={}' "battery summary log"
require_text "$main" 'touch-track: sample id={} source={}' "touch sample suppressed in NORMAL"
require_text "$main" 'battery-adc-batch: source={}' "battery diagnostic suppressed in NORMAL"
require_text "$main" 'power-gpio-diag: gpio=GPIO{}' "GPIO discovery suppressed in NORMAL"
require_text "$main" 'debug_println!("render: coalesced repaint ok")' "render ok suppressed in NORMAL"
require_text "$ffi" 'st77916_configure_runtime_logs' "runtime log profile FFI"
require_text "$ffi" 'st77916_read_sd_log_txt' "LOG.TXT FFI"
require_text "$shim_h" 'st77916_configure_runtime_logs' "runtime log profile shim header"
require_text "$shim_h" 'st77916_read_sd_log_txt' "LOG.TXT shim header"
require_text "$shim_c" 'esp_log_level_set("gpio"' "ESP-IDF gpio log level control"
require_text "$shim_c" 'st77916_read_sd_log_txt' "LOG.TXT shim reader"
require_text "$main" 'screen-sleep: source=settings-display-sleep-now backlight=OFF render=PAUSED services=ACTIVE' "Settings sleep preserved"
require_text "$main" 'screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH' "touch wake preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "r12 battery source preserved"

require_text "$main" 'v0.1.24-r6-r1 Log Macro Scope Compile Repair' "r6-r1 boot marker"
require_text "$main" 'compile-repair: debug_println macro defined before TouchTracker use' "r6-r1 macro scope marker"
require_text "$main" 'macro_rules! debug_println' "debug print macro exists"
require_text "$main" 'impl TouchTracker' "TouchTracker exists"
require_text "$main" 'debug_println!("touch-track: begin id={}"' "TouchTracker debug macro use exists"

require_text "$main" 'v0.1.24-r7 Wi-Fi Scan Quieting + Touch Finish Normalization' "r7 boot marker"
require_text "$main" 'wifi-scan: NORMAL profile skips scans; DEBUG scans only after station is connected' "wifi scan quieting marker"
require_text "$main" 'touch-normalization: NORMAL profile emits compact touch finish lines; DEBUG keeps full geometry' "touch normalization marker"
require_text "$main" 'let ap_count = if connected && log_debug_enabled()' "scan only in DEBUG when connected"
require_text "$main" 'wifi-scan: skipped reason=normal-profile connected=YES' "normal scan skip log"
require_text "$main" 'wifi-status: connected={} ssid={} aps={}' "compact wifi status log"
require_text "$main" 'fn compact_touch_kind' "compact touch kind helper"
require_text "$main" 'touch: id={} kind={} page={:?} end=({}, {}) ms={} gesture=0x{:02X}' "compact touch finish log"
require_text "$main" 'touch-track: finish id={} reason={}' "full touch finish debug log retained"
require_text "$main" 'screen-sleep: source=settings-display-sleep-now backlight=OFF render=PAUSED services=ACTIVE' "Settings sleep preserved"
require_text "$main" 'screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH' "touch wake preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "r12 battery source preserved"

require_text "$main" 'v0.1.25 SD-Backed UI Asset Loader + App Partition Relief' "v0.1.25 boot marker"
require_text "$main" 'UI_ASSET_EMBEDDED_BYTES_REMOVED' "embedded RGB565 relief constant"
require_text "$main" 'APP_BINARY_BEFORE_R7_BYTES: usize = 2_616_176' "r7 before app size marker"
require_text "$main" 'struct UiAssetCache' "PSRAM UI asset cache"
require_text "$main" 'fn new() -> Result<Self>' "UI asset cache constructor"
require_text "$main" 'st77916_read_sd_asset_rgb565' "SD RGB565 asset loader FFI call"
require_text "$main" 'draw_page_fallback_base' "minimal fallback renderer"
require_text "$main" 'asset-cache: page={:?} source=SD path=/ASSETS/{} bytes={}' "SD asset load log"
require_text "$main" 'asset-cache: page={:?} source=FALLBACK asset={}' "fallback asset log"
require_text "$main" 'HOME.RGB' "SD home asset name"
require_text "$main" 'SETTINGS.RGB' "SD settings asset name"
require_text "$ffi" 'st77916_read_sd_asset_rgb565' "SD asset FFI declaration"
require_text "$shim_h" 'st77916_read_sd_asset_rgb565' "SD asset shim header"
require_text "$shim_c" 'st77916_read_sd_asset_rgb565' "SD asset shim implementation"
reject_text "$main" 'include_bytes!("../assets/rgb565/home_base.rgb565")' "home asset must not be embedded"
reject_text "$main" 'include_bytes!("../assets/rgb565/weather_base.rgb565")' "weather asset must not be embedded"
reject_text "$main" 'include_bytes!("../assets/rgb565/music_base.rgb565")' "music asset must not be embedded"
reject_text "$main" 'include_bytes!("../assets/rgb565/assistant_base.rgb565")' "assistant asset must not be embedded"
reject_text "$main" 'include_bytes!("../assets/rgb565/settings_base.rgb565")' "settings asset must not be embedded"
require_text "$main" 'screen-sleep: source=settings-display-sleep-now backlight=OFF render=PAUSED services=ACTIVE' "Settings sleep preserved"
require_text "$main" 'screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH' "touch wake preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "r12 battery source preserved"

require_text "$main" 'v0.1.25-r1 SD Asset Cache Scope Compile Repair' "v0.1.25-r1 boot marker"
require_text "$main" 'compile-repair: UiAssetCache allocated in run_app before first render' "asset cache scope repair marker"
require_text "$main" 'let mut asset_cache = UiAssetCache::new()?;' "asset cache allocation in run_app"
require_text "$main" 'render_if_dirty(&mut dirty, &model, frame.as_mut_slice(), &mut asset_cache, true, &mut last_render)?;' "initial render receives asset cache"
require_text "$main" '&mut asset_cache,' "event-loop render receives asset cache"
require_text "$main" 'struct UiAssetCache' "UI asset cache struct"
require_text "$main" 'screen-sleep: source=settings-display-sleep-now backlight=OFF render=PAUSED services=ACTIVE' "Settings sleep preserved"
require_text "$main" 'screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH' "touch wake preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "r12 battery source preserved"

require_text "$main" 'v0.1.26 MJPEG Video Foundation' "v0.1.26 boot marker"
require_text "$main" 'video-foundation: path=/VIDEO formats=MJPG,MJPEG,MJP first-frame-boundary-scan=ENABLED playback=DEFERRED audio=DEFERRED' "video foundation boot marker"
require_text "$main" 'fn refresh_video_foundation' "video foundation refresh function"
require_text "$main" 'st77916_probe_sd_mjpeg_library' "MJPEG library probe FFI call"
require_text "$main" 'video-foundation: status={} files={} first={} file_bytes={} frame_offset={} frame_bytes={} playback=DEFERRED audio=DEFERRED' "video foundation status log"
require_text "$model" 'video_count' "video count state"
require_text "$model" 'video_first_frame_size' "video frame size state"
require_text "$model" 'update_video_foundation' "video state update helper"
require_text "$model" 'video_first_frame_text' "video frame display helper"
require_text "$main" 'SettingsDetailRow { label: "VIDEO"' "Storage video row"
require_text "$main" 'SettingsDetailRow { label: "FRAME"' "Storage frame row"
require_text "$ffi" 'St77916MjpegProbeResult' "MJPEG probe Rust struct"
require_text "$ffi" 'st77916_probe_sd_mjpeg_library' "MJPEG probe FFI declaration"
require_text "$shim_h" 'st77916_mjpeg_probe_result_t' "MJPEG probe C struct"
require_text "$shim_h" 'st77916_probe_sd_mjpeg_library' "MJPEG probe C header"
require_text "$shim_c" 'st77916_has_mjpeg_extension' "MJPEG extension filter"
require_text "$shim_c" 'st77916_find_first_jpeg_frame' "JPEG frame boundary scanner"
require_text "$shim_c" 'opendir("/sdcard/VIDEO")' "VIDEO directory scan"
require_text "$shim_c" '0xFF && current == 0xD8' "JPEG SOI detection"
require_text "$shim_c" '0xFF && current == 0xD9' "JPEG EOI detection"
require_text "$main" 'screen-sleep: source=settings-display-sleep-now backlight=OFF render=PAUSED services=ACTIVE' "Settings sleep preserved"
require_text "$main" 'screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH' "touch wake preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "r12 battery source preserved"

require_text "$main" 'v0.1.27 MJPEG First Frame Decode + Display' "v0.1.27 boot marker"
require_text "$main" 'video-decode: first-frame JPEG decode enabled on Settings > Storage preview; playback=DEFERRED audio=DEFERRED' "video decode boot marker"
require_text "$main" 'fn render_video_first_frame_preview' "video preview render hook"
require_text "$main" 'st77916_decode_first_mjpeg_frame_rgb565' "video decode FFI call"
require_text "$main" 'video-decode: status={} first={} jpeg={}x{} output={}x{} scale={} frame_bytes={} preview=({},{} {}x{}) swap={} display={}' "video decode status log"
require_text "$main" 'SettingsDetailRow { label: "DECODE"' "Storage decode row"
require_text "$ffi" 'St77916MjpegDecodeResult' "MJPEG decode Rust struct"
require_text "$ffi" 'st77916_decode_first_mjpeg_frame_rgb565' "MJPEG decode FFI declaration"
require_text "$shim_h" 'st77916_mjpeg_decode_result_t' "MJPEG decode C struct"
require_text "$shim_h" 'st77916_decode_first_mjpeg_frame_rgb565' "MJPEG decode C header"
require_text "$shim_c" 'jpeg_decoder.h' "esp_jpeg decoder include"
require_text "$shim_c" 'esp_jpeg_decode' "esp_jpeg decode call"
require_text "$shim_c" 'JPEG_IMAGE_FORMAT_RGB565' "RGB565 decode output format"
require_text "$shim_c" 'st77916_parse_jpeg_dimensions' "JPEG dimension parser"
require_text "$shim_c" 'st77916_decode_first_mjpeg_frame_rgb565' "MJPEG first-frame decode implementation"
require_text "$shim_c" 'MALLOC_CAP_SPIRAM' "PSRAM decode buffers"
require_text "$firmware_dir/components/st77916_shim/CMakeLists.txt" 'esp_jpeg' "esp_jpeg CMake dependency"
require_file "$firmware_dir/components/st77916_shim/idf_component.yml" "esp_jpeg component manifest"
require_text "$firmware_dir/components/st77916_shim/idf_component.yml" 'espressif/esp_jpeg' "esp_jpeg component registry dependency"
require_text "$main" 'screen-sleep: source=settings-display-sleep-now backlight=OFF render=PAUSED services=ACTIVE' "Settings sleep preserved"
require_text "$main" 'screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH' "touch wake preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "r12 battery source preserved"

require_text "$main" 'v0.1.27-r1 MJPEG JPEG Format Enum Compile Repair' "v0.1.27-r1 boot marker"
require_text "$main" 'compile-repair: esp_jpeg output format enum uses JPEG_IMAGE_FORMAT_RGB565' "JPEG enum compile repair marker"
require_text "$shim_c" 'JPEG_IMAGE_FORMAT_RGB565' "fixed esp_jpeg RGB565 enum"
reject_text "$shim_c" 'JPEG_IMAGE_OUT_FORMAT_RGB565' "broken esp_jpeg RGB565 enum must be removed"
require_text "$shim_c" 'esp_jpeg_decode' "esp_jpeg decode call preserved"
require_text "$shim_c" 'st77916_decode_first_mjpeg_frame_rgb565' "MJPEG decode implementation preserved"
require_text "$main" 'video-decode: status={} first={} jpeg={}x{} output={}x{} scale={} frame_bytes={} preview=({},{} {}x{}) swap={} display={}' "decode status log preserved"
require_text "$main" 'screen-sleep: source=settings-display-sleep-now backlight=OFF render=PAUSED services=ACTIVE' "Settings sleep preserved"
require_text "$main" 'screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH' "touch wake preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "r12 battery source preserved"

require_text "$main" 'v0.1.27-r2 MJPEG Preview Visibility Repair' "v0.1.27-r2 boot marker"
require_text "$main" 'video-preview: dedicated visible zone enabled; opaque Storage card skipped when decoded; RGB565 swap via /VIDEO/SWAP.TXT' "preview visibility marker"
require_text "$main" 'struct VideoPreviewOutcome' "preview outcome metadata"
require_text "$main" 'draw_storage_video_preview_overlay' "dedicated storage preview overlay"
require_text "$main" 'draw_storage_video_preview_row' "preview metadata rows"
require_text "$main" 'FIRST FRAME' "preview label"
require_text "$main" 'VISIBLE_PREVIEW_ZONE' "visible preview display log"
require_text "$main" 'preview.x - 5' "preview border uses returned coordinates"
require_text "$main" 'draw_settings_detail_template(' "fallback storage detail template preserved"
require_text "$shim_h" 'preview_x' "preview x returned by shim"
require_text "$shim_h" 'preview_y' "preview y returned by shim"
require_text "$shim_h" 'color_swap' "RGB565 swap returned by shim"
require_text "$ffi" 'preview_x' "preview x in Rust FFI"
require_text "$ffi" 'color_swap' "RGB565 swap in Rust FFI"
require_text "$shim_c" 'VIDEO_PREVIEW_MAX_WIDTH' "preview max width"
require_text "$shim_c" 'VIDEO_PREVIEW_MAX_HEIGHT' "preview max height"
require_text "$shim_c" 'VIDEO_PREVIEW_TOP_Y' "preview top y"
require_text "$shim_c" 'st77916_video_swap_rgb565_enabled' "RGB565 byte swap option helper"
require_text "$shim_c" '/sdcard/VIDEO/SWAP.TXT' "RGB565 byte swap SD config"
require_text "$shim_c" '.flags = { .swap_color_bytes = swap_color_bytes ? 1 : 0 }' "esp_jpeg byte-swap option"
require_text "$shim_c" 'out_result->preview_x = (uint16_t) dst_x;' "preview x populated"
require_text "$shim_c" 'out_result->preview_y = (uint16_t) dst_y;' "preview y populated"
require_text "$shim_c" 'uint32_t dst_y = VIDEO_PREVIEW_TOP_Y;' "preview uses fixed visible top zone"
require_text "$main" 'screen-sleep: source=settings-display-sleep-now backlight=OFF render=PAUSED services=ACTIVE' "Settings sleep preserved"
require_text "$main" 'screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH' "touch wake preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "r12 battery source preserved"

require_text "$main" 'v0.1.28 MJPEG Playback Loop, No Audio' "v0.1.28 boot marker"
require_text "$main" 'VIDEO_PLAYBACK_FRAME_MS: u64 = 200' "runtime-tuned playback frame timer"
require_text "$main" 'VIDEO_PLAYBACK_FRAME_INDEX' "playback atomic frame index"
require_text "$main" 'fn video_playback_visible' "video playback visibility guard"
require_text "$main" 'fn render_video_playback_frame' "video playback renderer"
require_text "$main" 'fn render_video_first_frame_preview' "first-frame compatibility wrapper"
require_text "$main" 'st77916_decode_mjpeg_frame_rgb565' "frame-index MJPEG decode FFI call"
require_text "$main" 'video-playback: status={} first={} frame={} jpeg={}x{} output={}x{} scale={} frame_bytes={} preview=({},{} {}x{}) swap={} fps={} audio=DEFERRED display={}' "video playback status log"
require_text "$main" 'last_video_playback_frame' "video playback loop ticker"
require_text "$ffi" 'decoded_frame_index' "decode result frame index"
require_text "$ffi" 'st77916_decode_mjpeg_frame_rgb565' "frame-index decode FFI declaration"
require_text "$shim_h" 'decoded_frame_index' "C decode result frame index"
require_text "$shim_h" 'st77916_decode_mjpeg_frame_rgb565' "frame-index decode C header"
require_text "$shim_c" 'st77916_find_jpeg_frame_at_index' "MJPEG indexed frame scanner"
require_text "$shim_c" 'st77916_decode_mjpeg_frame_rgb565' "MJPEG indexed frame decoder"
require_text "$shim_c" 'out_result->decoded_frame_index = actual_index' "decoded frame actual index set"
require_text "$shim_c" 'out_result->status = -12' "loop/end-of-file status"
require_text "$main" 'NO AUDIO - SWIPE AWAY TO STOP' "no-audio playback UI label"
require_text "$main" 'screen-sleep: source=settings-display-sleep-now backlight=OFF render=PAUSED services=ACTIVE' "Settings sleep preserved"
require_text "$main" 'screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH' "touch wake preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "r12 battery source preserved"

require_text "$main" 'v0.1.28-r1 Playback Frame Index Compile Repair' "v0.1.28-r1 boot marker"
require_text "$main" 'compile-repair: VIDEO_PLAYBACK_FRAME_INDEX static allocated for MJPEG frame loop' "frame index compile repair marker"
require_text "$main" 'static VIDEO_PLAYBACK_FRAME_INDEX: AtomicU32 = AtomicU32::new(0);' "frame index static definition"
require_text "$main" 'VIDEO_PLAYBACK_FRAME_INDEX.fetch_add(1, Ordering::Relaxed)' "frame index increment preserved"
require_text "$main" 'VIDEO_PLAYBACK_FRAME_INDEX.store(1, Ordering::Relaxed)' "frame index reset preserved"
require_text "$main" 'video-playback: status={} first={} frame={} jpeg={}x{} output={}x{} scale={} frame_bytes={} preview=({},{} {}x{}) swap={} fps={} audio=DEFERRED display={}' "playback status log preserved"
require_text "$main" 'NO AUDIO - SWIPE AWAY TO STOP' "no-audio playback UI label preserved"
require_text "$main" 'screen-sleep: source=settings-display-sleep-now backlight=OFF render=PAUSED services=ACTIVE' "Settings sleep preserved"
require_text "$main" 'screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH' "touch wake preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "r12 battery source preserved"

require_text "$main" 'v0.1.28-r2 MJPEG Playback Frame Advance Visibility Repair' "v0.1.28-r2 boot marker"
require_text "$main" 'video-playback-visibility: requested_frame and decoded_frame logged separately; overlay shows tick/requested/decoded; frame_skip={} audio=DEFERRED' "playback visibility marker"
require_text "$main" 'VIDEO_PLAYBACK_FRAME_STEP: u32 = 6' "frame skip constant"
require_text "$main" 'requested_frame={} decoded_frame={} tick={} skip={}' "requested/decoded frame log fields"
require_text "$main" 'active_requested_index = requested_index' "requested frame variable"
require_text "$main" 'playback_tick: active_tick' "overlay tick field"
require_text "$main" 'requested_frame: active_requested_index' "overlay requested field"
require_text "$main" 'let video_frame = format!("T{} R{}", preview.playback_tick, preview.requested_frame);' "overlay tick/requested display"
require_text "$main" 'let decoded_label = format!("{} D{} SKIP{}", video_playback_state_label(), preview.frame_index, VIDEO_PLAYBACK_FRAME_STEP);' "overlay decoded/skip display"
require_text "$shim_c" 'out_result->decoded_frame_index = actual_index;' "C decoded frame index reporting repair"
require_text "$main" 'NO AUDIO - SWIPE AWAY TO STOP' "no-audio stop label preserved"
require_text "$main" 'screen-sleep: source=settings-display-sleep-now backlight=OFF render=PAUSED services=ACTIVE' "Settings sleep preserved"
require_text "$main" 'screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH' "touch wake preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "r12 battery source preserved"

require_text "$main" 'v0.1.28-r3 MJPEG Playback Responsiveness Repair' "v0.1.28-r3 boot marker"
require_text "$main" 'video-playback-responsiveness: touch-priority=ENABLED touch-stop=IMMEDIATE fps={} state=STOPPED,PLAYING,PAUSED,TOUCH_STOP config=FPS_TXT_DEFERRED,SKIP_TXT_DEFERRED' "playback responsiveness marker"
require_text "$main" 'VIDEO_PLAYBACK_FRAME_MS: u64 = 200' "2 FPS playback timer"
require_text "$main" 'VIDEO_PLAYBACK_STATE_STOPPED' "STOPPED state"
require_text "$main" 'VIDEO_PLAYBACK_STATE_PLAYING' "PLAYING state"
require_text "$main" 'VIDEO_PLAYBACK_STATE_PAUSED' "PAUSED state"
require_text "$main" 'VIDEO_PLAYBACK_STATE_TOUCH_STOP' "TOUCH_STOP state"
require_text "$main" 'fn video_playback_state_label' "state label helper"
require_text "$main" 'fn set_video_playback_state' "state transition helper"
require_text "$main" 'state=TOUCH STOP reason=touch-int action=DECODE_PAUSED priority=TOUCH' "touch stop log"
require_text "$main" 'touch_int.is_high()' "touch must be idle before decode ticker"
require_text "$main" 'video_playback_state_is_playing()' "decode only in PLAYING state"
require_text "$main" 'if !video_playback_state_is_playing()' "render no-decode guard"
require_text "$main" 'video_playback_state_label()' "visible playback status label"
require_text "$main" 'NO AUDIO - SWIPE AWAY TO STOP' "no-audio stop label preserved"
require_text "$main" 'SD FREE / TOTAL' "legacy storage detail marker preserved"
require_text "$main" 'screen-sleep: source=settings-display-sleep-now backlight=OFF render=PAUSED services=ACTIVE' "Settings sleep preserved"
require_text "$main" 'screen-wake: source=touch-int backlight=ON guard_ms={} action=NO_TOUCH_DISPATCH' "touch wake preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "r12 battery source preserved"

require_text "$main" 'v0.1.29 Dedicated Video Player Screen + Worker Task Foundation' "v0.1.29 boot marker"
require_text "$main" 'video-worker: dedicated Video page enabled; worker task decodes into PSRAM; UI displays latest frame; settings-storage=STATUS_PREVIEW_ONLY audio=DEFERRED' "video worker boot marker"
require_text "$pages" 'Video' "dedicated Video page enum"
require_text "$main" 'AssistantPage::Video => draw_video_player_tile' "Video page renderer"
require_text "$main" 'AssistantPage::Video => "VIDEO.RGB"' "Video SD asset name"
require_text "$main" 'fn draw_video_player_tile' "Video player screen renderer"
require_text "$main" 'fn render_video_worker_latest_frame' "UI latest worker frame copy"
require_text "$main" 'st77916_video_worker_start' "UI START worker request"
require_text "$main" 'st77916_video_worker_stop' "UI STOP worker request"
require_text "$main" 'st77916_video_worker_request_next' "UI NEXT_FRAME worker request"
require_text "$main" 'st77916_video_worker_copy_latest' "UI copies latest worker frame"
require_text "$ffi" 'st77916_video_worker_start' "worker FFI start declaration"
require_text "$ffi" 'st77916_video_worker_stop' "worker FFI stop declaration"
require_text "$ffi" 'st77916_video_worker_request_next' "worker FFI next declaration"
require_text "$ffi" 'st77916_video_worker_copy_latest' "worker FFI copy declaration"
require_text "$shim_h" 'st77916_video_worker_start' "worker C header start"
require_text "$shim_h" 'st77916_video_worker_request_next' "worker C header next"
require_text "$shim_c" 'st77916_video_worker_task' "FreeRTOS video worker task"
require_text "$shim_c" 'xTaskCreatePinnedToCore' "worker task creation"
require_text "$shim_c" 's_video_worker_decode_frame' "worker decode PSRAM buffer"
require_text "$shim_c" 's_video_worker_latest_frame' "worker latest PSRAM buffer"
require_text "$shim_c" 'st77916_video_worker_copy_latest' "worker latest frame copy API"
require_text "$main" 'model.current_page == AssistantPage::Video && model.video_status_text() == "READY"' "playback visibility moved to Video page"
require_text "$main" 'settings-storage=STATUS_PREVIEW_ONLY' "Settings storage playback removed marker"
require_text "$main" 'NO AUDIO - TOUCH STOPS' "Video page no-audio status"
require_text "$main" 'screen-sleep: source=settings-display-sleep-now backlight=OFF render=PAUSED services=ACTIVE' "Settings sleep preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "battery source preserved"

require_text "$main" 'v0.1.29-r1 Video Page Compile Repair' "v0.1.29-r1 boot marker"
require_text "$main" 'compile-repair: Video page added to ALL_PAGES length=6 and select-action match' "Video page compile repair marker"
require_text "$pages" 'AssistantPage::InternetRadio' "ALL_PAGES length 6"
require_text "$actions" 'VideoToggle' "VideoToggle action"
require_text "$actions" 'AssistantPage::Video => AppAction::VideoToggle' "Video page action match"
require_text "$actions" 'screen: VideoToggle files={} first={} playback=WORKER audio=DEFERRED' "Video action log"
require_text "$main" 'AssistantPage::Video => draw_video_player_tile' "Video renderer preserved"
require_text "$main" 'video-worker: dedicated Video page enabled' "worker marker preserved"

require_text "$main" 'v0.1.29-r2 Video Worker SD Ownership Repair' "v0.1.29-r2 boot marker"
require_text "$main" 'video-worker-sd-owner: worker SD mount/unmount disabled; single SD owner mutex foundation enabled; decode deferred until persistent SD mount; ui_status=WORKER_SD_OWNER' "SD ownership repair marker"
require_text "$main" 'WORKER SD OWNER' "UI worker SD owner status"
require_text "$main" 'WORKER SD BUSY' "UI worker SD busy status"
require_text "$main" 'SD OWNER - PLAYBACK DEFERRED' "UI no-waiting status message"
require_text "$main" 'fn video_worker_status_label' "worker status label helper"
require_text "$shim_c" 's_sd_owner_mutex' "single SD owner mutex foundation"
require_text "$shim_c" 'st77916_sd_owner_try_acquire' "SD owner try acquire helper"
require_text "$shim_c" 'ST77916_MJPEG_STATUS_WORKER_SD_OWNER' "worker SD owner status code"
require_text "$shim_c" 'ST77916_MJPEG_STATUS_WORKER_SD_BUSY' "worker SD busy status code"
require_text "$shim_c" 'st77916_sd_owner_status' "SD owner status API"
require_text "$ffi" 'st77916_sd_owner_status' "SD owner status FFI"
require_text "$shim_h" 'st77916_sd_owner_status' "SD owner status header"
worker_task_body=$(sed -n '/static void st77916_video_worker_task/,/bool st77916_video_worker_start/p' "$shim_c")
if printf '%s' "$worker_task_body" | grep -Fq 'st77916_probe_sd_mjpeg_library'; then echo "worker still probes SD internally" >&2; exit 1; fi
require_text "$main" 'settings-storage=STATUS_PREVIEW_ONLY' "Settings storage status-only marker preserved"
require_text "$main" 'AssistantPage::Video => draw_video_player_tile' "Video page renderer preserved"
require_text "$main" 'NO AUDIO - TOUCH STOPS' "no-audio Video page preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "battery source preserved"

require_text "$main" 'v0.1.29-r3 Video Worker Status Quieting + Version Marker Repair' "v0.1.29-r3 boot marker"
require_text "$main" 'version: v0.1.31-r2 Stream Playback Polish + Log Quieting' "v0.1.29-r3 version banner"
require_text "$main" 'video-worker-quiet: repeated worker status logs throttled log_every={} stable_ui_status=WORKER_SD_OWNER no_crash_expected=true' "worker quieting marker"
require_text "$main" 'VIDEO_WORKER_STATUS_LOG_EVERY: u32 = 30' "worker status log throttle interval"
require_text "$main" 'VIDEO_WORKER_STATUS_LOG_COUNTER' "worker status log counter"
require_text "$main" 'fn video_worker_status_log_allowed' "worker status log throttle helper"
require_text "$main" 'if video_worker_status_log_allowed()' "worker status log gate"
require_text "$main" 'log_every={}' "throttled log field"
require_text "$main" 'WORKER SD OWNER' "safe deferred UI preserved"
require_text "$main" 'SD OWNER - PLAYBACK DEFERRED' "stable video page status preserved"
require_text "$main" 'video-worker-sd-owner: worker SD mount/unmount disabled' "worker SD safety preserved"
require_text "$main" 'settings-storage=STATUS_PREVIEW_ONLY' "settings storage status-only preserved"
require_text "$main" 'AssistantPage::Video => draw_video_player_tile' "Video page preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "battery source preserved"

require_text "$main" 'v0.1.30 Persistent SD Mount Foundation' "v0.1.30 boot marker"
require_text "$main" 'version: v0.1.31-r2 Stream Playback Polish + Log Quieting' "v0.1.30 version banner"
require_text "$main" 'sd-persistent: session={} path=/sdcard owner_mutex=REAL mount_count={} repeated_mounts=DISABLED video_worker=ENABLED' "persistent SD boot status"
require_text "$main" 'sd-persistent-foundation: single_mount_session=ENABLED owner_mutex=REAL legacy_mount_calls=COMPAT_REUSE worker_reads=ALREADY_MOUNTED_PATH settings_storage_preview=PRESERVED' "persistent SD foundation marker"
require_text "$main" 'st77916_sd_persistent_mount_session' "Rust calls persistent SD mount session"
require_text "$ffi" 'st77916_sd_persistent_mount_session' "persistent SD FFI mount session"
require_text "$ffi" 'st77916_sd_persistent_is_ready' "persistent SD FFI ready"
require_text "$shim_h" 'st77916_sd_persistent_mount_session' "persistent SD header mount session"
require_text "$shim_c" 'st77916_sd_persistent_mount_compat' "persistent SD mount compatibility wrapper"
require_text "$shim_c" '#define esp_vfs_fat_sdmmc_mount st77916_sd_persistent_mount_compat' "legacy mount compatibility macro"
require_text "$shim_c" '#define esp_vfs_fat_sdcard_unmount st77916_sd_persistent_release_compat' "legacy unmount compatibility macro"
require_text "$shim_c" 's_sd_persistent_card' "persistent SD card handle"
require_text "$shim_c" 's_sd_persistent_mount_ready' "persistent SD ready flag"
require_text "$shim_c" 's_sd_owner_mutex' "real SD owner mutex"
require_text "$shim_c" 'st77916_sd_persistent_release_compat' "unmount calls release owner only"
require_text "$shim_c" 'st77916_decode_mjpeg_frame_rgb565' "worker decode path restored through compat mount"
require_text "$shim_c" 'v0.1.30-r2 yield while scanning long MJPEG files' "MJPEG scan WDT yield"
require_text "$main" 'video-playback: dedicated Video page worker playback fps={} audio=DEFERRED stop=TOUCH_OR_SWIPE' "stale Settings playback marker removed"
reject_text "$main" 'video-playback: loop enabled in Settings > Storage preview fps={} audio=DEFERRED stop=SWIPE_AWAY' "stale Settings playback marker"
require_text "$main" 'settings-storage=STATUS_PREVIEW_ONLY' "Settings storage status-only preserved"
require_text "$main" 'AssistantPage::Video => draw_video_player_tile' "Video page preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "battery source preserved"

require_text "$main" 'v0.1.30-r1 Video Worker Frame Advance Request Latch' "v0.1.30-r1 boot marker"
require_text "$main" 'version: v0.1.31-r2 Stream Playback Polish + Log Quieting' "v0.1.30-r1 version banner"
require_text "$main" 'video-worker-request-latch: next-frame requests latch while BUSY pending=ONE requested_frame_and_decoded_frame_logged=YES fps=2 sd_reinit=NO' "request latch boot marker"
require_text "$shim_c" 's_video_worker_next_latched' "worker next-frame latch flag"
require_text "$shim_c" 'state == ST77916_VIDEO_WORKER_BUSY' "request_next handles busy state"
require_text "$shim_c" 's_video_worker_next_latched = true' "request_next latches busy request"
require_text "$shim_c" 's_video_worker_next_latched = false' "worker consumes latch"
require_text "$shim_c" 's_video_worker_request_counter = s_video_worker_completed_counter + 1' "worker converts latch to next request"
require_text "$shim_c" 'video-stream-decode: requested_frame=%lu actual_frame=%lu decoded_frame=%lu' "worker requested/decoded frame log"
require_text "$shim_c" 's_video_stream_frame_index++' "stream worker advances frame index"
require_text "$shim_c" 'st77916_sd_persistent_is_ready' "persistent SD readiness preserved"
require_text "$shim_c" 'st77916_sd_persistent_mount_compat' "persistent SD compat mount preserved"
require_text "$main" 'sd-persistent-foundation: single_mount_session=ENABLED' "persistent SD marker preserved"
require_text "$main" 'video-playback: dedicated Video page worker playback fps={} audio=DEFERRED stop=TOUCH_OR_SWIPE' "2FPS Video page playback marker preserved"
reject_text "$main" 'video-playback: loop enabled in Settings > Storage preview fps={} audio=DEFERRED stop=SWIPE_AWAY' "stale Settings playback marker"
require_text "$main" 'AssistantPage::Video => draw_video_player_tile' "dedicated Video page preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "battery source preserved"

require_text "$main" 'v0.1.30-r2 MJPEG Indexed Frame Scanner Repair' "v0.1.30-r2 boot marker"
require_text "$main" 'version: v0.1.31-r2 Stream Playback Polish + Log Quieting' "v0.1.30-r2 version banner"
require_text "$main" 'mjpeg-indexed-scanner: requested_frame_maps_to_actual_frame=YES eof_loop_to_zero=YES offset_size_logged=YES fps=2 sd_reinit=NO' "indexed scanner marker"
require_text "$shim_c" 'static bool st77916_find_jpeg_frame_at_index' "indexed frame scanner function"
require_text "$shim_c" '*out_actual_index = frame_index' "scanner returns actual frame index"
require_text "$shim_c" 'bool looped_to_zero = false' "EOF loop-to-zero guard"
require_text "$shim_c" 'found_index = st77916_find_jpeg_frame_at_index(fp, 0' "EOF fallback to first frame"
require_text "$shim_c" 'out_result->decoded_frame_index = actual_index' "decoded frame uses actual index"
require_text "$shim_c" 'video-stream-decode: requested_frame=%lu actual_frame=%lu decoded_frame=%lu frame_offset=%lu frame_size=%lu' "worker log requested/actual/offset/size"
require_text "$shim_c" 'meta.first_frame_offset' "worker logs frame offset"
require_text "$shim_c" 'meta.first_frame_size' "worker logs frame size"
require_text "$shim_c" 'v0.1.30-r2 yield while scanning long MJPEG files' "scanner WDT yield marker"
require_text "$shim_c" 's_video_worker_next_latched' "request latch preserved"
require_text "$shim_c" 'st77916_sd_persistent_mount_compat' "persistent SD mount preserved"
require_text "$main" 'sd-persistent-foundation: single_mount_session=ENABLED' "persistent SD marker preserved"
require_text "$main" 'video-playback: dedicated Video page worker playback fps={} audio=DEFERRED stop=TOUCH_OR_SWIPE' "2FPS Video marker preserved"
reject_text "$main" 'video-playback: loop enabled in Settings > Storage preview fps={} audio=DEFERRED stop=SWIPE_AWAY' "stale Settings playback marker"
require_text "$main" 'AssistantPage::Video => draw_video_player_tile' "dedicated Video page preserved"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "battery source preserved"

require_text "$main" 'v0.1.31 Sequential MJPEG Streaming Worker' "v0.1.31 boot marker"
require_text "$main" 'version: v0.1.31-r2 Stream Playback Polish + Log Quieting' "v0.1.31 version banner"
require_text "$main" 'mjpeg-stream-worker: sequential=ENABLED open_once=YES rolling_buffer=64KB scan_from_start=NO publish_latest_rgb565=YES fps=2 audio=DEFERRED' "streaming worker marker"
require_text "$main" 'const VIDEO_PLAYBACK_FRAME_STEP: u32 = 6' "sequential frame step"
require_text "$shim_c" 'VIDEO_STREAM_BUFFER_BYTES (64U * 1024U)' "64KB rolling MJPEG buffer"
require_text "$shim_c" 's_video_stream_fp' "streaming file handle"
require_text "$shim_c" 's_video_stream_buf' "streaming PSRAM byte buffer"
require_text "$shim_c" 'st77916_video_stream_open_first' "open selected MJPEG once"
require_text "$shim_c" 'st77916_video_stream_locate_frame' "SOI/EOI rolling buffer scanner"
require_text "$shim_c" 'st77916_video_stream_read_more' "sequential SD reads"
require_text "$shim_c" 'st77916_video_stream_decode_next' "streaming decode next frame"
require_text "$shim_c" 'st77916_decode_jpeg_bytes_to_rgb565' "decode JPEG frame from RAM"
require_text "$shim_c" 'memmove(s_video_stream_buf' "shift leftover bytes"
require_text "$shim_c" 'video-stream-decode: requested_frame=%lu actual_frame=%lu decoded_frame=%lu frame_offset=%lu frame_size=%lu' "stream decode log"
require_text "$shim_c" 'st77916_sd_owner_try_acquire("VIDEO_STREAM_READ"' "SD owner used for stream reads"
require_text "$shim_c" 'st77916_sd_persistent_is_ready' "persistent SD readiness preserved"
require_text "$shim_c" 'st77916_sd_persistent_mount_compat' "persistent SD mount preserved"
require_text "$main" 'video-playback: dedicated Video page worker playback fps={} audio=DEFERRED stop=TOUCH_OR_SWIPE' "2FPS Video marker preserved"
require_text "$main" 'AssistantPage::Video => draw_video_player_tile' "dedicated Video page preserved"
require_text "$main" 'NO AUDIO - TOUCH STOPS' "touch stop/no audio preserved"
reject_text "$main" 'video-playback: loop enabled in Settings > Storage preview fps={} audio=DEFERRED stop=SWIPE_AWAY' "stale Settings playback marker"
require_text "$main" 'C-SHIM GPIO8 VENDOR' "battery source preserved"

require_text "$main" 'v0.1.31-r1 Streaming FPS + Source Frame Skip Runtime Tuning' "v0.1.31-r1 boot marker"
require_text "$main" 'version: v0.1.31-r2 Stream Playback Polish + Log Quieting' "v0.1.31-r1 version banner"
require_text "$main" 'video-stream-runtime-tuning: fps_txt=/VIDEO/FPS.TXT default_fps=5 skip_txt=/VIDEO/SKIP.TXT default_skip=6 source_fps=30 decode_selected_only=YES no_audio=YES' "runtime tuning marker"
require_text "$main" 'const VIDEO_PLAYBACK_FRAME_MS: u64 = 200' "default 5FPS playback interval"
require_text "$main" 'const VIDEO_PLAYBACK_FRAME_STEP: u32 = 6' "default source frame skip"
require_text "$main" 'st77916_video_worker_frame_ms' "dynamic worker frame pacing"
require_text "$shim_c" 'VIDEO_STREAM_DEFAULT_DISPLAY_FPS 5U' "default display FPS"
require_text "$shim_c" 'VIDEO_STREAM_DEFAULT_SOURCE_SKIP 6U' "default source frame skip"
require_text "$shim_c" 'VIDEO_STREAM_SOURCE_FPS 30U' "source FPS marker"
require_text "$shim_c" 'st77916_video_stream_read_u32_config' "runtime config reader"
require_text "$shim_c" '/sdcard/VIDEO/FPS.TXT' "FPS config path"
require_text "$shim_c" '/sdcard/VIDEO/SKIP.TXT' "SKIP config path"
require_text "$shim_c" 'st77916_video_stream_drop_one_frame' "drop skipped source frame"
require_text "$shim_c" 'dropped++' "skip loop drops source frames"
require_text "$shim_c" 'decode_selected_only' "placeholder"

require_text "$main" 'v0.1.31-r2 Stream Playback Polish + Log Quieting' "v0.1.31-r2 boot marker"
require_text "$main" 'version: v0.1.31-r2 Stream Playback Polish + Log Quieting' "v0.1.31-r2 version banner"
require_text "$main" 'video-stream-polish: normal_log_every=30 debug_per_frame=YES eof_wrap_actual_frame_0=YES timing=read_ms,skip_ms,decode_ms,publish_ms fps_tests=6x5,8x4' "stream polish marker"
require_text "$shim_c" 'VIDEO_STREAM_NORMAL_LOG_EVERY 30U' "normal stream log throttle interval"
require_text "$shim_c" 's_runtime_log_debug_enabled = debug_enabled' "C runtime debug flag wired"
require_text "$shim_c" 's_runtime_log_debug_enabled ||' "debug per-frame stream logging"
require_text "$shim_c" 'log_mode=%s log_every=%lu' "stream log mode and interval"
require_text "$shim_c" 'read_ms=%lu skip_ms=%lu decode_ms=%lu publish_ms=%lu' "stream timing log fields"
require_text "$shim_c" 's_video_stream_wrapped = true' "EOF wrap flag set"
require_text "$shim_c" 'if (s_video_stream_wrapped)' "EOF wrap skip break"
require_text "$shim_c" 'eof_wrap decodes frame 0' "EOF wrap documentation marker"
require_text "$shim_c" 's_video_stream_decode_log_counter' "decode log counter"
require_text "$shim_c" 'VIDEO_STREAM_REWIND' "stream rewind preserved"
require_text "$main" 'video-stream-runtime-tuning: fps_txt=/VIDEO/FPS.TXT' "FPS/SKIP tuning preserved"
require_text "$main" 'const VIDEO_PLAYBACK_FRAME_MS: u64 = 200' "5FPS default preserved"
require_text "$main" 'const VIDEO_PLAYBACK_FRAME_STEP: u32 = 6' "default skip preserved"
reject_text "$main" 'video-playback: loop enabled in Settings > Storage preview fps={} audio=DEFERRED stop=SWIPE_AWAY' "stale Settings playback marker"
