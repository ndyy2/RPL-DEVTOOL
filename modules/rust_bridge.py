"""Rust bridge — Python dev-path memakai biner Rust bila ada (F3 source of truth).

Bila `core/rust/target/{release,debug}/rplkit` dapat dieksekusi, discovery
datang dari `rplkit --list` (satu sumber: registry Rust). Bila tidak ada
(mis. belum build), pemanggil jatuh ke discovery lokal. Tak pernah melempar.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

from . import ToolMeta

_CANDIDATES = (
    "core/rust/target/release/rplkit",
    "core/rust/target/debug/rplkit",
)


def find_bin(repo_root: str | Path) -> Path | None:
    """Biner Rust pertama yang executable; None bila tak ada."""
    root = Path(repo_root)
    for cand in _CANDIDATES:
        p = root / cand
        try:
            if p.is_file():
                import os

                if os.access(p, os.X_OK):
                    return p
        except OSError:
            continue
    return None


def _parse_list(text: str, repo_root: Path) -> list[ToolMeta]:
    """Parse output `rplkit --list`: baris `nama [runtime]  deskripsi`
    diikuti `    entry: ...`. Entry absolut di bawah repo dikembalikan
    relatif (konvensi ToolMeta). Baris tak dikenal dilewati diam-diam.
    """
    found: list[ToolMeta] = []
    name = runtime = desc = entry = ""
    for line in text.splitlines():
        if line.startswith("    entry: "):
            entry = line[len("    entry: ") :].strip()
            try:
                rel = Path(entry).relative_to(repo_root).as_posix()
            except ValueError:
                rel = entry
            if name:
                rt = runtime if runtime in ("native", "python", "typescript") else "python"
                found.append(
                    ToolMeta(
                        name=name,
                        runtime=rt,  # type: ignore[arg-type]
                        entry=rel,
                        description=desc,
                    )
                )
            name = runtime = desc = entry = ""
        elif line and not line[0].isspace() and "[" in line and "]" in line:
            head, _, tail = line.partition("[")
            rt, _, ds = tail.partition("]")
            name, runtime, desc = head.strip(), rt.strip(), ds.strip()
    return found


def list_via_bin(repo_root: str | Path, timeout: float = 10.0) -> list[ToolMeta] | None:
    """Discovery via biner Rust; None bila biner absen/gagal (fallback lokal)."""
    root = Path(repo_root)
    exe = find_bin(root)
    if exe is None:
        return None
    try:
        proc = subprocess.run(
            [str(exe), "--list"], capture_output=True, text=True, timeout=timeout, check=False
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if proc.returncode != 0:
        return None
    return _parse_list(proc.stdout, root)
