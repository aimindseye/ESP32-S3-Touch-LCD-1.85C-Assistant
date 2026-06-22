# macOS Development

## Root cause fixed in v0.1.15-r1

The default macOS `cargo build` used the host/default Rust toolchain, so the Xtensa target failed with:

```text
can't find crate for `std`
the `xtensa-esp32s3-espidf` target may not be installed
```

The fix is to force the ESP Rust toolchain:

```bash
cargo +esp build --release
cargo +esp espflash flash --release --monitor --port /dev/cu.usbmodemXXXX
```

The repo now includes:

```text
rust-toolchain.toml
firmware/assistant-rs/rust-toolchain.toml
```

Both pin `channel = "esp"`.

## Apply clean deliverable

Use `rsync --delete` so stale files and LVGL lab leftovers are removed while preserving `.git`:

```bash
cd ~/projects

unzip ~/Downloads/ESP32-S3-Touch-LCD-1.85C-Assistant-v0.1.15-r1-mac-esp-toolchain-fix-files.zip -d /tmp/v015r1

rsync -av --delete --exclude '.git/' \
  /tmp/v015r1/ESP32-S3-Touch-LCD-1.85C-Assistant-v0.1.15-r1-mac-esp-toolchain-fix-files/ \
  ~/projects/ESP32-S3-Touch-LCD-1.85C-Assistant/
```

## Validate

```bash
cd ~/projects/ESP32-S3-Touch-LCD-1.85C-Assistant
./scripts/setup_mac_env.sh
./scripts/fix_assistant_partition_path.sh
./scripts/validate_rust_assistant_repo.sh
```

## Build

```bash
./scripts/build_assistant_rs.sh --clean
```

## Flash

```bash
./scripts/flash_assistant_rs.sh --port /dev/cu.usbmodemXXXX
```
