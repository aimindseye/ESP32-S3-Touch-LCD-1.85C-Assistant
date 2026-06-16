# Apply v0.1.2 — Rust Assistant App Shell

From repo root:

```powershell
cd C:\projects\ESP32-S3-Touch-LCD-1.85C-Assistant

Expand-Archive -Force .\ESP32-S3-Touch-LCD-1.85C-Assistant-v0.1.2-r1-files.zip .\_v012r1
Copy-Item -Recurse -Force .\_v012r1\ESP32-S3-Touch-LCD-1.85C-Assistant-v0.1.2-r1-files\* .

.\scripts\normalize_assistant_timestamps.ps1
.\scripts\fix_assistant_partition_path.ps1
.\scripts\validate_rust_assistant_repo.ps1
.\scripts\build_assistant_rs.ps1 -Clean
.\scripts\flash_assistant_rs.ps1 -Port COM8
```

The timestamp normalization step prevents Ninja/CMake from looping with:

```text
ninja: error: manifest 'build.ninja' still dirty after 100 tries, perhaps system time is not set
```

This can happen after expanding a ZIP whose stored timestamps are ahead of local Windows time.
