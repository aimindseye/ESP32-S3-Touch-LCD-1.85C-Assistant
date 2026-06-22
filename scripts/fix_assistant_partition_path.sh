#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
firmware_dir="$repo_root/firmware/assistant-rs"
sdkconfig="$firmware_dir/sdkconfig.defaults"
cargo_config="$firmware_dir/.cargo/config.toml"
partition_path="$firmware_dir/partitions.csv"

python3 - "$sdkconfig" "$partition_path" <<'PY'
from pathlib import Path
import sys

sdkconfig = Path(sys.argv[1])
partition_path = Path(sys.argv[2]).resolve().as_posix()
text = sdkconfig.read_text()
lines = text.splitlines()
seen_custom = False
seen_filename = False
out = []
for line in lines:
    if line.startswith("CONFIG_PARTITION_TABLE_CUSTOM_FILENAME="):
        out.append(f'CONFIG_PARTITION_TABLE_CUSTOM_FILENAME="{partition_path}"')
        seen_custom = True
    elif line.startswith("CONFIG_PARTITION_TABLE_FILENAME="):
        out.append(f'CONFIG_PARTITION_TABLE_FILENAME="{partition_path}"')
        seen_filename = True
    else:
        out.append(line)
if not seen_custom:
    out.append(f'CONFIG_PARTITION_TABLE_CUSTOM_FILENAME="{partition_path}"')
if not seen_filename:
    out.append(f'CONFIG_PARTITION_TABLE_FILENAME="{partition_path}"')
sdkconfig.write_text("\n".join(out).rstrip() + "\n")
print(f"Partition path current: {partition_path}")
PY

if [[ -f "$cargo_config" ]]; then
  if grep -Fq 'ESP_IDF_TOOLS_INSTALL_DIR = "custom:C:/e"' "$cargo_config"; then
    perl -0pi -e 's/ESP_IDF_TOOLS_INSTALL_DIR = "custom:C:\/e"/ESP_IDF_TOOLS_INSTALL_DIR = "global"/g' "$cargo_config"
    echo "Updated ESP_IDF_TOOLS_INSTALL_DIR for macOS/global ESP-IDF tooling"
  fi
fi
