param(
  [string]$Port = ""
)
$ErrorActionPreference = "Stop"
$RepoRoot = Resolve-Path "$PSScriptRoot/.."
if ($Port -eq "") { throw "Provide -Port COMx or /dev/cu.usbmodemXXXX" }
& "$RepoRoot/scripts/validate_assistant_current.sh" "$RepoRoot"
Push-Location "$RepoRoot/firmware/assistant-rs"
try { cargo espflash flash --release --monitor --port $Port } finally { Pop-Location }
