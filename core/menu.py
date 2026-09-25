"""CLI menu for RPL Toolkit.

Thin view layer: seluruh lifecycle dipegang ``core.runtime.Runtime``
(agar identik dengan core C++/Rust). Modul ini dipertahankan demi
kompatibilitas ``from core.menu import run_menu``.
"""

from __future__ import annotations

from core.runtime import Runtime


def run_menu() -> None:
    """Run the main interactive menu (blocking, return saat user keluar)."""
    Runtime().interactive()
