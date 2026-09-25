"""RPL Toolkit - main entry point (thin: lifecycle milik core.runtime)."""

from core.runtime import Runtime


def main() -> int:
    return Runtime().run()


if __name__ == "__main__":
    raise SystemExit(main())
