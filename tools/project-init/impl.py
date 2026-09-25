"""Project scaffolder — minimal Python/JS project layout (stdlib only)."""

from __future__ import annotations

from pathlib import Path

LAYOUTS = ("python", "node")


def scaffold(name: str, kind: str, dest: str = ".") -> list[str]:
    """Buat struktur proyek minimal. Return daftar file dibuat.

    Raise ValueError bila kind tak dikenal / nama kosong / target ada.
    """
    if not name or not name.strip():
        raise ValueError("project name must not be empty")
    if kind not in LAYOUTS:
        raise ValueError(f"kind must be one of {LAYOUTS}")
    root = Path(dest) / name.strip()
    if root.exists():
        raise ValueError(f"target exists: {root}")
    files: dict[str, str] = {}
    if kind == "python":
        files = {
            "README.md": f"# {name}\n",
            "main.py": '"""Entry point."""\n\n\ndef main() -> None:\n    print("hello")\n\n\nif __name__ == "__main__":\n    main()\n',
            "tests/__init__.py": "",
            "requirements.txt": "# runtime dependencies\n",
            ".gitignore": "__pycache__/\n.venv/\n",
        }
    else:
        files = {
            "README.md": f"# {name}\n",
            "package.json": '{\n  "name": "%s",\n  "type": "module",\n  "scripts": {"start": "node src/main.js"}\n}\n' % name,
            "src/main.js": 'console.log("hello");\n',
            ".gitignore": "node_modules/\n",
        }
    created: list[str] = []
    for rel, content in files.items():
        p = root / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(content, encoding="utf-8")
        created.append(p.as_posix())
    return created


def run() -> None:
    """Interactive scaffolder. Empty line returns to main menu."""
    print("\n-- Project Init (empty to go back) --")
    try:
        name = input("name> ").strip()
    except (EOFError, KeyboardInterrupt):
        print()
        return
    if not name:
        return
    try:
        kind = input("kind [python/node]> ").strip().lower() or "python"
    except (EOFError, KeyboardInterrupt):
        print()
        return
    try:
        for f in scaffold(name, kind):
            print(f"created: {f}")
    except ValueError as exc:
        print(f"Error: {exc}")


def main(argv: list[str] | None = None) -> int:
    """CLI: ``python3 impl.py <name> [python|node] [dest]``."""
    import sys

    args = sys.argv[1:] if argv is None else argv
    if not args:
        run()
        return 0
    kind = args[1] if len(args) > 1 else "python"
    dest = args[2] if len(args) > 2 else "."
    try:
        for f in scaffold(args[0], kind, dest):
            print(f"created: {f}")
        return 0
    except ValueError as exc:
        print(f"Error: {exc}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
