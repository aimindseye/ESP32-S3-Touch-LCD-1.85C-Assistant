#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="v1.0.1-r13"
ASSET="dist/ESP32-S3-Touch-LCD-1.85C-Assistant-${VERSION}-clean-source.zip"
NOTES="docs/RELEASE_v1.0.1.md"

if [ ! -f "$ASSET" ]; then
  echo "missing release asset: $ASSET" >&2
  echo "run ./scripts/package_release.sh first" >&2
  exit 1
fi

gh release create "$VERSION" "$ASSET" \
  --title "v1.0.1-r13 — Internet Radio Stable + Main UI Cleanup" \
  --notes-file "$NOTES"
