$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$firmwareDir = Join-Path $repoRoot "firmware\assistant-rs"

function Require-File([string]$path) {
    if (-not (Test-Path $path -PathType Leaf)) {
        throw "Missing required file: $path"
    }
}

function Require-Dir([string]$path) {
    if (-not (Test-Path $path -PathType Container)) {
        throw "Missing required directory: $path"
    }
}

Require-Dir $firmwareDir

$requiredFiles = @(
    "Cargo.toml",
    "build.rs",
    ".cargo\config.toml",
    "sdkconfig.defaults",
    "partitions.csv",
    "src\main.rs",
    "src\app\mod.rs",
    "src\app\model.rs",
    "src\app\pages.rs",
    "components\st77916_shim\CMakeLists.txt",
    "components\st77916_shim\st77916_shim.c",
    "components\st77916_shim\esp_lcd_st77916.c"
)

foreach ($relative in $requiredFiles) {
    Require-File (Join-Path $firmwareDir $relative)
}

$cargo = Get-Content (Join-Path $firmwareDir "Cargo.toml") -Raw
if ($cargo -notmatch 'version\s*=\s*"0\.1\.3"') {
    throw "Cargo.toml version must be 0.1.3"
}
if ($cargo -notmatch 'extra_components') {
    throw "Cargo.toml missing esp-idf-sys extra_components metadata"
}
if ($cargo -notmatch 'components/st77916_shim') {
    throw "Cargo.toml missing components/st77916_shim extra component"
}

$sdk = Get-Content (Join-Path $firmwareDir "sdkconfig.defaults") -Raw
foreach ($pattern in @(
    'CONFIG_SPIRAM=y',
    'CONFIG_ESPTOOLPY_FLASHSIZE_16MB=y',
    'CONFIG_PARTITION_TABLE_CUSTOM=y',
    'CONFIG_PARTITION_TABLE_CUSTOM_FILENAME=',
    'CONFIG_PARTITION_TABLE_FILENAME=',
    'CONFIG_PARTITION_TABLE_OFFSET=0x8000'
)) {
    if ($sdk -notmatch [regex]::Escape($pattern)) {
        throw "sdkconfig.defaults missing $pattern"
    }
}

$partitionFilename = [regex]::Match($sdk, '(?m)^CONFIG_PARTITION_TABLE_FILENAME="?([^"\r\n]+)"?').Groups[1].Value
if ([string]::IsNullOrWhiteSpace($partitionFilename)) {
    throw "Unable to parse CONFIG_PARTITION_TABLE_FILENAME"
}
if (-not (Test-Path $partitionFilename -PathType Leaf)) {
    throw "Partition table path does not exist: $partitionFilename. Run scripts\fix_assistant_partition_path.ps1"
}

$main = Get-Content (Join-Path $firmwareDir "src\main.rs") -Raw
foreach ($pattern in @(
    'Active CST816 Polling and Gesture-First Navigation',
    'r12 touch contract: INT starts tracking, active poll gathers samples',
    'Touch classifier: gesture-first, span swipe dx>=20, active poll=8ms window=180ms',
    'TOUCH_POLL_MS: u64 = 8',
    'TOUCH_ACTIVE_POLL_WINDOW_MS',
    'TOUCH_NO_TOUCH_FINISH_COUNT',
    'TOUCH_GESTURE_SPAN_PREFER_PX',
    'no_touch_count',
    'finish_reason',
    'touch-track: begin id=',
    'touch-track: sample id={} source={}',
    'touch-track: no-touch id=',
    'touch-track: finish id={} reason={}',
    'touch-class: gesture-left accepted next',
    'touch-class: gesture-right accepted previous',
    'touch-class: span swipe-left accepted next',
    'touch-class: span swipe-right accepted previous',
    'touch-class: ignored insufficient samples',
    'gesture/span disagree',
    'SWIPE ANYWHERE',
    'gpio-status: initialized once',
    'gpio-status: periodic reconfigure disabled',
    'BOOT runtime control reserved while USB monitor is attached',
    'POWER candidate logging: GPIO6 experimental home/menu',
    'st77916_panel_draw_rgb565',
    'ADC battery read: inline accepted baseline path',
    'Watch UI polish: no divider lines, dim one-pixel outer ring'
)) {
    if ($main -notmatch [regex]::Escape($pattern)) {
        throw "src/main.rs missing r12 marker: $pattern"
    }
}

foreach ($pattern in @(
    'const\s+TOUCH_POLL_MS:\s*u64\s*=\s*8\s*;',
    'const\s+TOUCH_ACTIVE_POLL_WINDOW_MS:\s*u64\s*=\s*180\s*;',
    'const\s+TOUCH_NO_TOUCH_FINISH_COUNT:\s*u8\s*=\s*3\s*;',
    'const\s+TOUCH_GESTURE_SPAN_PREFER_PX:\s*i16\s*=\s*35\s*;',
    'const\s+UNIVERSAL_SWIPE_MIN_DX:\s*i16\s*=\s*20\s*;',
    'const\s+CENTER_TAP_MAX_MOVE_PX:\s*i16\s*=\s*12\s*;',
    'const\s+CST816_GESTURE_LEFT:\s*u8\s*=\s*0x03\s*;',
    'const\s+CST816_GESTURE_RIGHT:\s*u8\s*=\s*0x04\s*;'
)) {
    if ($main -notmatch $pattern) {
        throw "src/main.rs has incorrect r12 constant: $pattern"
    }
}

foreach ($forbidden in @(
    'TOUCH_RELEASE_DEBOUNCE_MS',
    'finish_if_debounced',
    'release_pending_since',
    'touch-track: release-pending id=',
    'touch-class: universal swipe-left accepted next',
    'touch-class: universal swipe-right accepted previous'
)) {
    if ($main -match [regex]::Escape($forbidden)) {
        throw "src/main.rs still contains stale r11 marker: $forbidden"
    }
}

if ($main -match 'SD_REFRESH_MS') {
    throw "src/main.rs still contains SD_REFRESH_MS periodic setting"
}
if ($main -match 'last_sd') {
    throw "src/main.rs still contains periodic SD refresh state last_sd"
}

$model = Get-Content (Join-Path $firmwareDir "src\app\model.rs") -Raw
foreach ($pattern in @(
    'ButtonName',
    'ButtonPressKind',
    'UiIntent',
    'BootReserved',
    'button_events',
    'nav_events',
    'last_action'
)) {
    if ($model -notmatch [regex]::Escape($pattern)) {
        throw "src/app/model.rs missing button/navigation marker: $pattern"
    }
}

$pages = Get-Content (Join-Path $firmwareDir "src\app\pages.rs") -Raw
foreach ($page in @('Home', 'Weather', 'Music', 'Settings')) {
    if ($pages -notmatch $page) {
        throw "src/app/pages.rs missing page: $page"
    }
}

Write-Host "Rust assistant repo validation: OK"
