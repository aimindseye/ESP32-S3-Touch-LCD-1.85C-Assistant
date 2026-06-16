# Apply v0.1.2 — Rust Assistant App Shell

From repo root:

```powershell
cd C:\projects\ESP32-S3-Touch-LCD-1.85C-Assistant

Expand-Archive -Force .\ESP32-S3-Touch-LCD-1.85C-Assistant-v0.1.2-files.zip .\_v012
Copy-Item -Recurse -Force .\_v012\ESP32-S3-Touch-LCD-1.85C-Assistant-v0.1.2-files\* .

.\scripts\fix_assistant_partition_path.ps1
.\scripts\validate_rust_assistant_repo.ps1
.\scripts\build_assistant_rs.ps1
.\scripts\flash_assistant_rs.ps1 -Port COM8
```

Commit after hardware smoke test:

```powershell
git status
git add .
git commit -m "v0.1.2 add Rust assistant app shell"
git tag -a v0.1.2 -m "v0.1.2 Rust Assistant App Shell"
git push origin main
git push origin v0.1.2
```
