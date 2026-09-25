"""System Services (Python) — infrastruktur untuk Core: config, logging, fs.

Cerminan dari core/cpp system_services.* agar perilaku konsisten
antar core. Semua fungsi stdlib-only dan tidak pernah melempar
untuk operasi non-kritis (kembalikan default).
"""

from __future__ import annotations

import datetime as _dt
import json
import subprocess
import sys
from pathlib import Path


def log(level: str, message: str) -> None:
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    print(f"[{ts}] [{level}] {message}", file=sys.stderr)


def load_config(repo_root: str | Path) -> dict:
    """Baca config/default.json; {} bila tak ada/rusak."""
    cfg = Path(repo_root) / "config" / "default.json"
    try:
        return dict(json.loads(cfg.read_text(encoding="utf-8")))
    except (OSError, ValueError):
        return {}


def file_info(path: str | Path) -> dict:
    p = Path(path)
    if not p.is_file():
        return {"exists": False, "size": -1, "lines": -1}
    try:
        data = p.read_bytes()
    except OSError:
        return {"exists": False, "size": -1, "lines": -1}
    return {"exists": True, "size": len(data), "lines": data.count(b"\n")}


def run_process(cmd: list[str]) -> tuple[int, str]:
    """Jalankan proses, kembalikan (exit_code, output gabungan)."""
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
        return proc.returncode, (proc.stdout or "") + (proc.stderr or "")
    except OSError as exc:
        return 127, str(exc)
