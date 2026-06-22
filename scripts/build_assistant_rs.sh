#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
firmware_dir="$repo_root/firmware/assistant-rs"

clean=0
if [[ "${1:-}" == "--clean" || "${1:-}" == "-c" ]]; then
  clean=1
fi

"$script_dir/normalize_assistant_timestamps.sh"
"$script_dir/fix_assistant_partition_path.sh"
"$script_dir/validate_rust_assistant_repo.sh"

cd "$firmware_dir"
if [[ "$clean" == "1" ]]; then
  rm -rf "${CARGO_TARGET_DIR:-target}"
fi

cargo +esp build --release
