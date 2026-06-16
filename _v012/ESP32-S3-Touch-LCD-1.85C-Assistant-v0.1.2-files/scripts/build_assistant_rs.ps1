param(
    [string]$TargetDir = "C:\t"
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$firmwareDir = Join-Path $repoRoot "firmware\assistant-rs"

& (Join-Path $PSScriptRoot "fix_assistant_partition_path.ps1")
& (Join-Path $PSScriptRoot "validate_rust_assistant_repo.ps1")

New-Item -ItemType Directory -Force $TargetDir | Out-Null
$env:CARGO_TARGET_DIR = $TargetDir
$env:RUST_BACKTRACE = "1"

Push-Location $firmwareDir
try {
    cargo build --release
}
finally {
    Pop-Location
}
