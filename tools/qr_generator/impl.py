"""QR generator — jujur tanpa dep eksternal.

Tanpa library `qrcode`, QR asli tidak bisa dibuat dari stdlib secara
wajar. Tool ini:
* bila `qrcode` terinstall → generate PNG beneran;
* bila tidak → cetak ASCII box + instruksi install.

Fungsi pure `qr_ascii_box()` selalu tersedia untuk framing teks.
"""

from __future__ import annotations


def qr_ascii_box(text: str, width: int = 40) -> str:
    """Bingkai teks dalam box ASCII (fallback yang jujur, bukan QR palsu)."""
    if width < 10:
        raise ValueError("width must be >= 10")
    inner = width - 4
    words, lines, cur = text.split(), [], ""
    for w in words:
        probe = f"{cur} {w}".strip()
        if len(probe) <= inner:
            cur = probe
        else:
            if cur:
                lines.append(cur)
            cur = w
            while len(cur) > inner:  # kata super panjang: potong keras
                lines.append(cur[:inner])
                cur = cur[inner:]
    if cur:
        lines.append(cur)
    if not lines:
        lines = [""]
    top = "+" + "-" * (width - 2) + "+"
    rows = [top] + [f"| {ln:<{inner}} |" for ln in lines] + [top]
    return "\n".join(rows)


def has_qrcode_lib() -> bool:
    try:
        import qrcode  # type: ignore  # noqa: F401

        return True
    except ImportError:
        return False


def save_qr_png(text: str, path: str) -> str:
    """Simpan QR PNG bila lib tersedia; raise RuntimeError bila tidak."""
    try:
        import qrcode  # type: ignore
    except ImportError as exc:
        raise RuntimeError("pip install qrcode[pil]  —  lib belum terinstall") from exc
    img = qrcode.make(text)
    getattr(img, "save")(path)  # getattr: hindari false-positive type-checker
    return path


def run() -> None:
    """Interactive generator. Empty line returns to main menu."""
    print("\n-- QR Generator (empty to go back) --")
    if not has_qrcode_lib():
        print("(mode ASCII-box; untuk QR asli: pip install qrcode[pil])")
    while True:
        try:
            raw = input("qr> ").strip()
        except (EOFError, KeyboardInterrupt):
            print()
            return
        if not raw:
            return
        if has_qrcode_lib():
            out = "qr.png"
            try:
                save_qr_png(raw, out)
                print(f"saved: {out}")
            except RuntimeError as exc:
                print(f"Error: {exc}")
        else:
            print(qr_ascii_box(raw))


def main(argv: list[str] | None = None) -> int:
    """CLI: ``python3 qr_generator.py "teks" [output.png]``."""
    import sys

    args = sys.argv[1:] if argv is None else argv
    if not args:
        run()
        return 0
    text = args[0]
    if len(args) > 1:
        try:
            print(f"saved: {save_qr_png(text, args[1])}")
            return 0
        except RuntimeError as exc:
            print(f"Error: {exc}")
            return 1
    if has_qrcode_lib():
        try:
            print(f"saved: {save_qr_png(text, 'qr.png')}")
            return 0
        except RuntimeError as exc:
            print(f"Error: {exc}")
            return 1
    print(qr_ascii_box(text))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
