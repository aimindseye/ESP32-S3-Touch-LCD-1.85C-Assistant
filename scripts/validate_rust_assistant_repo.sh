#!/usr/bin/env bash
set -euo pipefail

ROOT="${1:-.}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

if [[ "$ROOT" == "." ]]; then
  ROOT="$REPO_ROOT"
fi

exec "$SCRIPT_DIR/validate_assistant_current.sh" "$ROOT"
