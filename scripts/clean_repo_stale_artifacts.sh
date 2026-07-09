#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"

find "$repo_root" -type d -name '_archive_*' -prune -exec rm -rf {} +
rm -rf "$repo_root/tmp" "$repo_root/.cleanup" "$repo_root/dist"
find "$repo_root" -name '.DS_Store' -delete

echo "Repository cleanup helper: OK"
