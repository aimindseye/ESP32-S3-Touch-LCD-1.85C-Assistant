# Release Checklist

Current accepted firmware: `v0.1.36-r56-r2`

```bash
cd ~/projects/ESP32-S3-Touch-LCD-1.85C-Assistant
./scripts/validate_assistant_current.sh
cd firmware/assistant-rs
cargo build --release
cargo espflash flash --release --monitor --port /dev/cu.usbmodem2101
```

Physical smoke test:

```text
Home -> Weather -> Music -> InternetRadio -> Assistant -> Settings
Weather body tap cycle/fetch/cache
Weather nav row previous/next
Music MP3 play/stop/next/play/stop
Internet Radio direct HTTP play/stop
Internet Radio HTTPS M3U resolve/play/stop
Settings detail enter/cycle/header-back
No Video page
No reboot
```

Package:

```bash
cd ~/projects/ESP32-S3-Touch-LCD-1.85C-Assistant
./scripts/package_release.sh
```
