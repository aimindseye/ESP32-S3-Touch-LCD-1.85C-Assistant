$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
$Required = @(
  "README.md",
  "docs\ASSISTANT_ARCHITECTURE.md",
  "docs\PHASE_0_FOUNDATION.md",
  "docs\SD_CARD_LAYOUT.md",
  "firmware\Assistant",
  "scripts\bootstrap_from_reference_repo.ps1"
)

foreach ($Path in $Required) {
  $Full = Join-Path $Root $Path
  if (-not (Test-Path $Full)) {
    throw "Missing required path: $Path"
  }
}

$Sketch = Join-Path $Root "firmware\Assistant\Assistant.ino"
if (Test-Path $Sketch) {
  $Text = Get-Content $Sketch -Raw
  foreach ($Needle in @("Audio.h", "SD_MMC", "I2S_BCLK", "Lvgl_Init", "audio.loop")) {
    if ($Text -notmatch [regex]::Escape($Needle)) {
      throw "Assistant.ino appears to be missing expected baseline marker: $Needle"
    }
  }
  Write-Host "Assistant sketch markers: OK"
} else {
  Write-Host "Assistant.ino not imported yet. Run scripts\bootstrap_from_reference_repo.ps1" -ForegroundColor Yellow
}

Write-Host "Assistant repo structure: OK"
