"""Unit converter — temperature, length, weight (pure functions)."""


def celsius_to_fahrenheit(c: float) -> float:
    return c * 9 / 5 + 32


def fahrenheit_to_celsius(f: float) -> float:
    return (f - 32) * 5 / 9


def celsius_to_kelvin(c: float) -> float:
    return c + 273.15


_LENGTH_TO_M = {
    "mm": 0.001,
    "cm": 0.01,
    "m": 1.0,
    "km": 1000.0,
    "in": 0.0254,
    "ft": 0.3048,
    "mi": 1609.344,
}

_WEIGHT_TO_KG = {
    "g": 0.001,
    "kg": 1.0,
    "lb": 0.45359237,
    "oz": 0.028349523125,
}


def convert_length(value: float, from_unit: str, to_unit: str) -> float:
    """Convert between mm/cm/m/km/in/ft/mi."""
    fu, tu = from_unit.lower(), to_unit.lower()
    if fu not in _LENGTH_TO_M or tu not in _LENGTH_TO_M:
        raise ValueError(f"unsupported length units: {from_unit} -> {to_unit}")
    return value * _LENGTH_TO_M[fu] / _LENGTH_TO_M[tu]


def convert_weight(value: float, from_unit: str, to_unit: str) -> float:
    """Convert between g/kg/lb/oz."""
    fu, tu = from_unit.lower(), to_unit.lower()
    if fu not in _WEIGHT_TO_KG or tu not in _WEIGHT_TO_KG:
        raise ValueError(f"unsupported weight units: {from_unit} -> {to_unit}")
    return value * _WEIGHT_TO_KG[fu] / _WEIGHT_TO_KG[tu]


def convert_temperature(value: float, from_unit: str, to_unit: str) -> float:
    """Convert between C/F/K."""
    fu, tu = from_unit.upper(), to_unit.upper()
    if fu == tu:
        return value
    to_c = {"C": value, "F": fahrenheit_to_celsius(value), "K": value - 273.15}
    if fu not in to_c:
        raise ValueError(f"unsupported temperature unit: {from_unit}")
    c = to_c[fu]
    if tu == "C":
        return c
    if tu == "F":
        return celsius_to_fahrenheit(c)
    if tu == "K":
        return celsius_to_kelvin(c)
    raise ValueError(f"unsupported temperature unit: {to_unit}")


def run() -> None:
    """Interactive converter. Empty line returns to main menu."""
    print("\n-- Converter (empty to go back) --")
    print("examples: 100 C F | 5 km mi | 70 kg lb")
    while True:
        try:
            raw = input("conv> ").strip()
        except (EOFError, KeyboardInterrupt):
            print()
            return
        if not raw:
            return
        parts = raw.split()
        if len(parts) != 3:
            print("Format: <value> <from> <to>")
            continue
        val_s, fu, tu = parts
        try:
            val = float(val_s)
        except ValueError:
            print("Error: value must be a number")
            continue
        try:
            if fu.upper() in ("C", "F", "K") or tu.upper() in ("C", "F", "K"):
                print(convert_temperature(val, fu, tu))
            elif fu.lower() in _LENGTH_TO_M and tu.lower() in _LENGTH_TO_M:
                print(convert_length(val, fu, tu))
            else:
                print(convert_weight(val, fu, tu))
        except ValueError as exc:
            print(f"Error: {exc}")


def convert(value: float, from_unit: str, to_unit: str) -> float:
    """Dispatch otomatis suhu/panjang/berat berdasarkan satuan."""
    if from_unit.upper() in ("C", "F", "K") or to_unit.upper() in ("C", "F", "K"):
        return convert_temperature(value, from_unit, to_unit)
    if from_unit.lower() in _LENGTH_TO_M and to_unit.lower() in _LENGTH_TO_M:
        return convert_length(value, from_unit, to_unit)
    return convert_weight(value, from_unit, to_unit)


def main(argv: list[str] | None = None) -> int:
    """CLI: ``python3 converter.py 100 C F``."""
    import sys

    args = sys.argv[1:] if argv is None else argv
    if not args:
        run()
        return 0
    if len(args) != 3:
        print("Usage: converter.py <value> <from> <to>")
        return 2
    try:
        print(convert(float(args[0]), args[1], args[2]))
        return 0
    except ValueError as exc:
        print(f"Error: {exc}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
