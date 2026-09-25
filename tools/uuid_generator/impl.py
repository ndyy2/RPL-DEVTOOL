"""UUID generator — v1 & v4 (stdlib only)."""

from __future__ import annotations

import uuid


def generate(count: int = 1, version: int = 4) -> list[str]:
    """Generate `count` UUIDs (1..100). Raise ValueError bila argumen invalid."""
    if not 1 <= count <= 100:
        raise ValueError("count must be 1..100")
    if version == 4:
        return [str(uuid.uuid4()) for _ in range(count)]
    if version == 1:
        return [str(uuid.uuid1()) for _ in range(count)]
    raise ValueError("version must be 1 or 4")


def run() -> None:
    """Interactive generator. Empty line returns to main menu."""
    print("\n-- UUID Generator (empty to go back, e.g. '5' or '5 v1') --")
    while True:
        try:
            raw = input("uuid> ").strip()
        except (EOFError, KeyboardInterrupt):
            print()
            return
        if not raw:
            return
        parts = raw.split()
        try:
            count = int(parts[0]) if parts else 1
            version = 1 if len(parts) > 1 and parts[1].lower() == "v1" else 4
            for u in generate(count, version):
                print(u)
        except ValueError as exc:
            print(f"Error: {exc}")


def main(argv: list[str] | None = None) -> int:
    """CLI: ``python3 uuid_generator.py [count] [v1|v4]``."""
    import sys

    args = sys.argv[1:] if argv is None else argv
    if not args:
        run()
        return 0
    try:
        count = int(args[0]) if args else 1
        version = 1 if len(args) > 1 and args[1].lower() == "v1" else 4
        for u in generate(count, version):
            print(u)
        return 0
    except ValueError as exc:
        print(f"Error: {exc}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
