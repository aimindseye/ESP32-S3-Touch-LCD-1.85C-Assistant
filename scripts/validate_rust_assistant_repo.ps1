$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$firmwareDir = Join-Path $repoRoot "firmware\assistant-rs"

function Require-File([string]$path) {
    if (-not (Test-Path $path -PathType Leaf)) {
        throw "Missing required file: $path"
    }
}

$requiredFiles = @(
    "Cargo.toml",
    "rust-toolchain.toml",
    "build.rs",
    ".cargo\config.toml",
    "sdkconfig.defaults",
    "partitions.csv",
    "src\main.rs",
    "src\app\mod.rs",
    "src\app\intents.rs",
    "src\app\providers.rs",
    "src\app\state.rs",
    "src\app\settings.rs",
    "src\app\time.rs",
    "src\app\model.rs",
    "src\app\actions.rs",
    "src\app\pages.rs",
    "assets\rgb565\home_base.rgb565",
    "assets\rgb565\weather_base.rgb565",
    "assets\rgb565\music_base.rgb565",
    "assets\rgb565\assistant_base.rgb565",
    "assets\rgb565\settings_base.rgb565"
)

foreach ($relative in $requiredFiles) {
    Require-File (Join-Path $firmwareDir $relative)
}

& (Join-Path $PSScriptRoot "validate_ui_baseline_freeze.ps1")

Write-Host "Rust assistant repo validation: OK"
