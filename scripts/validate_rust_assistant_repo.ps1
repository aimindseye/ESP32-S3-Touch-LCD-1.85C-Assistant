$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$RustDir = Join-Path $Root "firmware\assistant-rs"
$ArduinoRef = Join-Path $Root "firmware\reference\arduino-v1-baseline"

$RequiredRustPaths = @(
  "Cargo.toml",
  "src",
  ".cargo",
  "components\st77916_shim",
  "build.rs",
  "sdkconfig.defaults"
)

if (!(Test-Path $RustDir)) {
  throw "Missing canonical Rust firmware directory: firmware\assistant-rs"
}

foreach ($Path in $RequiredRustPaths) {
  $FullPath = Join-Path $RustDir $Path
  if (!(Test-Path $FullPath)) {
    throw "Missing Rust full-port path: firmware\assistant-rs\$Path"
  }
}

if (!(Test-Path $ArduinoRef)) {
  throw "Missing Arduino V1 hardware reference under firmware\reference\arduino-v1-baseline"
}

if (!(Test-Path (Join-Path $ArduinoRef "Assistant.ino"))) {
  throw "Missing preserved Arduino reference sketch"
}

Write-Host "Rust Assistant structure: OK"
Write-Host "Arduino V1 hardware reference: OK"
