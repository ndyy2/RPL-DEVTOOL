"""Tools registry — titik temu Core dan Tools via System Module.

``discover()`` murni dari manifest (`tools/*/tool.json`, tanpa import).
Op native (calc, uuid, …) diakses MELALUI tool pemiliknya
(mis. `calculator` → op `calc`), bukan sebagai nama tool tersendiri —
sama seperti registry Rust. Untuk menjalankan, pakai
``core.module_manager.ModuleManager``.
"""

from __future__ import annotations

from pathlib import Path

from modules import ToolMeta
from modules import manifests
from modules import python_runtime, typescript_runtime


def discover(repo_root: str | Path | None = None) -> list[ToolMeta]:
    from core.runtime import find_repo_root

    root = Path(repo_root) if repo_root else find_repo_root()
    # manifests.discover mengembalikan SEMUA runtime (native/python/typescript).
    return manifests.discover(root)
