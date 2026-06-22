#!/usr/bin/env bash
set -euo pipefail

echo "Checking macOS ESP Rust prerequisites..."

missing=0
for cmd in rustup cargo rustc python3 shasum; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Missing command: $cmd"
    missing=1
  else
    echo "OK: $cmd -> $(command -v "$cmd")"
  fi
done

if ! cargo +esp --version >/dev/null 2>&1; then
  echo "ESP Rust toolchain is not usable through cargo +esp."
  echo "Install/update with espup, then source the generated export file."
  echo "Typical commands:"
  echo "  cargo install espup"
  echo "  espup install"
  echo "  source ~/export-esp.sh"
  missing=1
else
  echo "OK: cargo +esp -> $(cargo +esp --version)"
fi

if ! rustc +esp -Vv >/dev/null 2>&1; then
  echo "rustc +esp is not usable."
  missing=1
else
  echo "OK: rustc +esp"
  rustc +esp -Vv | sed 's/^/  /'
fi

if ! cargo +esp espflash --help >/dev/null 2>&1; then
  echo "cargo-espflash is not available to cargo +esp."
  echo "Install with: cargo install espflash"
  missing=1
else
  echo "OK: cargo +esp espflash available"
fi

if [[ "$missing" == "1" ]]; then
  echo "macOS setup check found missing prerequisites."
  exit 1
fi

echo "macOS setup check: OK"
