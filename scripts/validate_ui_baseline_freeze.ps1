$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$firmwareDir = Join-Path $repoRoot "firmware\assistant-rs"
$mainPath = Join-Path $firmwareDir "src\main.rs"
$pagesPath = Join-Path $firmwareDir "src\app\pages.rs"
$cargoPath = Join-Path $firmwareDir "Cargo.toml"
$assetDir = Join-Path $firmwareDir "assets\rgb565"

function Require-Text([string]$content, [string]$pattern, [string]$label) {
    if ($content -notmatch [regex]::Escape($pattern)) {
        throw "$label missing: $pattern"
    }
}

function Reject-Text([string]$content, [string]$pattern, [string]$label) {
    if ($content -match [regex]::Escape($pattern)) {
        throw "$label contains forbidden/stale marker: $pattern"
    }
}

if (-not (Test-Path $mainPath -PathType Leaf)) { throw "Missing src/main.rs" }
if (-not (Test-Path $pagesPath -PathType Leaf)) { throw "Missing src/app/pages.rs" }
if (-not (Test-Path $cargoPath -PathType Leaf)) { throw "Missing Cargo.toml" }

$main = Get-Content $mainPath -Raw
$pages = Get-Content $pagesPath -Raw
$cargo = Get-Content $cargoPath -Raw

if ($cargo -notmatch 'version\s*=\s*"0\.1\.14"') {
    throw "Cargo.toml version must be 0.1.14 for v0.1.14"
}

$expectedHashes = @{
    "home_base.rgb565" = "e08c25e66648989237b310728f0048593aad54dbdb2444397d8eede64b4d7744"
    "weather_base.rgb565" = "1d1e82c3936e281c7e3affc9cbe099c81c80d213aacd2d73a5e972b1033424ce"
    "music_base.rgb565" = "5757144b865cce59569be9504da650d77113d9bdfc4e026e626c3ee82a4c9a3a"
    "assistant_base.rgb565" = "7e24c36c419d2832fc7b3df09f5d0d191849efc1dac2c6098b676af3591003fa"
    "settings_base.rgb565" = "54bafea9208ad49c57d34dac40865988b4967bd775aaad9a0a5c06bba9fe58e5"
}

# v0.1.14-r1: Copy-Item -Recurse does not delete stale assets from older labs.
# Remove non-frozen RGB565 files first, then enforce the exact accepted five-asset set.
$allAssetsBeforeCleanup = Get-ChildItem $assetDir -Filter "*.rgb565" | Sort-Object Name
foreach ($asset in $allAssetsBeforeCleanup) {
    if (-not $expectedHashes.ContainsKey($asset.Name)) {
        Write-Host "Removing stale non-frozen RGB565 asset: $($asset.Name)"
        Remove-Item -Force $asset.FullName
    }
}

$actualAssets = Get-ChildItem $assetDir -Filter "*.rgb565" | Sort-Object Name
if ($actualAssets.Count -ne 5) {
    throw "Expected exactly 5 frozen RGB565 base assets after cleanup, found $($actualAssets.Count)"
}

foreach ($asset in $actualAssets) {
    if (-not $expectedHashes.ContainsKey($asset.Name)) {
        throw "Unexpected RGB565 asset present after cleanup: $($asset.Name)"
    }
    if ($asset.Length -ne 259200) {
        throw "RGB565 asset has wrong size: $($asset.Name) length=$($asset.Length) expected=259200"
    }
    $hash = (Get-FileHash -Algorithm SHA256 $asset.FullName).Hash.ToLowerInvariant()
    if ($hash -ne $expectedHashes[$asset.Name]) {
        throw "RGB565 asset hash changed for $($asset.Name): $hash expected=$($expectedHashes[$asset.Name])"
    }
}

foreach ($pattern in @(
    'v0.1.14-r2 Weather Baseline Guard Marker Repair',
    'Screens frozen: Home r3 | Weather r8-r2 | Music v0.1.11 | Assistant v0.1.12 | Settings Option A',
    'Input: r12 gesture-first touch, poll={}ms, window={}ms, cooldown={}ms',
    'Renderer: hybrid RGB565 five page assets + dynamic overlays',
    'Integrations: mocked/local; periodic SD/GPIO refresh disabled',
    'UI baseline: frozen five-screen layout with regression guards',
    'Asset guard: stale non-frozen RGB565 files are cleaned before validation',
    'Weather guard: timeline temp marker aligned with accepted y=262 layout',
    'Build cleanup: retained fallback helpers use crate-level dead_code allowance'
)) { Require-Text $main $pattern "boot baseline" }

foreach ($pattern in @(
    'pub const ALL_PAGES: [AssistantPage; 5]',
    'AssistantPage::Home',
    'AssistantPage::Weather',
    'AssistantPage::Music',
    'AssistantPage::Assistant',
    'AssistantPage::Settings',
    'Self::Home => 0',
    'Self::Weather => 1',
    'Self::Music => 2',
    'Self::Assistant => 3',
    'Self::Settings => 4'
)) { Require-Text $pages $pattern "five-page order" }

foreach ($pattern in @(
    'blit_rgb565_asset(frame, HOME_BASE_RGB565)',
    'draw_home_battery_complication(frame, 66, 58, model.battery_percent_value())',
    'draw_text(frame, 86, 53, &model.battery_home_text(), WHITE, 1)',
    'draw_wifi_icon(frame, 218, 58, WHITE)',
    'draw_text(frame, 238, 53, &model.wifi_home_text(), WHITE, 1)',
    'draw_text_centered_at(frame, 180, 102, &model.rtc_home_date_text(), WHITE, 2)',
    'draw_numeric_value_centered(frame, 122, &model.rtc_hms(), 42, 6, WHITE)',
    'draw_home_weather_icon(frame, 106, 250, condition)',
    'draw_text_centered_at(frame, 106, 278, condition, WHITE, 1)',
    'draw_text_centered_at(frame, 247, 262, model.home_weather_temp(), WHITE, 3)'
)) { Require-Text $main $pattern "Home baseline" }

foreach ($pattern in @(
    'blit_rgb565_asset(frame, WEATHER_BASE_RGB565)',
    'draw_numeric_value_centered(frame, 70, temp, 24, 4, WHITE)',
    'draw_text_centered(frame, 138, condition, WHITE, 2)',
    'draw_weather_hour_values(frame, condition)',
    'fn draw_weather_hour_values',
    'WeatherHourMock { hour: "11A"',
    'draw_text_centered_at(frame, cx, 190, entry.hour, hour_color, 2)',
    'draw_text_centered_at(frame, cx, 262, entry.temp, WHITE, 2)'
)) { Require-Text $main $pattern "Weather baseline" }

foreach ($pattern in @(
    'blit_rgb565_asset(frame, MUSIC_BASE_RGB565)',
    'v0.1.11 Music Screen Option C Minimal Equalizer',
    'draw_text_centered_at(frame, 180, 38, &model.rtc_hms(), WHITE, 2)',
    'draw_text_centered_at(frame, 180, 134, model.music.track_label, WHITE, 3)',
    'draw_text_centered_at(frame, 180, 162, model.music.subtitle_label, ACCENT_MUSIC_BLUE, 2)',
    'draw_music_transport_controls(frame, model.music.playing)',
    'draw_music_progress_row(',
    'draw_text_centered_at(frame, 180, 318, model.music.source, WHITE, 1)'
)) { Require-Text $main $pattern "Music baseline" }

foreach ($pattern in @(
    'blit_rgb565_asset(frame, ASSISTANT_BASE_RGB565)',
    'v0.1.12 AI Assistant Option B Conversation Card',
    'draw_waveform(',
    'draw_text_centered_at(frame, 180, 112, model.assistant.title_label(), WHITE, 3)',
    'draw_text_centered_at(frame, 180, 140, model.assistant.subtitle_label(), MUTED, 2)',
    'draw_assistant_robot_badge(frame, 91, 190, model.assistant.listening)',
    'draw_text(frame, 126, 178, model.assistant.card_label(), WHITE, 2)',
    'draw_text(frame, 126, 204, model.assistant.card_aux_label(), MUTED, 1)',
    'draw_microphone_button(frame, 180, 272, model.assistant.listening)',
    'draw_cancel_glyph(frame, 116, 272, MUTED)'
)) { Require-Text $main $pattern "Assistant baseline" }

foreach ($pattern in @(
    'blit_rgb565_asset(frame, SETTINGS_BASE_RGB565)',
    'v0.1.13 Settings Screen Option A List Style',
    'draw_text_centered_at(frame, 180, 32, &model.rtc_hms(), MUTED, 1)',
    'draw_text_centered_at(frame, 180, 76, "SETTINGS", ACCENT_SETTINGS, 2)',
    'draw_settings_list_row(frame, 55, 96, "WI-FI", SettingsIcon::Wifi, false)',
    'draw_settings_list_row(frame, 55, 146, "DISPLAY", SettingsIcon::Display, true)',
    'draw_settings_list_row(frame, 55, 196, "SOUND", SettingsIcon::Sound, false)',
    'draw_settings_list_row(frame, 55, 246, "ABOUT", SettingsIcon::About, false)',
    'draw_text(frame, x + 74, y + 26, label, WHITE, 2)',
    'draw_text_centered_at(frame, 180, 76, "DISPLAY", ACCENT_SETTINGS, 2)',
    'draw_text(frame, 78, 271, "QUIET RENDER", WHITE, 1)',
    'fn stroke_rounded_rect'
)) { Require-Text $main $pattern "Settings baseline" }

foreach ($pattern in @(
    'const TOUCH_POLL_MS: u64 = 8;',
    'const TOUCH_ACTIVE_POLL_WINDOW_MS: u64 = 180;',
    'const TOUCH_NO_TOUCH_FINISH_COUNT: u8 = 3;',
    'const TOUCH_GESTURE_SPAN_PREFER_PX: i16 = 35;',
    'const UNIVERSAL_SWIPE_MIN_DX: i16 = 20;',
    'const CENTER_TAP_MAX_MOVE_PX: i16 = 12;',
    'touch-class: gesture-left accepted next',
    'touch-class: gesture-right accepted previous',
    'touch-class: span swipe-left accepted next',
    'touch-class: span swipe-right accepted previous'
)) { Require-Text $main $pattern "r12 touch baseline" }

foreach ($pattern in @(
    'Preserve v0.1.5-r1 validated behavior',
    'r3a build fix: last_render initialized before first render',
    'ADC battery read: inline accepted baseline path',
    'Keep v0.1.3 circular UI direction',
    'Weather lane repair: smaller temp glyph, lower condition label, no overlap',
    'Weather icon repair: clear/sunny uses sun-only icon above temp lane',
    'Preserve v0.1.13-r1 Settings Option A Compile Repair',
    'v0.1.13-r2 settings text alignment: header and rows rebalanced',
    'v0.1.13-r3 Settings Baseline Alignment and Log Cleanup',
    'v0.1.14 Five-Screen UI Baseline Freeze + Regression Guards',
    'v0.1.14-r1 UI Baseline Freeze Asset Inventory Repair',
    'draw_text_centered_at(frame, 180, 64, "SETTINGS", ACCENT_SETTINGS, 2)',
    'draw_text_centered_at(frame, 180, 64, "DISPLAY", ACCENT_SETTINGS, 2)',
    'draw_text(frame, x + 62, y + 10, label, WHITE, 2)',
    'draw_text(frame, 78, 260, "QUIET RENDER", WHITE, 1)',
    'draw_scroll_arc(frame);',
    'draw_toggle_ring(frame, 180, 132, model.settings.quiet_render_enabled);',
    'draw_text_centered(frame, 128, "QUIET", WHITE, 2);',
    'SD_REFRESH_MS',
    'last_sd',
    'TOUCH_RELEASE_DEBOUNCE_MS',
    'finish_if_debounced',
    'touch-class: universal swipe-left accepted next',
    'touch-class: universal swipe-right accepted previous',
    'lvgl_lab_bridge',
    'esp_lvgl_port'
)) { Reject-Text $main $pattern "UI baseline freeze" }

foreach ($pattern in @('lvgl','esp_lvgl_port','reqwest','ureq','embedded-svc.*http')) {
    if ($cargo -match $pattern) { throw "Cargo.toml contains forbidden integration dependency pattern: $pattern" }
}

Write-Host "UI baseline freeze validation: OK"
