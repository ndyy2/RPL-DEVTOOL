"""Encoder/decoder — base64, hex, url, rot13 (pure functions, stdlib only)."""

from __future__ import annotations

import base64
import binascii
import codecs
import urllib.parse


def b64_encode(text: str) -> str:
    return base64.b64encode(text.encode("utf-8")).decode("ascii")


def b64_decode(text: str) -> str:
    try:
        return base64.b64decode(text.strip(), validate=True).decode("utf-8")
    except (binascii.Error, ValueError, UnicodeDecodeError) as exc:
        raise ValueError(f"invalid base64: {exc}") from exc


def hex_encode(text: str) -> str:
    return text.encode("utf-8").hex()


def hex_decode(text: str) -> str:
    try:
        return bytes.fromhex(text.strip()).decode("utf-8")
    except (ValueError, UnicodeDecodeError) as exc:
        raise ValueError(f"invalid hex: {exc}") from exc


def url_encode(text: str) -> str:
    return urllib.parse.quote(text, safe="")


def url_decode(text: str) -> str:
    return urllib.parse.unquote(text)


def rot13(text: str) -> str:
    return codecs.encode(text, "rot_13")


def run() -> None:
    """Interactive encoder. Empty line returns to main menu."""
    print("\n-- Encoder/Decoder (empty to go back) --")
    print("commands: b64enc b64dec hexenc hexdec urlenc urldec rot13")
    while True:
        try:
            raw = input("enc> ").strip()
        except (EOFError, KeyboardInterrupt):
            print()
            return
        if not raw:
            return
        parts = raw.split(maxsplit=1)
        if len(parts) != 2:
            print("Format: <command> <text>")
            continue
        cmd, text = parts
        fns = {
            "b64enc": b64_encode,
            "b64dec": b64_decode,
            "hexenc": hex_encode,
            "hexdec": hex_decode,
            "urlenc": url_encode,
            "urldec": url_decode,
            "rot13": rot13,
        }
        fn = fns.get(cmd.lower())
        if fn is None:
            print(f"Unknown command: {cmd}")
            continue
        try:
            print(fn(text))
        except ValueError as exc:
            print(f"Error: {exc}")


def main(argv: list[str] | None = None) -> int:
    """CLI: ``python3 encoder.py b64enc "hello"``."""
    import sys

    args = sys.argv[1:] if argv is None else argv
    if not args:
        run()
        return 0
    if len(args) < 2:
        print("Usage: encoder.py <b64enc|b64dec|hexenc|hexdec|urlenc|urldec|rot13> <text>")
        return 2
    fns = {
        "b64enc": b64_encode,
        "b64dec": b64_decode,
        "hexenc": hex_encode,
        "hexdec": hex_decode,
        "urlenc": url_encode,
        "urldec": url_decode,
        "rot13": rot13,
    }
    fn = fns.get(args[0].lower())
    if fn is None:
        print(f"Unknown command: {args[0]}")
        return 2
    try:
        print(fn(" ".join(args[1:])))
        return 0
    except ValueError as exc:
        print(f"Error: {exc}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
