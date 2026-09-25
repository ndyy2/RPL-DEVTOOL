"""Tools registry — titik temu Core dan Tools via System Module.

``discover()`` menggabungkan native builtins + Python + TypeScript tanpa
meng-import tool apa pun (aman). Untuk menjalankan, pakai
``core.module_manager.ModuleManager``.
"""

from __future__ import annotations

from pathlib import Path

from modules import ToolMeta
from modules import python_runtime, typescript_runtime
from modules import NATIVE_BUILTINS


def discover(repo_root: str | Path | None = None) -> list[ToolMeta]:
    from core.runtime import find_repo_root

    root = Path(repo_root) if repo_root else find_repo_root()
    tools: list[ToolMeta] = list(NATIVE_BUILTINS)
    tools.extend(python_runtime.discover(root))
    tools.extend(typescript_runtime.discover(root))
    uniq: list[ToolMeta] = []
    seen: set[str] = set()
    for t in tools:
        if t.name not in seen:
            seen.add(t.name)
            uniq.append(t)
    return uniq
