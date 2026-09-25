"""Runtime (Python) — application lifecycle. Cerminan core/cpp Runtime."""

from __future__ import annotations

import sys
from pathlib import Path

from core.module_manager import ModuleManager

BANNER = (
    "\n+----------------------------------+\n"
    "|          RPL TOOLKIT             |\n"
    "|     Student Developer Tools      |\n"
    "+----------------------------------+"
)


def find_repo_root(start: str | Path | None = None) -> Path:
    cur = Path(start or Path.cwd()).resolve()
    for _ in range(5):
        if (
            (cur / "tools").is_dir()
            or (cur / "config").is_dir()
            or (cur / "main.py").is_file()
            or (cur / "pcc.conf.json").is_file()
        ):
            return cur
        if cur.parent == cur:
            break
        cur = cur.parent
    return Path.cwd()


class Runtime:
    def __init__(self, repo_root: str | Path | None = None) -> None:
        self.repo_root = Path(repo_root) if repo_root else find_repo_root()

    def list_tools(self) -> int:
        for t in ModuleManager(self.repo_root).discover():
            print(f"{t.name} [{t.runtime}]  {t.description}")
            print(f"    entry: {t.entry}")
        return 0

    def interactive(self) -> int:
        mm = ModuleManager(self.repo_root)
        mm.discover()
        while True:
            print(BANNER)
            for i, t in enumerate(mm.tools, 1):
                print(f"[{i}] {t.name} ({t.runtime})")
            print("[0] Exit")
            try:
                choice = input("\nSelect tool > ").strip()
            except (EOFError, KeyboardInterrupt):
                print("\nGoodbye!")
                return 0
            if choice in ("0", "q", "exit"):
                print("Goodbye!")
                return 0
            tool = None
            if choice.isdigit():
                idx = int(choice) - 1
                if 0 <= idx < len(mm.tools):
                    tool = mm.tools[idx]
            if tool is None:
                tool = mm.find(choice)
            if tool is None:
                print(f"Unknown tool: {choice}.")
                continue
            mm.run(tool)
        return 0

    def run(self, argv: list[str] | None = None) -> int:
        argv = list(sys.argv[1:] if argv is None else argv)
        if not argv:
            return self.interactive()
        if argv[0] in ("--help", "-h", "help"):
            print("RPLKit Developer Tools (Python core)\n")
            print("Usage:\n  main.py [--list] [--run <tool> [args...]] [--help]")
            return 0
        if argv[0] in ("--list", "list"):
            return self.list_tools()
        if argv[0] in ("--run", "run"):
            if len(argv) < 2:
                print("usage: main.py --run <tool> [args...]")
                return 2
            mm = ModuleManager(self.repo_root)
            mm.discover()
            tool = mm.find(argv[1])
            if tool is None:
                print(f"unknown tool: {argv[1]}")
                return 1
            return mm.run(tool, argv[2:])
        print(f"unknown command: {argv[0]}")
        return 2
