# assistant-rs firmware crate

Active firmware crate for the Waveshare ESP32-S3-Touch-LCD-1.85C Assistant.

Build from the repository root with:

```bash
./scripts/build_assistant_rs.sh
```

Or from this directory with:

```bash
cargo build --release
```

If the repository has moved, run the root helper before building so `sdkconfig.defaults` points to the local `partitions.csv` path:

```bash
../../scripts/fix_assistant_partition_path.sh
```

Active extra components are listed in `Cargo.toml` under `package.metadata.esp-idf-sys.extra_components`.

<!-- RAW-V1-0-1-R14-CLEAN-FIRMWARE-README -->
