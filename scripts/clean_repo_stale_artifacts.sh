#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"

stale_paths=(
  "source-asset-manifest.json"
  "source-asset-overlay_map.json"
  "asset-overlay_map.json"
  "docs/SD_CARD_LAYOUT.md"
  "docs/ASSISTANT_ARCHITECTURE.md"
  "docs/PHASE_0_FOUNDATION.md"
  "docs/PROJECT_REFERENCES.md"
  "docs/UI_BASELINE_v0.1.14.md"
  "docs/REGRESSION_GUARDS_v0.1.14.md"
  "scripts/bootstrap_from_reference_repo.ps1"
  "scripts/validate_assistant_repo.ps1"
  "asset-previews/home_default_base.png"
  "asset-previews/home_default_base.rgb565-preview.png"
  "firmware/assistant-rs/components/lvgl_lab_bridge"
  "firmware/assistant-rs/src/ui"
)

for relative in "${stale_paths[@]}"; do
  path="$repo_root/$relative"
  if [[ -e "$path" ]]; then
    rm -rf "$path"
    echo "Removed stale repo artifact: $relative"
  fi
done

asset_dir="$repo_root/firmware/assistant-rs/assets/rgb565"
if [[ -d "$asset_dir" ]]; then
  for asset in "$asset_dir"/*.rgb565; do
    [[ -e "$asset" ]] || continue
    name="$(basename "$asset")"
    case "$name" in
      home_base.rgb565|weather_base.rgb565|music_base.rgb565|assistant_base.rgb565|settings_base.rgb565)
        ;;
      *)
        rm -f "$asset"
        echo "Removed stale RGB565 asset: $name"
        ;;
    esac
  done
fi

echo "Repository stale-artifact cleanup: OK"
