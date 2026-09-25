"""System Module — Module Interface (kontrak antar Core dan Tools).

Sesuai README: Modules provide runtime/integration, Tools provide
functionality. Setiap tool WAJIB memenuhi kontrak ini:

* modul Python dengan fungsi ``run() -> None`` (mode interaktif), atau
* script TypeScript yang bisa dieksekusi via node, atau
* builtin native yang didaftarkan di ``NATIVE_BUILTINS``.

Manifest JSON opsional berdampingan dengan tool
(``tools/python/<nama>.json``) berisi ``{name, description, runtime}``
dan dibaca oleh core C++/Rust tanpa mengeksekusi kode apa pun.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Callable, Literal

RuntimeName = Literal["native", "python", "typescript"]


@dataclass(frozen=True)
class ToolMeta:
    """Satu entri registry System Module."""

    name: str
    runtime: RuntimeName
    # native: builtin id; python/typescript: path script relatif repo root
    entry: str
    description: str = ""
    # Callable hanya untuk tool Python yang sudah di-import (in-process).
    handler: Callable[[], None] | None = None


# Builtin native — SELALU tersedia, cerminan dari
# core/cpp ModuleManager::native_builtins() dan
# core/rust rplkit_core::native_builtins().
NATIVE_BUILTINS: tuple[ToolMeta, ...] = (
    ToolMeta("calc", "native", "calc", "Safe arithmetic evaluator (native core)"),
    ToolMeta("b64enc", "native", "b64enc", "Base64 encode (native core)"),
    ToolMeta("b64dec", "native", "b64dec", "Base64 decode (native core)"),
    ToolMeta("hexenc", "native", "hexenc", "Hex encode (native core)"),
    ToolMeta("hexdec", "native", "hexdec", "Hex decode (native core)"),
    ToolMeta("uuid", "native", "uuid", "Random UUID v4 (native core)"),
    ToolMeta("timestamp", "native", "timestamp", "Unix + ISO-8601 UTC time (native core)"),
    ToolMeta("file-info", "native", "file-info", "File size / line count (native core)"),
)
