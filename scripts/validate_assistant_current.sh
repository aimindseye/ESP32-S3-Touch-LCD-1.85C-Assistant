#!/usr/bin/env bash
set -euo pipefail

ROOT="${1:-.}"
SRC="$ROOT/firmware/assistant-rs/src"
COMP="$ROOT/firmware/assistant-rs/components"

python3 - "$ROOT" "$SRC" "$COMP" <<'PY'
from pathlib import Path
import re
import sys

root = Path(sys.argv[1])
src = Path(sys.argv[2])
comp = Path(sys.argv[3])

def fail(msg):
    print(f"CURRENT VALIDATION FAILED: {msg}", file=sys.stderr)
    raise SystemExit(1)

main = src / "main.rs"
boot = src / "boot_report.rs"
pages = src / "app" / "pages.rs"
actions = src / "app" / "actions.rs"
model = src / "app" / "model.rs"
ffi = src / "ffi.rs"
audio = src / "audio_foundation.rs"
radio = src / "internet_radio.rs"
media = src / "media_controls.rs"

screens_mod = src / "screens" / "mod.rs"
home = src / "screens" / "home.rs"
weather = src / "screens" / "weather.rs"
music = src / "screens" / "music.rs"
assistant = src / "screens" / "assistant.rs"
settings = src / "screens" / "settings.rs"
radio_inventory = src / "screens" / "internet_radio.rs"

required = [
    main, boot, pages, actions, model, ffi, audio, radio, media,
    screens_mod, home, weather, music, assistant, settings, radio_inventory,
]
for p in required:
    if not p.exists():
        fail(f"missing required file: {p.relative_to(root)}")

main_t = main.read_text(errors="replace")
boot_t = boot.read_text(errors="replace")
pages_t = pages.read_text(errors="replace")
actions_t = actions.read_text(errors="replace")
model_t = model.read_text(errors="replace")
ffi_t = ffi.read_text(errors="replace")
audio_t = audio.read_text(errors="replace")
radio_t = radio.read_text(errors="replace")
media_t = media.read_text(errors="replace")
screens_mod_t = screens_mod.read_text(errors="replace")
home_t = home.read_text(errors="replace")
weather_t = weather.read_text(errors="replace")
music_t = music.read_text(errors="replace")
assistant_t = assistant.read_text(errors="replace")
settings_t = settings.read_text(errors="replace")
radio_inventory_t = radio_inventory.read_text(errors="replace")
all_rust = "\n".join(p.read_text(errors="replace") for p in src.rglob("*.rs"))

# Page list contract.
m = re.search(r'ALL_PAGES\s*:\s*[^=]+=\s*\[(?P<body>.*?)\]\s*;', pages_t, re.S)
if not m:
    fail("ALL_PAGES declaration missing")

entries = re.findall(r'AssistantPage::([A-Za-z0-9_]+)', m.group("body"))
expected = ["Home", "Weather", "Music", "InternetRadio", "Assistant", "Settings"]
if entries != expected:
    fail(f"ALL_PAGES expected {expected}, found {entries}")

# No screen include! paths remain.
if 'include!("screens/' in main_t:
    fail("main.rs still contains a screen include! path")

if "RAW-R50-SCREEN-MODULES-NO-INCLUDES" not in main_t:
    fail("main.rs missing r50 no-screen-includes marker")

# Screen module declarations.
if "mod screens;" not in main_t:
    fail("main.rs must declare mod screens")

for mod_name in ["home", "weather", "music", "assistant", "settings", "internet_radio"]:
    if f"pub(crate) mod {mod_name};" not in screens_mod_t:
        fail(f"screens/mod.rs must expose {mod_name}")

# True screen module markers.
for name, text, marker in [
    ("home", home_t, "RAW-R45-HOME-TRUE-SCREEN-MODULE"),
    ("weather", weather_t, "RAW-R46-R1-WEATHER-TRUE-SCREEN-MODULE"),
    ("music", music_t, "RAW-R47-MUSIC-TRUE-SCREEN-MODULE"),
    ("assistant", assistant_t, "RAW-R48-ASSISTANT-TRUE-SCREEN-MODULE"),
    ("settings", settings_t, "RAW-R49-SETTINGS-TRUE-SCREEN-MODULE"),
]:
    if marker not in text:
        fail(f"{name} true module marker missing")
    if "use crate::*;" not in text:
        fail(f"{name} module missing crate import")

if "RAW-R50-SCREEN-INTERNET-RADIO-INVENTORY-MODULE" not in radio_inventory_t:
    fail("Internet Radio inventory module marker missing")

# Export guards.
if not re.search(r'pub\(crate\)\s+fn\s+draw_home', home_t):
    fail("Home draw functions are not exported")

if not re.search(r'pub\(crate\)\s+fn\s+draw_weather', weather_t):
    fail("Weather draw functions are not exported")

if not re.search(r'pub\(crate\)\s+fn\s+draw_music|pub\(crate\)\s+fn\s+draw_audio', music_t):
    fail("Music draw functions are not exported")

if not re.search(r'pub\(crate\)\s+fn\s+draw_assistant|pub\(crate\)\s+fn\s+draw_ai', assistant_t):
    fail("Assistant draw functions are not exported")

if not re.search(r'pub\(crate\)\s+fn\s+draw_settings|pub\(crate\)\s+fn\s+settings_', settings_t):
    fail("Settings draw functions are not exported")

for mod_name in ["home", "weather", "music", "assistant", "settings"]:
    if f"screens::{mod_name}::" not in main_t and f"crate::screens::{mod_name}::" not in all_rust:
        fail(f"{mod_name} callsites not routed through screens::{mod_name}")

if "RAW-R49-SETTINGS-MODULE-CALLSITE" not in main_t:
    fail("main.rs missing r49 Settings module callsite marker")

# Video removal contract.
for forbidden, name, text in [
    ("VideoToggle", "actions.rs", actions_t),
    ("AssistantPage::Video", "pages/main", pages_t + main_t),
    ("video_status::", "main.rs", main_t),
    ("mod video_status;", "main.rs", main_t),
    ("VIDEO_PLAYBACK_", "main.rs", main_t),
    ("VIDEO.RGB", "main.rs", main_t),
    ("st77916_video_worker", "ffi.rs", ffi_t),
    ("St77916Mjpeg", "ffi.rs", ffi_t),
    ("video: status=READY", "boot_report.rs", boot_t),
]:
    if forbidden in text:
        fail(f"removed Video path still present in {name}: {forbidden}")

if (src / "video_status.rs").exists():
    fail("video_status.rs should not exist")

if re.search(r'(?m)^\s*pub\s+video_[A-Za-z0-9_]+\s*:', model_t):
    fail("model.rs still exposes video_* fields")

# Boot/report contract.
if "firmware: v0.1.36-r56-r2" not in boot_t:
    fail("boot report version must be v0.1.36-r56-r2")

if "RAW-R50-NO-SCREEN-INCLUDES-BOOT" not in boot_t:
    fail("r50 boot marker missing")

if "ui: pages=Home,Weather,Music,InternetRadio,Assistant,Settings controls=DEDICATED_MEDIA_ZONES" not in boot_t:
    fail("boot page list must remain six-page route")

# Settings behavior guards.
for marker in ["SettingsEnter", "SettingsBack", "settings-nav", "detail", "overview"]:
    if marker.lower() not in settings_t.lower() and marker not in all_rust:
        fail(f"Settings semantic marker missing: {marker}")

# Weather source guards.
for marker in ["WeatherLocation", "location", "units"]:
    if marker.lower() not in weather_t.lower() and marker not in all_rust:
        fail(f"Weather semantic marker missing: {marker}")

# Music accepted behavior guards.
for marker in ["wav", "mp3", "volume", "progress"]:
    if marker not in audio_t.lower() and marker not in music_t.lower():
        fail(f"audio/music baseline marker missing: {marker}")

if "apply_volume_percent_silent" not in audio_t:
    fail("audio silent boot volume helper missing")

if not any(token in all_rust for token in ["audio-mp3-r35-r2", "audio-mp3-r35-r3"]):
    fail("accepted MP3 baseline marker missing")

for marker in ["VOL", "PREV", "NEXT"]:
    if marker not in media_t and marker not in music_t:
        fail(f"media control marker missing: {marker}")

# Assistant semantic guard.
if "AssistantPage::Assistant" not in all_rust:
    fail("Assistant page variant route missing")

if "Assistant" not in assistant_t:
    fail("Assistant screen source missing Assistant semantic marker")

# Internet Radio baseline.
if "InternetRadio" not in pages_t:
    fail("InternetRadio page missing")

if "internet_radio::" not in main_t and "internet_radio_screen" not in main_t:
    fail("main.rs no longer routes Internet Radio modules")

for marker in ["station", "volume", "radio"]:
    if marker not in radio_t.lower():
        fail(f"radio semantic marker missing: {marker}")

shim = comp / "internet_radio_stream_shim" / "internet_radio_stream_shim.c"
if not shim.exists():
    fail("internet radio stream shim missing")

shim_t = shim.read_text(errors="replace")
for marker in ["esp_http_client", "esp_crt_bundle_attach", "RADIO_INPUT_BYTES"]:
    if marker not in shim_t:
        fail(f"radio stream shim marker missing: {marker}")

# Accepted historical guards.
if "v0.1.36-r35" not in all_rust:
    fail("r35 dedicated media touch zone marker missing")

if "v0.1.36-r36" not in all_rust:
    fail("r36 Music radio-style layout marker missing")

if "to_ascii_uppercase" not in all_rust:
    fail("lowercase station/text rendering guard missing")

# Documentation marker.
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
    if not any("RAW-R50-DOCS-NO-SCREEN-INCLUDES" in p.read_text(errors="replace") for p in existing_docs):
        fail("no documentation file contains RAW-R50-DOCS-NO-SCREEN-INCLUDES")

print("Assistant current consolidated validation: OK")
print(f"ALL_PAGES file: {pages.relative_to(root)} entries={len(entries)}")
print("Home true screen module: OK")
print("Weather true screen module: OK")
print("Music true screen module: OK")
print("Assistant true screen module: OK")
print("Settings true screen module: OK")
print("InternetRadio inventory module: OK")
print("No screen include! paths: OK")
print("Music WAV/MP3 accepted baseline: OK")
print("Media touch zones: OK")
print("Video removed/dead source cleanup: OK")
PY

# RAW-R51-MAIN-ORCHESTRATION-VALIDATOR
python3 - "$ROOT" "$SRC" <<'PY_R51'
from pathlib import Path
import re
import sys

root = Path(sys.argv[1])
src = Path(sys.argv[2])

def fail(msg):
    print(f"CURRENT VALIDATION FAILED: {msg}", file=sys.stderr)
    raise SystemExit(1)

main = src / "main.rs"
orch = src / "page_orchestration.rs"
boot = src / "boot_report.rs"

for p in [main, orch, boot]:
    if not p.exists():
        fail(f"missing r51 required file: {p.relative_to(root)}")

main_t = main.read_text(errors="replace")
orch_t = orch.read_text(errors="replace")
boot_t = boot.read_text(errors="replace")

if "mod page_orchestration;" not in main_t:
    fail("main.rs must declare page_orchestration module")

if "RAW-R51-MAIN-ORCHESTRATION-CALLSITES" not in main_t:
    fail("main.rs missing r51 orchestration callsite marker")

if "RAW-R51-MAIN-ORCHESTRATION-CLEANUP" not in orch_t:
    fail("page_orchestration.rs missing r51 marker")

if "use crate::*;" not in orch_t:
    fail("page_orchestration.rs must import crate root helpers")

if not re.search(r'(?m)^pub\(crate\)\s+fn\s+[A-Za-z0-9_]+\s*\(', orch_t):
    fail("page_orchestration.rs has no exported orchestration functions")

if not any(token in orch_t for token in ["AssistantPage::", "NextPage", "PreviousPage", "nav:", "settings-nav"]):
    fail("page_orchestration.rs does not contain page dispatch/navigation semantics")

if "firmware: v0.1.36-r56-r2" not in boot_t:
    fail("boot report version must be v0.1.36-r56-r2")

if "RAW-R51-MAIN-ORCHESTRATION-CLEANUP-BOOT" not in boot_t:
    fail("boot report missing r51 marker")

if 'include!("screens/' in main_t:
    fail("main.rs must remain free of screen include! paths after r51")

print("Main orchestration cleanup: OK")
PY_R51


# RAW-R51-R1-ORCHESTRATION-COMPILE-REPAIR-VALIDATOR
python3 - "$ROOT" "$SRC" <<'PY_R51_R1'
from pathlib import Path
import sys

root = Path(sys.argv[1])
src = Path(sys.argv[2])

def fail(msg):
    print(f"CURRENT VALIDATION FAILED: {msg}", file=sys.stderr)
    raise SystemExit(1)

main = src / "main.rs"
orch = src / "page_orchestration.rs"
assistant = src / "screens" / "assistant.rs"
boot = src / "boot_report.rs"

main_t = main.read_text(errors="replace")
orch_t = orch.read_text(errors="replace")
assistant_t = assistant.read_text(errors="replace")
boot_t = boot.read_text(errors="replace")

if "RAW-R51-R1-PAGE-ORCHESTRATION-AFTER-DEBUG-MACRO" not in main_t:
    fail("main.rs missing r51-r1 macro-scope repair marker")

if "crate::page_assets::draw_cached_page_base(" not in assistant_t:
    fail("assistant screen must call page_assets draw_cached_page_base")

if "firmware: v0.1.36-r56-r2" not in boot_t:
    fail("boot report version must be v0.1.36-r56-r2")

print("Main orchestration compile repair: OK")
PY_R51_R1


# RAW-R52-TOUCH-HANDLER-VALIDATOR
python3 - "$ROOT" "$SRC" <<'PY_R52'
from pathlib import Path
import re
import sys

root = Path(sys.argv[1])
src = Path(sys.argv[2])

def fail(msg):
    print(f"CURRENT VALIDATION FAILED: {msg}", file=sys.stderr)
    raise SystemExit(1)

main = src / "main.rs"
orch = src / "page_orchestration.rs"
touch = src / "touch_router.rs"
boot = src / "boot_report.rs"

for p in [main, orch, touch, boot]:
    if not p.exists():
        fail(f"missing r52 required file: {p.relative_to(root)}")

main_t = main.read_text(errors="replace")
orch_t = orch.read_text(errors="replace")
touch_t = touch.read_text(errors="replace")
boot_t = boot.read_text(errors="replace")

if "mod touch_router;" not in main_t:
    fail("main.rs must declare touch_router module")

if "mod page_orchestration;" not in main_t:
    fail("main.rs must keep page_orchestration module")

if "RAW-R52-TOUCH-ROUTER-AFTER-DEBUG-MACRO" not in main_t:
    fail("main.rs missing r52 macro-scope marker")

if "RAW-R52-TOUCH-HANDLER-CLEANUP" not in touch_t:
    fail("touch_router.rs missing r52 marker")

if "use crate::*;" not in touch_t:
    fail("touch_router.rs must import crate root helpers")

if not re.search(r'(?m)^pub\(crate\)\s+fn\s+[A-Za-z0-9_]+\s*\(', touch_t):
    fail("touch_router.rs has no exported touch/action functions")

touch_semantics = ["touch-class", "gesture", "swipe-left", "swipe-right", "swipe-down", "tap"]
action_semantics = ["SettingsEnter", "SettingsBack", "SettingsToggle", "AudioPlay", "AudioStop", "RadioPlay", "RadioStop", "NextPage", "PreviousPage", "action:"]

if not any(token in touch_t for token in touch_semantics):
    fail("touch_router.rs does not contain touch classification semantics")

if not any(token in touch_t for token in action_semantics):
    fail("touch_router.rs does not contain action routing semantics")

if "touch_router::" not in main_t and "touch_router::" not in orch_t:
    fail("touch_router callsites are not routed from main/page_orchestration")

if "firmware: v0.1.36-r56-r2" not in boot_t:
    fail("boot report version must be v0.1.36-r56-r2")

if "RAW-R52-TOUCH-HANDLER-CLEANUP-BOOT" not in boot_t:
    fail("boot report missing r52 marker")

if 'include!("screens/' in main_t:
    fail("main.rs must remain free of screen include! paths after r52")

print("Touch handler cleanup: OK")
PY_R52


# RAW-R52-R1-TOUCH-ROUTER-CALLSITE-VALIDATOR
python3 - "$ROOT" "$SRC" <<'PY_R52_R1'
from pathlib import Path
import sys

root = Path(sys.argv[1])
src = Path(sys.argv[2])

def fail(msg):
    print(f"CURRENT VALIDATION FAILED: {msg}", file=sys.stderr)
    raise SystemExit(1)

main = src / "main.rs"
touch = src / "touch_router.rs"
settings = src / "screens" / "settings.rs"
settings_router = src / "settings_action_router.rs"
boot = src / "boot_report.rs"

main_t = main.read_text(errors="replace")
touch_t = touch.read_text(errors="replace")
settings_t = settings.read_text(errors="replace")
settings_router_t = settings_router.read_text(errors="replace")
boot_t = boot.read_text(errors="replace")

if "pub(crate) fn process_touch_summary" not in touch_t:
    fail("process_touch_summary must be exported from touch_router")

if "pub(crate) fn is_settings_detail_header_tap" not in settings_router_t:
    fail("is_settings_detail_header_tap must be exported from settings_action_router")

if "touch_router::process_touch_summary(" not in main_t:
    fail("main.rs must route process_touch_summary through touch_router")

if "page_orchestration::process_touch_summary(" in main_t:
    fail("main.rs still routes process_touch_summary through page_orchestration")

if "crate::settings_action_router::is_settings_detail_header_tap(" not in settings_t:
    fail("settings screen must route header tap helper through settings_action_router")

if "firmware: v0.1.36-r56-r2" not in boot_t:
    fail("boot report version must be v0.1.36-r56-r2")

if "RAW-R52-R1-TOUCH-ROUTER-CALLSITE-REPAIR-BOOT" not in boot_t:
    fail("boot report missing r52-r1 marker")

print("Touch router callsite repair: OK")
PY_R52_R1


# RAW-R53-PAGE-ASSETS-VALIDATOR
python3 - "$ROOT" "$SRC" <<'PY_R53'
from pathlib import Path
import re
import sys

root = Path(sys.argv[1])
src = Path(sys.argv[2])

def fail(msg):
    print(f"CURRENT VALIDATION FAILED: {msg}", file=sys.stderr)
    raise SystemExit(1)

main = src / "main.rs"
boot = src / "boot_report.rs"
orch = src / "page_orchestration.rs"
touch = src / "touch_router.rs"
assets = src / "page_assets.rs"
assistant = src / "screens" / "assistant.rs"

for p in [main, boot, orch, touch, assets, assistant]:
    if not p.exists():
        fail(f"missing r53 required file: {p.relative_to(root)}")

main_t = main.read_text(errors="replace")
boot_t = boot.read_text(errors="replace")
orch_t = orch.read_text(errors="replace")
touch_t = touch.read_text(errors="replace")
assets_t = assets.read_text(errors="replace")
assistant_t = assistant.read_text(errors="replace")

if "mod page_assets;" not in main_t:
    fail("main.rs must declare page_assets module")

if "RAW-R53-PAGE-ASSETS-MODULE-AFTER-DEBUG-MACRO" not in main_t:
    fail("main.rs missing r53 page_assets module marker")

if "RAW-R53-PAGE-ASSETS-CLEANUP" not in assets_t:
    fail("page_assets.rs missing r53 marker")

if "use crate::*;" not in assets_t:
    fail("page_assets.rs must import crate root helpers")

if "pub(crate) fn draw_cached_page_base" not in assets_t:
    fail("draw_cached_page_base must be exported from page_assets")

if re.search(r'(?m)^(?:pub\(crate\)\s+)?fn\s+draw_cached_page_base\s*\(', orch_t):
    fail("draw_cached_page_base must not remain defined in page_orchestration")

if "page_orchestration::draw_cached_page_base(" in main_t + orch_t + touch_t + assistant_t:
    fail("draw_cached_page_base still routes through page_orchestration")

if "crate::page_assets::draw_cached_page_base(" not in main_t + orch_t + touch_t + assistant_t:
    fail("draw_cached_page_base callsites are not routed through page_assets")

if "firmware: v0.1.36-r56-r2" not in boot_t:
    fail("boot report version must be v0.1.36-r56-r2")

if "RAW-R53-PAGE-ASSETS-CLEANUP-BOOT" not in boot_t:
    fail("boot report missing r53 marker")

if 'include!("screens/' in main_t:
    fail("main.rs must remain free of screen include! paths after r53")

print("Page assets cleanup: OK")
PY_R53


# RAW-R53-R1-VALIDATOR-REGEX-REPAIR
python3 - "$ROOT" "$SRC" <<'PY_R53_R1'
from pathlib import Path
import re
import sys

root = Path(sys.argv[1])
src = Path(sys.argv[2])

def fail(msg):
    print(f"CURRENT VALIDATION FAILED: {msg}", file=sys.stderr)
    raise SystemExit(1)

assets = src / "page_assets.rs"
orch = src / "page_orchestration.rs"
boot = src / "boot_report.rs"

assets_t = assets.read_text(errors="replace")
orch_t = orch.read_text(errors="replace")
boot_t = boot.read_text(errors="replace")

if "pub(crate) fn draw_cached_page_base" not in assets_t:
    fail("draw_cached_page_base must be exported from page_assets")

if re.search(r'(?m)^(?:pub\(crate\)\s+)?fn\s+draw_cached_page_base\s*\(', orch_t):
    fail("draw_cached_page_base must not remain defined in page_orchestration")

if "firmware: v0.1.36-r56-r2" not in boot_t:
    fail("boot report version must be v0.1.36-r56-r2")

print("Page assets validator regex repair: OK")
PY_R53_R1


# RAW-R54-MEDIA-ACTION-VALIDATOR
python3 - "$ROOT" "$SRC" <<'PY_R54'
from pathlib import Path
import re
import sys

root = Path(sys.argv[1])
src = Path(sys.argv[2])

def fail(msg):
    print(f"CURRENT VALIDATION FAILED: {msg}", file=sys.stderr)
    raise SystemExit(1)

main = src / "main.rs"
boot = src / "boot_report.rs"
touch = src / "touch_router.rs"
media = src / "media_action_router.rs"

for p in [main, boot, touch, media]:
    if not p.exists():
        fail(f"missing r54 required file: {p.relative_to(root)}")

main_t = main.read_text(errors="replace")
boot_t = boot.read_text(errors="replace")
touch_t = touch.read_text(errors="replace")
media_t = media.read_text(errors="replace")

if "mod media_action_router;" not in main_t:
    fail("main.rs must declare media_action_router module")

if "RAW-R54-MEDIA-ACTION-ROUTER-AFTER-DEBUG-MACRO" not in main_t:
    fail("main.rs missing r54 media router module marker")

if "RAW-R54-MEDIA-ACTION-CLEANUP" not in media_t:
    fail("media_action_router.rs missing r54 marker")

if "use crate::*;" not in media_t:
    fail("media_action_router.rs must import crate root helpers")

if not re.search(r'(?m)^pub\(crate\)\s+fn\s+[A-Za-z0-9_]+\s*\(', media_t):
    fail("media_action_router.rs has no exported media helpers")

if not any(token in media_t for token in ["AudioPlay", "AudioStop", "AudioNext", "AudioVol", "audio-r35"]):
    fail("media_action_router.rs missing Music/Audio action semantics")

if not any(token in media_t for token in ["RadioPlay", "RadioStop", "RadioNext", "RadioVol", "radio-r35"]):
    fail("media_action_router.rs missing Internet Radio action semantics")

if "media_action_router::" not in touch_t + main_t:
    fail("media_action_router callsites are not routed from touch/main")

if "firmware: v0.1.36-r56-r2" not in boot_t:
    fail("boot report version must be v0.1.36-r56-r2")

if "RAW-R54-MEDIA-ACTION-CLEANUP-BOOT" not in boot_t:
    fail("boot report missing r54 marker")

if 'include!("screens/' in main_t:
    fail("main.rs must remain free of screen include! paths after r54")

print("Media action cleanup: OK")
PY_R54


# RAW-R55-SETTINGS-ACTION-VALIDATOR
python3 - "$ROOT" "$SRC" <<'PY_R55'
from pathlib import Path
import re
import sys

root = Path(sys.argv[1])
src = Path(sys.argv[2])

def fail(msg):
    print(f"CURRENT VALIDATION FAILED: {msg}", file=sys.stderr)
    raise SystemExit(1)

main = src / "main.rs"
boot = src / "boot_report.rs"
touch = src / "touch_router.rs"
settings = src / "settings_action_router.rs"
settings_screen = src / "screens" / "settings.rs"

for p in [main, boot, touch, settings, settings_screen]:
    if not p.exists():
        fail(f"missing r55 required file: {p.relative_to(root)}")

main_t = main.read_text(errors="replace")
boot_t = boot.read_text(errors="replace")
touch_t = touch.read_text(errors="replace")
settings_t = settings.read_text(errors="replace")
settings_screen_t = settings_screen.read_text(errors="replace")

if "mod settings_action_router;" not in main_t:
    fail("main.rs must declare settings_action_router module")

if "RAW-R55-SETTINGS-ACTION-ROUTER-AFTER-DEBUG-MACRO" not in main_t:
    fail("main.rs missing r55 settings router module marker")

if "RAW-R55-SETTINGS-ACTION-CLEANUP" not in settings_t:
    fail("settings_action_router.rs missing r55 marker")

if "use crate::*;" not in settings_t:
    fail("settings_action_router.rs must import crate root helpers")

if not re.search(r'(?m)^pub\(crate\)\s+fn\s+[A-Za-z0-9_]+\s*\(', settings_t):
    fail("settings_action_router.rs has no exported Settings helpers")

if not any(token in settings_t for token in ["SettingsEnter", "SettingsBack", "SettingsToggle", "settings-nav", "settings detail", "is_settings_detail_header_tap"]):
    fail("settings_action_router.rs missing Settings action/detail semantics")

if "settings_action_router::" not in touch_t + main_t + settings_screen_t:
    fail("settings_action_router callsites are not routed from touch/main/settings screen")

if "touch_router::is_settings_detail_header_tap(" in settings_screen_t:
    fail("settings screen still routes header tap helper through touch_router")

if "firmware: v0.1.36-r56-r2" not in boot_t:
    fail("boot report version must be v0.1.36-r56-r2")

if "RAW-R55-SETTINGS-ACTION-CLEANUP-BOOT" not in boot_t:
    fail("boot report missing r55 marker")

if 'include!("screens/' in main_t:
    fail("main.rs must remain free of screen include! paths after r55")

print("Settings action cleanup: OK")
PY_R55


# RAW-R56-R1-WEATHER-ACTION-VALIDATOR
python3 - "$ROOT" "$SRC" <<'PY_R56_R1'
from pathlib import Path
import sys

root = Path(sys.argv[1])
src = Path(sys.argv[2])

def fail(msg):
    print(f"CURRENT VALIDATION FAILED: {msg}", file=sys.stderr)
    raise SystemExit(1)

main = src / "main.rs"
boot = src / "boot_report.rs"
touch = src / "touch_router.rs"
router = src / "weather_action_router.rs"
actions = src / "app" / "actions.rs"
providers = src / "app" / "providers.rs"
weather = src / "app" / "weather.rs"
screen = src / "screens" / "weather.rs"

for p in [main, boot, touch, router, actions, providers, weather, screen]:
    if not p.exists():
        fail(f"missing r56-r1 required file: {p.relative_to(root)}")

main_t = main.read_text(errors="replace")
boot_t = boot.read_text(errors="replace")
touch_t = touch.read_text(errors="replace")
router_t = router.read_text(errors="replace")
actions_t = actions.read_text(errors="replace")
providers_t = providers.read_text(errors="replace")
weather_t = weather.read_text(errors="replace")
screen_t = screen.read_text(errors="replace")

if "mod weather_action_router;" not in main_t:
    fail("main.rs must declare weather_action_router")

if "RAW-R56-R1-WEATHER-ACTION-CLEANUP" not in router_t:
    fail("weather_action_router.rs missing r56-r1 marker")

if "pub(crate) fn handle_weather_select_action" not in router_t:
    fail("weather select handler must live in weather_action_router")

if "pub(crate) fn handle_weather_action" not in router_t:
    fail("weather action handler must live in weather_action_router")

if "crate::weather_action_router::handle_weather_select_action(model, providers)" not in actions_t:
    fail("app/actions.rs must route Weather page select through weather_action_router")

if (
    "crate::weather_action_router::handle_weather_action(model, action);" not in touch_t
    and "RAW-R56-R1-WEATHER-ACTION-FALLBACK-NO-TOUCH-BLOCKS" not in touch_t
):
    fail("touch_router.rs must route Weather action or record no-touch-block fallback")

if "pub fn previous_weather_location" not in providers_t:
    fail("LocalProviders must expose previous_weather_location")

if "pub fn previous_location(&mut self)" not in weather_t:
    fail("WeatherState must expose previous_location")

if 'label: "MUMBAI"' not in weather_t or 'timezone_url: "Asia%2FKolkata"' not in weather_t:
    fail("Mumbai timezone must remain Asia%2FKolkata")

if "WEATHER LOCATION" not in screen_t:
    fail("Weather screen must show weather-location label")

if "draw_weather_location_nav_buttons" not in screen_t:
    fail("Weather screen must render nav buttons")

for token in ["< LOC", "LOC >", "model.weather.units.suffix()"]:
    if token not in screen_t:
        fail(f"Weather nav button token missing: {token}")

if "firmware: v0.1.36-r56-r2" not in boot_t:
    fail("boot report version must be v0.1.36-r56-r2")

if "RAW-R56-R1-WEATHER-ACTION-CLEANUP-BOOT" not in boot_t:
    fail("boot report missing r56-r1 marker")

if 'include!("screens/' in main_t:
    fail("main.rs must remain free of screen include! paths after r56-r1")

print("Weather action cleanup fallback repair: OK")
PY_R56_R1


# RAW-R56-R2-WEATHER-NAV-ROW-LABEL-VALIDATOR
python3 - "$ROOT" "$SRC" <<'PY_R56_R2'
from pathlib import Path
import sys

root = Path(sys.argv[1])
src = Path(sys.argv[2])

def fail(msg):
    print(f"CURRENT VALIDATION FAILED: {msg}", file=sys.stderr)
    raise SystemExit(1)

boot = src / "boot_report.rs"
touch = src / "touch_router.rs"
router = src / "weather_action_router.rs"
screen = src / "screens" / "weather.rs"

for p in [boot, touch, router, screen]:
    if not p.exists():
        fail(f"missing r56-r2 required file: {p.relative_to(root)}")

boot_t = boot.read_text(errors="replace")
touch_t = touch.read_text(errors="replace")
router_t = router.read_text(errors="replace")
screen_t = screen.read_text(errors="replace")

if "firmware: v0.1.36-r56-r2" not in boot_t:
    fail("boot report version must be v0.1.36-r56-r2")

if "WEATHER LOCATION" not in screen_t:
    fail("Weather screen must say WEATHER LOCATION, not CURRENT LOCATION")

if "CURRENT LOCATION" in screen_t:
    fail("Weather screen must not say CURRENT LOCATION after r56-r2")

if "pub(crate) fn maybe_handle_weather_nav_row_touch" not in router_t:
    fail("weather nav row handler must live in weather_action_router")

if "crate::weather_action_router::maybe_handle_weather_nav_row_touch(model, providers)" not in touch_t:
    fail("touch_router must short-circuit Weather nav row taps")

if "weather-nav-r56-r2: handled" not in router_t:
    fail("weather nav row handler must log handled nav row taps")

print("Weather nav row label repair: OK")
PY_R56_R2


# RAW-R56-R2-RELEASE-DOCS-VALIDATOR-REPAIR
python3 - "$ROOT" "$SRC" <<'PY_RELEASE_DOCS'
from pathlib import Path
import sys
root = Path(sys.argv[1])
src = Path(sys.argv[2])

def fail(msg):
    print(f"CURRENT VALIDATION FAILED: {msg}", file=sys.stderr)
    raise SystemExit(1)

readme = root / "README.md"
arch = root / "architecture.md"
hw = root / "docs" / "HARDWARE.md"
rel = root / "docs" / "RELEASE_v0.1.36-r56-r2.md"
boot = src / "boot_report.rs"
validate = root / "scripts" / "validate_assistant_current.sh"

for p in [readme, arch, hw, rel, boot, validate]:
    if not p.exists():
        fail(f"missing release documentation file: {p.relative_to(root)}")

readme_t = readme.read_text(errors="replace")
arch_t = arch.read_text(errors="replace")
hw_t = hw.read_text(errors="replace")
rel_t = rel.read_text(errors="replace")
boot_t = boot.read_text(errors="replace")
validate_t = validate.read_text(errors="replace")

for token in ["v0.1.36-r56-r2", "Weather Location", "Internet Radio", "Bluetooth Classic / A2DP"]:
    if token not in readme_t:
        fail(f"README missing token: {token}")

for token in ["Hardware-driven architecture", "ESP32-S3", "BLE-only", "media_action_router.rs", "weather_action_router.rs"]:
    if token not in arch_t:
        fail(f"architecture.md missing token: {token}")

for token in ["ESP32-S3R8", "ST77916", "CST816", "PCF85063", "PCM5101", "Asia/Kolkata"]:
    if token not in hw_t:
        fail(f"HARDWARE.md missing token: {token}")

if "firmware: v0.1.36-r56-r2" not in boot_t:
    fail("boot report must remain v0.1.36-r56-r2")

if "v0.1.36-r56-r2" + "-r2" in validate_t:
    fail("validator must not contain accidental duplicate r56-r2 suffix")

if "RAW-R56-R2-RELEASE-NOTES" not in rel_t:
    fail("release notes marker missing")

print("Release docs cleanup: OK")
PY_RELEASE_DOCS
