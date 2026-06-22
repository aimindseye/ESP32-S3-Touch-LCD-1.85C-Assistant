#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

VERSION="v0.1.36-r56-r2"
ZIP="dist/ESP32-S3-Touch-LCD-1.85C-Assistant-${VERSION}-source.zip"

./scripts/validate_assistant_current.sh
./scripts/package_release.sh

git diff --check

if ! command -v gh >/dev/null 2>&1; then
  echo "gh CLI is required to create the GitHub release" >&2
  exit 1
fi

git tag -a "$VERSION" -m "$VERSION Weather Action Cleanup and Repo Documentation Release" 2>/dev/null || true
git push origin "$VERSION"

gh release create "$VERSION" "$ZIP" \
  --title "$VERSION Weather Action Cleanup" \
  --notes-file docs/RELEASE_v0.1.36-r56-r2.md
