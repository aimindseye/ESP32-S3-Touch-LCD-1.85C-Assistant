#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="v1.0.1-r13"
OUT_DIR="dist"
OUT="$OUT_DIR/ESP32-S3-Touch-LCD-1.85C-Assistant-${VERSION}-clean-source.zip"

./scripts/validate_assistant_current.sh .

mkdir -p "$OUT_DIR"
rm -f "$OUT"

python3 - "$OUT" <<'PY_PACKAGE'
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
import sys

root = Path.cwd()
out = Path(sys.argv[1]).resolve()

exclude_dirs = {".git", ".cleanup", "target", ".embuild", "build", "dist", "node_modules", "__pycache__", "sdcard", "tmp"}
exclude_suffixes = (".pyc", ".pyo", ".DS_Store", ".log", ".zip", ".bak", ".orig", ".tmp")

files = []
for path in root.rglob("*"):
    if not path.is_file():
        continue
    rel = path.relative_to(root)
    parts = rel.parts
    if any(part in exclude_dirs or part.startswith("_archive_") for part in parts):
        continue
    if path.name.endswith(exclude_suffixes):
        continue
    files.append(rel.as_posix())

with ZipFile(out, "w", ZIP_DEFLATED) as z:
    for rel in sorted(files):
        z.write(root / rel, rel)

with ZipFile(out, "r") as z:
    names = z.namelist()

bad = [n for n in names if "/_archive_" in f"/{n}" or "/tmp/" in f"/{n}" or n.startswith("dist/") or n.endswith(".log") or n.endswith(".zip")]
if bad:
    for n in bad[:40]:
        print(f"forbidden package entry: {n}", file=sys.stderr)
    raise SystemExit(1)

print(out)
print(f"files packaged: {len(names)}")
PY_PACKAGE

ls -lh "$OUT"

# RAW-V1-0-1-R14-CLEAN-PACKAGE-SCRIPT
