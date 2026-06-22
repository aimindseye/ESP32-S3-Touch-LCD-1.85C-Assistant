#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

VERSION="v1.0.0"
OUTDIR="dist"
OUT="$OUTDIR/ESP32-S3-Touch-LCD-1.85C-Assistant-${VERSION}-source.zip"
mkdir -p "$OUTDIR"
rm -f "$OUT"

python3 - <<'PY'
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED

root = Path.cwd()
version = "v1.0.0"
out = root / "dist" / f"ESP32-S3-Touch-LCD-1.85C-Assistant-{version}-source.zip"

exclude_dirs = {
    ".git", ".cleanup", "target", ".embuild", "build", "dist",
    "node_modules", "__pycache__", "sdcard",
}
exclude_prefixes = ("_archive_", ".v0.1.")
exclude_suffixes = (".pyc", ".pyo", ".DS_Store", ".bak", ".orig", ".tmp")
exclude_name_contains = (".bak.", ".pre-", ".v0_1_", ".v0_")

def keep(path: Path) -> bool:
    rel = path.relative_to(root)
    for part in rel.parts:
        if part in exclude_dirs:
            return False
        if part.startswith(exclude_prefixes):
            return False
    name = path.name
    if name.endswith(exclude_suffixes):
        return False
    if any(token in name for token in exclude_name_contains):
        return False
    if rel.as_posix().startswith("docs/history/"):
        return False
    return path.is_file()

with ZipFile(out, "w", ZIP_DEFLATED) as z:
    for path in sorted(root.rglob("*")):
        if keep(path):
            z.write(path, path.relative_to(root))

print(out)
PY

ls -lh "$OUT"

# RAW-V1-0-0-PACKAGE-SCRIPT
