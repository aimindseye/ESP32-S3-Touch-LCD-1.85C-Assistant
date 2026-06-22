#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="v1.0.0"
ASSET="dist/ESP32-S3-Touch-LCD-1.85C-Assistant-${VERSION}-source.zip"
NOTES="docs/RELEASE_v1.0.0.md"

if [ ! -f "$ASSET" ]; then
  echo "missing release asset: $ASSET" >&2
  echo "run ./scripts/package_release.sh first" >&2
  exit 1
fi

if [ ! -f "$NOTES" ]; then
  echo "missing release notes: $NOTES" >&2
  exit 1
fi

gh release create "$VERSION" "$ASSET"   --title "v1.0.0 Stable Release"   --notes-file "$NOTES"
