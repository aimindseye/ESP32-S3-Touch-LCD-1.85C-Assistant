#!/usr/bin/env bash
set -euo pipefail

ROOT="${1:-.}"
SRC="$ROOT/firmware/assistant-rs/src"
COMP="$ROOT/firmware/assistant-rs/components"

fail() {
  echo "CURRENT VALIDATION FAILED: $*" >&2
  exit 1
}

python3 - "$ROOT" "$SRC" "$COMP" <<'PY'
from pathlib import Path
import re
import sys

root = Path(sys.argv[1])
src = Path(sys.argv[2])
comp = Path(sys.argv[3])

def fail(msg: str) -> None:
    print(f"CURRENT VALIDATION FAILED: {msg}", file=sys.stderr)
    raise SystemExit(1)

required_files = [
    src / "main.rs",
    src / "ffi.rs",
    src / "boot_report.rs",
    src / "app" / "pages.rs",
    src / "app" / "actions.rs",
    src / "app" / "model.rs",
    src / "audio_foundation.rs",
    src / "internet_radio.rs",
]
for p in required_files:
    if not p.exists():
        fail(f"missing required source file: {p.relative_to(root)}")

texts = {str(p.relative_to(root)): p.read_text(errors="replace") for p in required_files}
pages_text = (src / "app" / "pages.rs").read_text(errors="replace")
main_text = (src / "main.rs").read_text(errors="replace")
ffi_text = (src / "ffi.rs").read_text(errors="replace")
actions_text = (src / "app" / "actions.rs").read_text(errors="replace")
model_text = (src / "app" / "model.rs").read_text(errors="replace")
boot_text = (src / "boot_report.rs").read_text(errors="replace")
audio_text = (src / "audio_foundation.rs").read_text(errors="replace")
radio_text = (src / "internet_radio.rs").read_text(errors="replace")

all_rust = "\n".join(p.read_text(errors="replace") for p in src.rglob("*.rs"))

# ------------------------------------------------------------------
# Current page list contract.
# ------------------------------------------------------------------
m = re.search(r'ALL_PAGES\s*:\s*[^=]+=\s*\[(?P<body>.*?)\]\s*;', pages_text, re.S)
if not m:
    fail("ALL_PAGES declaration missing")

entries = re.findall(r'AssistantPage::([A-Za-z0-9_]+)', m.group("body"))
expected = ["Home", "Weather", "Music", "InternetRadio", "Assistant", "Settings"]
if entries != expected:
    fail(f"ALL_PAGES expected {expected}, found {entries}")

if "AssistantPage::Video" in pages_text:
    fail("AssistantPage::Video must not exist after r42/r43 cleanup")

# ------------------------------------------------------------------
# Video removal contract. This checks active source paths, not docs/history.
# ------------------------------------------------------------------
for forbidden, source_name, source_text in [
    ("VideoToggle", "actions.rs", actions_text),
    ("AssistantPage::Video", "main.rs", main_text),
    ("video_status::", "main.rs", main_text),
    ("mod video_status;", "main.rs", main_text),
    ("draw_video_player_tile", "main.rs", main_text),
    ("draw_video_removed_tile", "main.rs", main_text),
    ("render_video_", "main.rs", main_text),
    ("video_r4_", "main.rs", main_text),
    ("video_r5_", "main.rs", main_text),
    ("video_r6_", "main.rs", main_text),
    ("video_r9_", "main.rs", main_text),
    ("VIDEO_PLAYBACK_", "main.rs", main_text),
    ("VIDEO.RGB", "main.rs", main_text),
    ("st77916_video_worker", "ffi.rs", ffi_text),
    ("St77916Mjpeg", "ffi.rs", ffi_text),
    ("st77916_probe_sd_mjpeg", "ffi.rs", ffi_text),
    ("st77916_decode_mjpeg", "ffi.rs", ffi_text),
    ("video: status=READY", "boot_report.rs", boot_text),
]:
    if forbidden in source_text:
        fail(f"removed Video path still present in {source_name}: {forbidden}")

if (src / "video_status.rs").exists():
    fail("video_status.rs should not exist in active source after r42/r43 cleanup")

if re.search(r'(?m)^\s*pub\s+video_[A-Za-z0-9_]+\s*:', model_text):
    fail("model.rs still exposes public video_* state fields")

# ------------------------------------------------------------------
# Boot/report contract.
# ------------------------------------------------------------------
if "firmware: v0.1.36-r43" not in boot_text:
    fail("boot report version must be v0.1.36-r43")

if "RAW-R43-SOURCE-VALIDATOR-STABILIZATION" not in boot_text:
    fail("missing r43 boot/source stabilization marker")

if "ui: pages=Home,Weather,Music,InternetRadio,Assistant,Settings controls=DEDICATED_MEDIA_ZONES" not in boot_text:
    fail("boot report must advertise the current six-page route")

if "Assistant,Video,Settings" in boot_text or "video=REMOVED" in boot_text:
    fail("boot report should not advertise Video or a compatibility Video page")

# ------------------------------------------------------------------
# Music baseline contract. Keep this semantic and stable.
# ------------------------------------------------------------------
audio_lower = audio_text.lower()
for marker in ["wav", "mp3", "volume", "progress"]:
    if marker not in audio_lower:
        fail(f"audio baseline missing semantic marker: {marker}")

if "apply_volume_percent_silent" not in audio_text:
    fail("audio silent boot volume apply helper missing")

if not any(token in all_rust for token in ["audio-mp3-r35-r2", "audio-mp3-r35-r3"]):
    fail("accepted MP3 baseline markers missing")

# ------------------------------------------------------------------
# Internet Radio baseline contract.
# ------------------------------------------------------------------
if "InternetRadio" not in pages_text:
    fail("InternetRadio page missing from pages.rs")

if "internet_radio::" not in main_text and "internet_radio_screen" not in main_text:
    fail("main.rs no longer routes to Internet Radio modules")

for marker in ["station", "volume", "radio"]:
    if marker not in radio_text.lower():
        fail(f"internet radio semantic marker missing: {marker}")

shim = comp / "internet_radio_stream_shim" / "internet_radio_stream_shim.c"
if not shim.exists():
    fail("internet radio stream shim missing")

shim_text = shim.read_text(errors="replace")
for marker in ["esp_http_client", "esp_crt_bundle_attach", "RADIO_INPUT_BYTES"]:
    if marker not in shim_text:
        fail(f"internet radio stream shim missing marker: {marker}")

# ------------------------------------------------------------------
# Shared media controls / accepted layout markers.
# ------------------------------------------------------------------
media_controls = src / "media_controls.rs"
if not media_controls.exists():
    fail("media_controls.rs missing")

media_text = media_controls.read_text(errors="replace")
for marker in ["VOL", "PREV", "NEXT"]:
    if marker not in media_text:
        fail(f"media control zone marker missing: {marker}")

if "v0.1.36-r35" not in all_rust:
    fail("r35 dedicated media touch zone marker missing")

if "v0.1.36-r36" not in all_rust:
    fail("r36 Music radio-style layout marker missing")

# Preserve accepted lowercase station/text rendering behavior semantically.
if "to_ascii_uppercase" not in all_rust:
    fail("lowercase-to-uppercase glyph/text behavior missing")

# ------------------------------------------------------------------
# Documentation stabilization marker. At least one top-level doc should reflect r43.
# ------------------------------------------------------------------
doc_candidates = [
    root / "README.md",
    root / "architecture.md",
    root / "ARCHITECTURE.md",
    root / "docs" / "README.md",
    root / "docs" / "architecture.md",
    root / "docs" / "ARCHITECTURE.md",
]
existing_docs = [p for p in doc_candidates if p.exists()]
if existing_docs:
    if not any("RAW-R43-DOCS-CURRENT-PAGE-LIST" in p.read_text(errors="replace") for p in existing_docs):
        fail("no documentation file contains RAW-R43-DOCS-CURRENT-PAGE-LIST")

print("Assistant current consolidated validation: OK")
print(f"ALL_PAGES file: {(src / 'app' / 'pages.rs').relative_to(root)} entries={len(entries)}")
print("InternetRadio: OK")
print("Music WAV/MP3 accepted baseline: OK")
print("Media touch zones: OK")
print("Video removed/dead source cleanup: OK")
print("Source/validator stabilization: OK")
PY
