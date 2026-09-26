"""Module Manager (Python) — discovery + dispatch antar runtime.

Sumber kebenaran: registry Rust via biner bila ada (`rust_bridge`),
else discovery manifest lokal — hasil identik di kedua jalur sehingga
`rplkit --list` sama di semua core.
Op native dijalankan MELALUI tool pemiliknya (mis. `calculator`),
sementara runtime `native` langsung (lawan bicara biner) ditangani
`modules.native_runtime` bila biner tersedia.
"""

from __future__ import annotations

from pathlib import Path

from modules import ToolMeta
from modules import manifests, python_runtime, rust_bridge, typescript_runtime


class ModuleManager:
    def __init__(self, repo_root: str | Path) -> None:
        self.repo_root = Path(repo_root)
        self._tools: list[ToolMeta] = []

    def discover(self) -> list[ToolMeta]:
        # Sumber kebenaran: biner Rust bila ada (registry + overlay di sana),
        # else discovery manifest lokal. Hasil identik di kedua jalur.
        bridged = rust_bridge.list_via_bin(self.repo_root)
        tools = bridged if bridged is not None else manifests.discover(self.repo_root)
        # De-duplikasi: nama pertama dipertahankan.
        uniq: list[ToolMeta] = []
        seen: set[str] = set()
        for t in tools:
            if t.name not in seen:
                seen.add(t.name)
                uniq.append(t)
        self._tools = uniq
        return uniq

    @property
    def tools(self) -> list[ToolMeta]:
        return self._tools

    def find(self, name: str) -> ToolMeta | None:
        for t in self._tools:
            if t.name == name:
                return t
        return None

    def run(self, tool: ToolMeta, args: list[str] | None = None) -> int:
        from modules import native_runtime  # lazy: hindari import cycle

        if tool.runtime == "python" and not args:
            return python_runtime.run(tool)
        if tool.runtime == "python":
            # CLI langsung ke script: python3 <entry> [args]
            from core.system_services import run_process

            code, out = run_process([native_runtime.which_python(), tool.entry, *(args or [])])
            print(out, end="")
            return code
        if tool.runtime == "typescript":
            return typescript_runtime.run(tool, self.repo_root, args)
        if tool.runtime == "native":
            return native_runtime.run(tool, self.repo_root, args)
        print(f"unsupported runtime: {tool.runtime}")
        return 1
