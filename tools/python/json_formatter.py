"""JSON formatter — validate & pretty-print (stdlib only)."""

from __future__ import annotations

import json


def format_json(text: str, indent: int = 2, sort_keys: bool = False) -> str:
    """Parse lalu cetak ulang cantik. Raise ValueError bila bukan JSON valid."""
    try:
        data = json.loads(text)
    except json.JSONDecodeError as exc:
        raise ValueError(f"invalid JSON: {exc}") from exc
    return json.dumps(data, indent=indent, sort_keys=sort_keys, ensure_ascii=False)


def validate_json(text: str) -> bool:
    try:
        json.loads(text)
        return True
    except json.JSONDecodeError:
        return False


def run() -> None:
    """Interactive formatter. Empty line returns to main menu."""
    print("\n-- JSON Formatter (paste JSON, empty line to go back) --")
    buf: list[str] = []
    print("Enter JSON (end with an empty line):")
    while True:
        try:
            line = input()
        except (EOFError, KeyboardInterrupt):
            print()
            return
        if not line.strip():
            break
        buf.append(line)
    if not buf:
        return
    try:
        print(format_json("\n".join(buf)))
    except ValueError as exc:
        print(f"Error: {exc}")


def main(argv: list[str] | None = None) -> int:
    """CLI: ``python3 json_formatter.py '{"b":1}'`` atau via stdin."""
    import sys

    args = sys.argv[1:] if argv is None else argv
    text = " ".join(args) if args else sys.stdin.read()
    if not text.strip():
        run()
        return 0
    try:
        print(format_json(text))
        return 0
    except ValueError as exc:
        print(f"Error: {exc}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
