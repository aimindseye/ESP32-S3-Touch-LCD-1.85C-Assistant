$ErrorActionPreference = "Stop"
$RepoRoot = Resolve-Path "$PSScriptRoot/.."
& "$RepoRoot/scripts/validate_assistant_current.sh" "$RepoRoot"
Push-Location "$RepoRoot/firmware/assistant-rs"
try { cargo build --release } finally { Pop-Location }
