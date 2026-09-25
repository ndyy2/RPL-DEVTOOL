"""Calculator tool — safe arithmetic evaluation (stdlib only)."""

import ast
import operator

_ALLOWED_BINOPS = {
    ast.Add: operator.add,
    ast.Sub: operator.sub,
    ast.Mult: operator.mul,
    ast.Div: operator.truediv,
    ast.FloorDiv: operator.floordiv,
    ast.Mod: operator.mod,
    ast.Pow: operator.pow,
}

_ALLOWED_UNARYOPS = {
    ast.UAdd: operator.pos,
    ast.USub: operator.neg,
}


def _eval_node(node: ast.AST):
    if isinstance(node, ast.Expression):
        return _eval_node(node.body)
    if isinstance(node, ast.Constant):
        if isinstance(node.value, (int, float)):
            return node.value
        raise ValueError(f"unsupported constant: {node.value!r}")
    if isinstance(node, ast.BinOp):
        op = _ALLOWED_BINOPS.get(type(node.op))
        if op is None:
            raise ValueError(f"unsupported operator: {ast.dump(node.op)}")
        return op(_eval_node(node.left), _eval_node(node.right))
    if isinstance(node, ast.UnaryOp):
        op = _ALLOWED_UNARYOPS.get(type(node.op))
        if op is None:
            raise ValueError(f"unsupported unary operator: {ast.dump(node.op)}")
        return op(_eval_node(node.operand))
    raise ValueError(f"unsupported expression: {ast.dump(node)}")


def calculate(expression: str):
    """Evaluate a basic arithmetic expression safely.

    Supports + - * / // % ** and parentheses. Raises ValueError on
    empty input, syntax error, or disallowed constructs.
    """
    if not expression or not expression.strip():
        raise ValueError("empty expression")
    try:
        tree = ast.parse(expression.strip(), mode="eval")
    except SyntaxError as exc:
        raise ValueError(f"invalid expression: {exc}") from exc
    try:
        return _eval_node(tree)
    except ZeroDivisionError:
        raise
    except ValueError:
        raise
    except Exception as exc:  # pragma: no cover - defensive
        raise ValueError(f"invalid expression: {exc}") from exc


def run() -> None:
    """Interactive REPL. Empty line returns to main menu."""
    print("\n-- Calculator (empty to go back) --")
    while True:
        try:
            expr = input("calc> ").strip()
        except (EOFError, KeyboardInterrupt):
            print()
            return
        if not expr:
            return
        try:
            print(calculate(expr))
        except ZeroDivisionError:
            print("Error: division by zero")
        except ValueError as exc:
            print(f"Error: {exc}")


def main(argv: list[str] | None = None) -> int:
    """CLI sekali-jalan untuk core native: ``python3 calculator.py "2+2"``."""
    import sys

    args = sys.argv[1:] if argv is None else argv
    if not args:
        run()
        return 0
    try:
        print(calculate(" ".join(args)))
        return 0
    except ZeroDivisionError:
        print("Error: division by zero")
        return 1
    except ValueError as exc:
        print(f"Error: {exc}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
