"""Native Runtime — dispatch ke biner C++/Rust yang sudah di-build.

Urutan: build/cpp/rplkit (C++) dulu, lalu core/rust/target/debug/rplkit.
Bila belum ada yang di-build, cetak instruksi build — bukan traceback.
"""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

from . import ToolMeta

CANDIDATES = ("build/cpp/rplkit", "core/rust/target/debug/rplkit", "core/rust/target/release/rplkit")


def binary(repo_root: str | Path) -> Path | None:
    root = Path(repo_root)
    for cand in CANDIDATES:
        p = root / cand
        if p.is_file():
            return p
    return None


def run(meta: ToolMeta, repo_root: str | Path, args: list[str] | None = None) -> int:
    exe = binary(repo_root)
    if exe is None:
        print("native core belum di-build. Jalankan salah satu:")
        print("  cmake -S core/cpp -B build/cpp && cmake --build build/cpp")
        print("  cargo build --manifest-path core/rust/Cargo.toml")
        return 1
    cmd = [str(exe), "--run", meta.entry, *(args or [])]
    # Interaktif bila tanpa arg: teruskan stdin/stdout ke user.
    if not args and meta.entry in ("calc", "b64enc", "b64dec", "hexenc", "hexdec", "file-info"):
        extra = input("argument (kosong = batal) > ").strip()
        if not extra:
            return 0
        cmd.append(extra)
    try:
        proc = subprocess.run(cmd, check=False)
        return proc.returncode
    except OSError as exc:
        print(f"error menjalankan native tool: {exc}")
        return 1


def which_python() -> str:
    return shutil.which("python3") or "python3"
