param(
    [string]$TargetDir = "C:\t",
    [switch]$Clean
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$firmwareDir = Join-Path $repoRoot "firmware\assistant-rs"

& (Join-Path $PSScriptRoot "normalize_assistant_timestamps.ps1")
& (Join-Path $PSScriptRoot "fix_assistant_partition_path.ps1")
& (Join-Path $PSScriptRoot "validate_rust_assistant_repo.ps1")

if ($Clean) {
    Write-Host "Removing Cargo target dir: $TargetDir"
    Remove-Item -Recurse -Force $TargetDir -ErrorAction SilentlyContinue
}

New-Item -ItemType Directory -Force $TargetDir | Out-Null
$env:CARGO_TARGET_DIR = $TargetDir
$env:RUST_BACKTRACE = "1"

Push-Location $firmwareDir
try {
    cargo +esp build --release
}
finally {
    Pop-Location
}
