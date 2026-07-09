#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
firmware_dir="$repo_root/firmware/assistant-rs"

port=""
monitor=1

while [[ $# -gt 0 ]]; do
  case "$1" in
    --port|-p)
      port="${2:-}"
      shift 2
      ;;
    --no-monitor)
      monitor=0
      shift
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

if [[ -z "$port" ]]; then
  for candidate in /dev/cu.usbmodem* /dev/cu.usbserial*; do
    if [[ -e "$candidate" ]]; then
      port="$candidate"
      break
    fi
  done
fi

if [[ -z "$port" ]]; then
  echo "No serial port found. Re-run with --port /dev/cu.usbmodemXXXX" >&2
  exit 1
fi

"$script_dir/fix_assistant_partition_path.sh"
"$script_dir/validate_assistant_current.sh" "$repo_root"

cd "$firmware_dir"
if [[ "$monitor" == "1" ]]; then
  cargo espflash flash --release --monitor --port "$port"
else
  cargo espflash flash --release --port "$port"
fi
