"""File utilities — size, hash, listing (stdlib only, read-only default)."""

from __future__ import annotations

import hashlib
from pathlib import Path

_CHUNK = 65536


def file_size(path: str) -> int:
    """Ukuran file dalam bytes. Raise FileNotFoundError bila tak ada."""
    p = Path(path)
    if not p.is_file():
        raise FileNotFoundError(f"no such file: {path}")
    return p.stat().st_size


def sha256(path: str) -> str:
    """Hex SHA-256 isi file (streaming, aman untuk file besar)."""
    p = Path(path)
    if not p.is_file():
        raise FileNotFoundError(f"no such file: {path}")
    h = hashlib.sha256()
    with p.open("rb") as f:
        while chunk := f.read(_CHUNK):
            h.update(chunk)
    return h.hexdigest()


def list_dir(path: str = ".") -> list[str]:
    """Daftar nama entri direktori terurut. Raise NotADirectoryError."""
    p = Path(path)
    if not p.is_dir():
        raise NotADirectoryError(f"not a directory: {path}")
    return sorted(e.name for e in p.iterdir())


def human_size(n: int) -> str:
    """1024 -> '1.0 KiB'. Raise ValueError untuk negatif."""
    if n < 0:
        raise ValueError("size must be >= 0")
    units = ["B", "KiB", "MiB", "GiB", "TiB"]
    v: float = float(n)
    for u in units:
        if v < 1024 or u == units[-1]:
            return f"{v:.1f} {u}" if u != "B" else f"{int(v)} B"
        v /= 1024
    return f"{v:.1f} TiB"  # unreachable, penuhi type-checker


def run() -> None:
    """Interactive utilities. Empty line returns to main menu."""
    print("\n-- File Utilities (empty to go back) --")
    print("commands: size <p> | sha256 <p> | ls [dir]")
    while True:
        try:
            raw = input("file> ").strip()
        except (EOFError, KeyboardInterrupt):
            print()
            return
        if not raw:
            return
        parts = raw.split(maxsplit=1)
        cmd, arg = parts[0].lower(), (parts[1] if len(parts) > 1 else ".")
        try:
            if cmd == "size":
                n = file_size(arg)
                print(f"{n} bytes ({human_size(n)})")
            elif cmd == "sha256":
                print(sha256(arg))
            elif cmd == "ls":
                for name in list_dir(arg):
                    print(name)
            else:
                print(f"Unknown command: {cmd}")
        except (FileNotFoundError, NotADirectoryError, ValueError, OSError) as exc:
            print(f"Error: {exc}")


def main(argv: list[str] | None = None) -> int:
    """CLI: ``python3 file_utils.py size <p> | sha256 <p> | ls [dir]``."""
    import sys

    args = sys.argv[1:] if argv is None else argv
    if not args:
        run()
        return 0
    cmd, arg = args[0].lower(), (args[1] if len(args) > 1 else ".")
    try:
        if cmd == "size":
            n = file_size(arg)
            print(f"{n} bytes ({human_size(n)})")
        elif cmd == "sha256":
            print(sha256(arg))
        elif cmd == "ls":
            for name in list_dir(arg):
                print(name)
        else:
            print(f"Unknown command: {cmd}")
            return 2
        return 0
    except (FileNotFoundError, NotADirectoryError, ValueError, OSError) as exc:
        print(f"Error: {exc}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
