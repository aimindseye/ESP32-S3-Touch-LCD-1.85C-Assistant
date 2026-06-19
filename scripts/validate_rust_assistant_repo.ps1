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
    "src\app\actions.rs",
    "src\app\home.rs",
    "src\app\weather.rs",
    "src\app\music.rs",
    "src\app\assistant.rs",
    "src\app\settings.rs",
    "assets\rgb565\home_base.rgb565",
    "assets\rgb565\weather_base.rgb565",
    "assets\rgb565\music_base.rgb565",
    "assets\rgb565\assistant_base.rgb565",
    "assets\rgb565\settings_base.rgb565",
    "components\st77916_shim\CMakeLists.txt",
    "components\st77916_shim\st77916_shim.c",
    "components\st77916_shim\esp_lcd_st77916.c"
)

foreach ($relative in $requiredFiles) {
    Require-File (Join-Path $firmwareDir $relative)
}

& (Join-Path $PSScriptRoot "validate_ui_baseline_freeze.ps1")

Write-Host "Rust assistant repo validation: OK"
