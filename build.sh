#!/usr/bin/env bash
# build.sh — bangun seluruh RPLKit berurutan (wajib urut!):
#   1. Rust runtime dulu (menghasilkan librplkit_runtime.a yang di-link C++),
#   2. baru C++ (CMake memverifikasi keberadaan .a saat configure).
#
# Catatan: JANGAN andalkan artefak `cargo test` untuk .a — selalu jalankan
# `cargo build` eksplisit (staticlib bisa basi bila hanya `cargo test`).
#
# Usage: ./build.sh [--release]
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODE=debug
if [[ "${1:-}" == "--release" ]]; then MODE=release; fi

echo "==> [1/2] cargo build ($MODE) ..."
export PATH="$HOME/.cargo/bin:$PATH"
if [[ "$MODE" == "release" ]]; then
  cargo build --release --manifest-path "$ROOT/core/rust/Cargo.toml"
else
  cargo build --manifest-path "$ROOT/core/rust/Cargo.toml"
fi

echo "==> [2/2] cmake build ($MODE) ..."
if [[ "$MODE" == "release" ]]; then
  CMAKE_TYPE=Release
else
  CMAKE_TYPE=Debug
fi
cmake -S "$ROOT/core/cpp" -B "$ROOT/build/cpp" -DCMAKE_BUILD_TYPE="$CMAKE_TYPE"
cmake --build "$ROOT/build/cpp" -j"$(nproc)"

echo "==> OK: $ROOT/build/cpp/rplkit + $ROOT/core/rust/target/$MODE/rplkit"
