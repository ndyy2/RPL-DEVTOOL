"""Python Runtime — memuat dan menjalankan tools/python/*.py.

Discovery: scan direktori (tanpa mengeksekusi), lalu import lazy hanya
modul yang diminta. Tool valid bila punya callable ``run()``.
"""

from __future__ import annotations

import importlib
import json
from pathlib import Path

from . import ToolMeta
from . import manifests

PACKAGE_DIRS = ("tools/python", "tools")  # legacy, dipakai bila tanpa manifest


def _describe(script: Path, fallback: str) -> str:
    manifest = script.with_suffix(".json")
    if manifest.is_file():
        try:
            data = json.loads(manifest.read_text(encoding="utf-8"))
            desc = str(data.get("description", "")).strip()
            if desc:
                return desc
        except (ValueError, OSError):
            pass
    try:
        for line in script.read_text(encoding="utf-8").splitlines()[:4]:
            s = line.strip().strip('"').strip("#").strip()
            if s and not s.startswith("from ") and not s.startswith("import "):
                return s
    except OSError:
        pass
    return fallback


def discover(repo_root: str | Path) -> list[ToolMeta]:
    """Temukan tool Python: manifest dulu (`tools/*/tool.json`), lalu legacy."""
    root = Path(repo_root)
    found = [t for t in manifests.discover(root) if t.runtime == "python"]
    if found:
        return found
    return _discover_legacy(root)


def _discover_legacy(repo_root: Path) -> list[ToolMeta]:
    """Layout lama tanpa manifest (dihapus Fase 7)."""
    root = repo_root
    found: list[ToolMeta] = []
    seen: set[str] = set()
    for pkg in PACKAGE_DIRS:
        d = root / pkg
        if not d.is_dir():
            continue
        for script in sorted(d.glob("*.py")):
            if script.name == "__init__.py":
                continue
            name = script.stem
            if name in seen:
                continue
            seen.add(name)
            rel = script.relative_to(root).as_posix()
            found.append(
                ToolMeta(
                    name=name,
                    runtime="python",
                    entry=rel,
                    description=_describe(script, f"Python tool: {name}"),
                )
            )
    return found


def load(meta: ToolMeta):
    """Import modul tool; raise ImportError/AttributeError bila tak valid."""
    dotted = meta.entry.replace("/", ".").removesuffix(".py")
    module = importlib.import_module(dotted)
    run = getattr(module, "run")
    if not callable(run):
        raise AttributeError(f"tool {meta.name!r} has no callable run()")
    return module


def run(meta: ToolMeta) -> int:
    """Jalankan tool interaktif in-process. Return 0 sukses, 1 gagal."""
    try:
        load(meta).run()
        return 0
    except KeyboardInterrupt:
        print()
        return 0
    except Exception as exc:  # tool tidak boleh men-crash-kan core
        print(f"error in tool {meta.name!r}: {exc}")
        return 1
