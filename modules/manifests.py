"""Manifest discovery — satu-satunya sumber daftar tool (Fase 1).

Memindai `tools/*/tool.json` tanpa mengeksekusi kode apa pun.
Layout lama (`tools/python/*.py`, flat `tools/*.py`) tetap didukung
sebagai fallback bila repo tanpa manifest.
"""

from __future__ import annotations

import json
from pathlib import Path

from . import ToolMeta


def _from_manifest(repo_root: Path, manifest: Path) -> ToolMeta | None:
    try:
        data = json.loads(manifest.read_text(encoding="utf-8"))
        name = str(data["name"]).strip()
        if not name:
            return None
    except (ValueError, OSError, KeyError):
        return None
    entries = data.get("entries", {}) if isinstance(data.get("entries"), dict) else {}
    tool_dir = manifest.parent
    if "python" in entries:
        entry = (tool_dir / str(entries["python"])).relative_to(repo_root).as_posix()
        runtime = "python"
    elif "typescript" in entries:
        entry = (tool_dir / str(entries["typescript"])).relative_to(repo_root).as_posix()
        runtime = "typescript"
    else:
        # Native-only: entry = NAMA tool (biner resolve op via manifest).
        native = data.get("native", {})
        op = native.get("op", "") if isinstance(native, dict) else ""
        if not str(op).strip():
            return None
        entry, runtime = name, "native"
    return ToolMeta(
        name=name,
        runtime=runtime,  # type: ignore[arg-type]
        entry=entry,
        description=str(data.get("description", "")).strip() or f"{runtime} tool: {name}",
        module=str(data.get("module", "")).strip() or "general",
        group=str(data.get("group", "")).strip(),
    )


def _overlay(root: Path) -> dict:
    """Overlay config/modules.json; {} bila absen/rusak."""
    try:
        data = json.loads((root / "config" / "modules.json").read_text(encoding="utf-8"))
        return data if isinstance(data, dict) else {}
    except (ValueError, OSError):
        return {}


def _descriptors(root: Path) -> dict[str, dict]:
    """modules/*.module.json per nama modul."""
    out: dict[str, dict] = {}
    for path in sorted((root / "modules").glob("*.module.json")):
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except (ValueError, OSError):
            continue
        if isinstance(data, dict) and str(data.get("name", "")).strip():
            out[str(data["name"]).strip()] = data
    return out


def is_enabled(root: Path, module: str, tool: str | None = None) -> bool:
    """Status efektif: override tool > overlay modul > default deskriptor."""
    overlay = _overlay(root)
    tools_sec = overlay.get("tools", {})
    if tool is not None and isinstance(tools_sec, dict) and tool in tools_sec:
        entry = tools_sec[tool]
        if isinstance(entry, dict) and isinstance(entry.get("enabled"), bool):
            return entry["enabled"]
    mods_sec = overlay.get("modules", {})
    if isinstance(mods_sec, dict) and module in mods_sec:
        entry = mods_sec[module]
        if isinstance(entry, dict) and isinstance(entry.get("enabled"), bool):
            return entry["enabled"]
    desc = _descriptors(root).get(module, {})
    default = desc.get("enabled_by_default", True)
    return bool(default) if isinstance(default, bool) else True


def discover(repo_root: str | Path, *, include_disabled: bool = False) -> list[ToolMeta]:
    """Tool bermanifest, terurut nama. Default hanya yang aktif (overlay)."""
    root = Path(repo_root)
    tools_dir = root / "tools"
    if not tools_dir.is_dir():
        return []
    found: list[ToolMeta] = []
    for manifest in sorted(tools_dir.glob("*/tool.json")):
        meta = _from_manifest(root, manifest)
        if meta is not None and (include_disabled or is_enabled(root, meta.module, meta.name)):
            found.append(meta)
    return found


def load_manifest(repo_root: str | Path, name: str) -> dict:
    """Baca mentah tool.json satu tool; {} bila tak ada/rusak."""
    path = Path(repo_root) / "tools" / name / "tool.json"
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
        return data if isinstance(data, dict) else {}
    except (ValueError, OSError):
        return {}
