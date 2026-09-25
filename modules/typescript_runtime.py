"""TypeScript Runtime — menjalankan tools/typescript/*.ts via node.

Sengaja subprocess (bukan in-process): isolasi runtime sesuai filosofi
README "Keep the tools replaceable". node >= 22 men-strip tipe bawaan;
node lama akan gagal dengan pesan jelas, bukan crash.
"""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

from . import ToolMeta
from . import manifests


def discover(repo_root: str | Path) -> list[ToolMeta]:
    root = Path(repo_root)
    found = [t for t in manifests.discover(root) if t.runtime == "typescript"]
    if found:
        return found
    return _discover_legacy(root)


def _discover_legacy(repo_root: Path) -> list[ToolMeta]:
    root = repo_root
    d = root / "tools" / "typescript"
    if not d.is_dir():
        return []
    found: list[ToolMeta] = []
    for script in sorted(d.glob("*.ts")):
        if script.name.endswith(".d.ts"):
            continue
        rel = script.relative_to(root).as_posix()
        found.append(
            ToolMeta(
                name=script.stem,
                runtime="typescript",
                entry=rel,
                description=f"TypeScript tool: {script.stem}",
            )
        )
    return found


def run(meta: ToolMeta, repo_root: str | Path, args: list[str] | None = None) -> int:
    node = shutil.which("node")
    if node is None:
        print("error: 'node' not found in PATH — install Node.js 22+ to run TypeScript tools")
        return 1
    script = Path(repo_root) / meta.entry
    try:
        proc = subprocess.run([node, str(script), *(args or [])], check=False)
        return proc.returncode
    except OSError as exc:
        print(f"error running {meta.name!r}: {exc}")
        return 1
