#!/usr/bin/env python3
from pathlib import Path
import sys

if len(sys.argv) != 2:
    print("usage: validate_monitor_current.py <monitor.log>", file=sys.stderr)
    raise SystemExit(2)

text = Path(sys.argv[1]).read_text(errors="replace")
required_boot = [
    "radio-r36: foundation=ENABLED",
    "audio-mp3-r35-r2: stop_serialization=DECODE_THREAD_OWNS_I2S_STOP",
    "audio-progress-r34-r4-r1: reset_recursion=FIXED",
    "v0.1.31-r9 Manual Video State Machine Hard Stop Repair",
]
missing = [item for item in required_boot if item not in text]
if missing:
    print("monitor validation failed; missing:", file=sys.stderr)
    for item in missing:
        print("  -", item, file=sys.stderr)
    raise SystemExit(1)

for bad in ["Guru Meditation Error", "assert failed: spinlock_acquire", "TG1WDT_SYS_RST", "SyntaxError", "IndentationError"]:
    if bad in text:
        print(f"monitor validation failed; forbidden: {bad}", file=sys.stderr)
        raise SystemExit(1)

print("Monitor current validation: OK")
