$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
$ProjectParent = Split-Path -Parent $Root
$ReferencePath = Join-Path $ProjectParent "ESP32-S3-Touch-LCD-1.85C"
$ReferenceGit = "https://github.com/aimindseye/ESP32-S3-Touch-LCD-1.85C.git"
$ReferenceSketch = Join-Path $ReferencePath "demo\original\Arduino\LVGL_Arduino"
$DestSketch = Join-Path $Root "firmware\Assistant"

if (-not (Test-Path $ReferencePath)) {
  Write-Host "Reference repo not found at $ReferencePath"
  Write-Host "Cloning reference repo..."
  git clone $ReferenceGit $ReferencePath
}

if (-not (Test-Path $ReferenceSketch)) {
  throw "Reference sketch not found: $ReferenceSketch"
}

New-Item -ItemType Directory -Force -Path $DestSketch | Out-Null

$FilesToCopy = @(
  "BAT_Driver.cpp",
  "BAT_Driver.h",
  "Display_ST77916.cpp",
  "Display_ST77916.h",
  "I2C_Driver.cpp",
  "I2C_Driver.h",
  "LVGL_Driver.cpp",
  "LVGL_Driver.h",
  "RTC_PCF85063.cpp",
  "RTC_PCF85063.h",
  "SD_Card.cpp",
  "SD_Card.h",
  "TCA9554PWR.cpp",
  "TCA9554PWR.h",
  "Touch_CST816.cpp",
  "Touch_CST816.h",
  "esp_lcd_st77916.c",
  "esp_lcd_st77916.h"
)

foreach ($Name in $FilesToCopy) {
  Copy-Item -Force (Join-Path $ReferenceSketch $Name) (Join-Path $DestSketch $Name)
}

$SourceIno = Join-Path $ReferenceSketch "LVGL_Arduino.ino"
$DestIno = Join-Path $DestSketch "Assistant.ino"
$Ino = Get-Content $SourceIno -Raw

# Minimal Phase 0 branding changes. Keep hardware behavior unchanged.
$Ino = $Ino.Replace("Booting LVGL audio app...", "Booting ESP32-S3 Assistant...")
$Ino = $Ino.Replace("ESP32-S3 Touch 1.85", "ESP32-S3 Assistant")
$Ino = $Ino.Replace("Status + Player", "Home + Music")
$Ino = $Ino.Replace('"Player"', '"Music"')

Set-Content -Path $DestIno -Value $Ino -Encoding UTF8

Write-Host "Imported known-good Arduino/LVGL baseline into firmware\Assistant"
Write-Host "Next: open firmware\Assistant\Assistant.ino in Arduino IDE and flash with the known-good V1 settings."
