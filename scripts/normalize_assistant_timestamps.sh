#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"

find "$repo_root/firmware/assistant-rs" "$repo_root/scripts" "$repo_root/docs" \
  -type f \( -name '*.rs' -o -name '*.toml' -o -name '*.csv' -o -name '*.defaults' -o -name '*.ps1' -o -name '*.sh' -o -name '*.md' \) \
  -exec touch {} +

echo "Normalized assistant source timestamps for macOS build"
