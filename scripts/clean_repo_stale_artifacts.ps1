$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")

$staleRelativePaths = @(
    "source-asset-manifest.json",
    "source-asset-overlay_map.json",
    "asset-overlay_map.json",
    "docs/SD_CARD_LAYOUT.md",
    "docs/ASSISTANT_ARCHITECTURE.md",
    "docs/PHASE_0_FOUNDATION.md",
    "docs/PROJECT_REFERENCES.md",
    "docs/UI_BASELINE_v0.1.14.md",
    "docs/REGRESSION_GUARDS_v0.1.14.md",
    "scripts/bootstrap_from_reference_repo.ps1",
    "scripts/validate_assistant_repo.ps1",
    "asset-previews/home_default_base.png",
    "asset-previews/home_default_base.rgb565-preview.png",
    "firmware/assistant-rs/components/lvgl_lab_bridge",
    "firmware/assistant-rs/src/ui",
)

foreach ($relative in $staleRelativePaths) {
    $path = Join-Path $repoRoot $relative
    if (Test-Path $path) {
        Remove-Item -Recurse -Force $path
        Write-Host "Removed stale repo artifact: $relative"
    }
}

$assetDir = Join-Path $repoRoot "firmwaressistant-rsssets
gb565"
$allowedAssets = @(
    "home_base.rgb565",
    "weather_base.rgb565",
    "music_base.rgb565",
    "assistant_base.rgb565",
    "settings_base.rgb565"
)

if (Test-Path $assetDir) {
    Get-ChildItem $assetDir -Filter "*.rgb565" | ForEach-Object {
        if ($allowedAssets -notcontains $_.Name) {
            Remove-Item -Force $_.FullName
            Write-Host "Removed stale RGB565 asset: $($_.Name)"
        }
    }
}

Write-Host "Repository stale-artifact cleanup: OK"
