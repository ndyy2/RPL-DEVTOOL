"""Compat shim — real implementation: tools/project-init/impl.py (dashes illegal in imports)."""

from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path

_spec = spec_from_file_location(
    "tools.project_init_impl",
    Path(__file__).resolve().parent.parent / "project-init" / "impl.py",
)
assert _spec is not None and _spec.loader is not None
_mod = module_from_spec(_spec)
_spec.loader.exec_module(_mod)

scaffold = _mod.scaffold
run = _mod.run
main = _mod.main
