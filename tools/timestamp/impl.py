"""Timestamp — Unix epoch <-> ISO-8601 UTC (stdlib only)."""

from __future__ import annotations

import datetime as dt
import time


def now_unix() -> int:
    return int(time.time())


def now_iso() -> str:
    return dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def unix_to_iso(ts: float) -> str:
    return dt.datetime.fromtimestamp(ts, tz=dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def iso_to_unix(text: str) -> float:
    """Parse ISO-8601 (terima akhiran Z). Raise ValueError bila gagal."""
    s = text.strip()
    if s.endswith("Z"):
        s = s[:-1] + "+00:00"
    try:
        return dt.datetime.fromisoformat(s).timestamp()
    except ValueError as exc:
        raise ValueError(f"invalid ISO-8601: {exc}") from exc


def run() -> None:
    """Interactive converter. Empty line returns to main menu."""
    print("\n-- Timestamp (empty to go back) --")
    print(f"now: {now_unix()}  {now_iso()}")
    print("input Unix seconds -> ISO, atau ISO -> Unix")
    while True:
        try:
            raw = input("ts> ").strip()
        except (EOFError, KeyboardInterrupt):
            print()
            return
        if not raw:
            return
        try:
            print(unix_to_iso(float(raw)))
        except ValueError:
            try:
                print(int(iso_to_unix(raw)))
            except ValueError as exc:
                print(f"Error: {exc}")


def main(argv: list[str] | None = None) -> int:
    """CLI: ``python3 timestamp.py`` (now) atau ``python3 timestamp.py <ts|iso>``."""
    import sys

    args = sys.argv[1:] if argv is None else argv
    if not args:
        print(f"{now_unix()}\n{now_iso()}")
        return 0
    raw = " ".join(args)
    try:
        print(unix_to_iso(float(raw)))
        return 0
    except ValueError:
        pass
    try:
        print(int(iso_to_unix(raw)))
        return 0
    except ValueError as exc:
        print(f"Error: {exc}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
