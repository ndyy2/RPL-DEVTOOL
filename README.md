# RPLKit Developer Tools

**Upgraded by Ndyndy — 12 RPL-1**

> [🇮🇩 Bahasa Indonesia](README.id.md) | **English** (you are here)

<p align="center">
  <img src="assets/architecture.svg" alt="RPLKit animated architecture: native C++/Rust core, System Module, and replaceable tools" width="900" />
</p>

> A modern, modular developer toolkit with a native C++ and Rust core.

RPLKit Developer Tools is an upgraded version of the original **RPLKit**, redesigned to provide a faster, more extensible, and modular foundation for developer utilities.

The core is **natively implemented in C++ and Rust** (not aspirational — it builds and runs today), with a **System Module** that keeps individual tools flexible and easy to develop.

---

## Overview

RPLKit was originally built as a collection of developer-oriented utilities using Python.

This upgraded version focuses on three major improvements:

* Native **C++ and Rust core** (workspace: `rplkit-core` + `rplkit-runtime` + `rplkit-tui`)
* Modular **System Module architecture** (`modules/` manifest-driven registry)
* Support for writing tools using **Python or TypeScript** (**18 tools**, **5 modules**)

The goal is to separate the application's core system from individual tools, allowing new functionality to be added without modifying the entire application.

```text
                         RPLKit
                            │
                    ┌───────┴───────┐
                    │   Core System │
                    │  C++ / Rust   │
                    └───────┬───────┘
                            │
                     System Module
                            │
              ┌─────────────┼─────────────┐
              │             │             │
           Python       TypeScript      Native
            Tools          Tools         Modules
```

---

## Architecture

<p align="center">
  <img src="assets/architecture-chart.svg" alt="RPLKit architecture chart: entry points, Rust/C++ runtime, TUI, tools and config data flow" width="1200" />
</p>

### Core

The core runtime is built with:

* **C++** — CLI, menu, and a thin FFI layer over `librplkit_runtime.a`
* **Rust** — safe primitives (`rplkit-core`), execution engine (`rplkit-runtime`), interactive workspace (`rplkit-tui`)
* **Python (dev-runner)** — `main.py` + `core/` mirror the native lifecycle so `rplkit --list` is identical across all three cores

The native core manages the application lifecycle, module registry, execution chain, and system-level operations.

---

## System Module

The **System Module** is the main architectural improvement in the upgraded RPLKit.

Instead of implementing every tool inside the core application, tools are treated as independent modules discovered from manifests:

```text
RPLKit
│
├── Core
│   ├── Runtime
│   ├── Module Manager
│   └── System Services
│
├── System Module
│   ├── Python Runtime
│   ├── TypeScript Runtime (+ Native Runtime)
│   └── Module Interface (ToolMeta)
│
└── Tools (18 via tools/*/tool.json)
    ├── Python Tools (9)
    ├── TypeScript Tools (1: hello)
    └── Native Tools (8 ops via Rust core)
```

This allows developers to create new tools without tightly coupling them to the core.

---

## Multi-Language Tools

### Python

Python is used for rapid development and scripting.

```text
RPLKit Core
    │
    └── Python Module
            │
            └── tool.py
```

### TypeScript

TypeScript is used for tools that benefit from the JavaScript ecosystem and type safety.

```text
RPLKit Core
    │
    └── TypeScript Module
            │
            └── tool.ts
```

This approach keeps the core native while letting developers choose the most appropriate language per tool.

---

## Why C++ + Rust?

The upgrade does not replace Python for the sake of performance alone.

The native core provides a stronger foundation for:

* System-level operations
* Process management
* Resource control
* Native integrations
* Runtime management
* Modular execution
* Long-running developer utilities

Rust is used where memory safety and reliability matter, while C++ provides mature native-system capabilities and ecosystem compatibility. Python remains a first-class tool language (embed-first via PyO3, stdlib-only).

---

## Project Goals

RPLKit aims to become a small but extensible developer platform rather than a static collection of scripts.

### Current Direction

* [x] Replace the original Python-based core
* [x] Introduce C++ / Rust architecture (builds via `./build.sh`)
* [x] Design modular system
* [x] Introduce System Module
* [x] Support Python-based tools (9 tools, stdlib-only, embed-first via PyO3)
* [x] Support TypeScript-based tools (`tools/hello/impl.ts`, swc + QuickJS embed)
* [x] Expand module API (`modules/` ToolMeta interface + `tool.json` v2 manifests)
* [x] Native-first execution chain (native → embed → system fallback + `executed_by`)
* [x] Add module discovery (`tools/*/tool.json`, identical across 3 cores, 18/18 parity)
* [ ] Improve module isolation (sandboxing embed)
* [ ] Improve runtime management (e.g. persistent Node worker, health checks)
* [ ] Expand developer utilities (forms, custom UI per tool)

---

## Design Philosophy

> **Keep the core stable. Keep the tools replaceable.**

The core should provide the infrastructure.

Modules should provide the capabilities.

Tools should remain independent.

```text
Core
 │
 ├── provides infrastructure
 │
 ▼
Modules
 │
 ├── provide runtime / integration
 │
 ▼
Tools
 │
 └── provide actual functionality
```

---

## Project Structure

```text
rplkit/
│
├── core/
│   ├── cpp/          # CLI + menu + thin FFI to librplkit_runtime.a
│   ├── __init__.py, runtime.py, module_manager.py, menu.py, system_services.py
│   └── rust/         # workspace: rplkit-core + rplkit-runtime + rplkit-tui
│       ├── rplkit-core/     # safe primitives: calc, base64, uuid, time, sys
│       ├── rplkit-runtime/  # registry, exec_native/py/js, chain, history, FFI
│       └── rplkit-tui/      # ratatui workspace (Overview/Tools/Exec/Modules/Logs/Help)
│
├── modules/          # System Module (Python dev-runner, no build needed)
│   ├── __init__.py       # ToolMeta + NATIVE_BUILTINS
│   ├── manifests.py      # scan tools/*/tool.json (sole source of truth)
│   ├── python_runtime.py
│   ├── typescript_runtime.py
│   └── native_runtime.py
│
├── tools/<name>/     # 18 dynamic per-tool folders
│   ├── tool.json     # manifest v2: exec chain + entries
│   ├── impl.py       # Python entry (embed-first via PyO3)
│   └── impl.ts       # TypeScript entry (swc + QuickJS)
│
├── modules/*.module.json  # 5 module descriptors (dev/git/network/school/system)
├── config/default.json    # core binary paths + runtimes
├── config/modules.json    # enable/disable overlay (tooling never writes tool.json)
├── docs/tool-contract.md  # tool v2 contract
├── docs/architecture-report.md  # generated architecture report
├── docs/roadmap.md    # approved-but-unexecuted backlog planning
├── build.sh           # cargo build → cmake build (order matters)
└── README.md / README.id.md
```

### Execution chain

Each tool declares a chain in `tool.json`, e.g.
`"exec": ["native", "py-embed", "py-sys"]`. The runner tries them in order —
the system interpreter (`python3`/`node`) only runs if the native/embed chain
fails. `--verbose` shows the actual `executed_by`.

### Build & test

```sh
./build.sh            # debug: cargo + cmake
./build.sh --release  # release
python3 -m unittest discover -s tests   # Python: 18 tests, stdlib only (no pytest)
cargo test --manifest-path core/rust/Cargo.toml  # Rust: 9 + 55 + 21 = 85 tests
```

### Two modes: CLI + TUI

```text
             RPLKit
               │
       ┌───────┴────────┐
       │                │
     CLI              TUI
       │                │
 scripting          interactive
 automation         developer UX
```

No arguments + tty → terminal workspace (`ratatui`, monochrome + single
Blue Iris accent): Overview (system + recent), Tools (search `/`),
Exec (timing + `executed_by` + copy), Modules (toggle `Space`),
palette (`Ctrl+K`), Logs, Help. No tty (pipe/script) → help text,
never hangs.

```sh
rplkit                              # TUI
rplkit system-info                   # run directly (scripting)
rplkit module list                   # modules + status
rplkit module disable school-tools   # overlay config/modules.json
```

Keybindings: `1/2/3` Tools·Modules·Logs, `?` Help, `Enter` run/detail,
`Space` enable/disable, `Ctrl+R`/`Ctrl+Y` (Exec) run-again/copy,
`Esc` back, `q` quit.

---

## What's New

Recent landed work (see `git log`):

* **Embedded-first runner + dynamic modules + TUI workspace** — PyO3 embed-first Python, swc + QuickJS TypeScript path, `tool.json` v2 manifests, `executed_by` reporting.
* **Input gating, registry optimize, index, bridge, caches** — hardened TUI input handling, faster registry/discovery, cross-core parity work.
* **Native C++/Rust core + System Module** — lifecycle, module manager, system services moved native; 8 original Python tools preserved and grown to **18 tools / 5 modules**.
* **Docs** — animated architecture SVGs, generated `docs/architecture-report.md`, backlog snapshot `docs/roadmap.md`.

Verified snapshot: **18 tool manifests**, **5 module descriptors**, Python **18/18 green**, Rust **85/85 green (9 core + 55 runtime + 21 TUI)**, Python `--list` parity **18 tools**.

---

## Roadmap (Planning)

Full detail: [`docs/roadmap.md`](docs/roadmap.md). Architecture deep-dive: [`docs/architecture-report.md`](docs/architecture-report.md).

| # | Plan | Status | Size |
|---|---|---|---|
| 1 | Verify remaining TUI style refinement | Verification pending | Small |
| 2 | Bilingual README (EN + ID) + refresh | **Done by this change** | Small |
| 3 | Persistent Node worker + fallback (F5) | Backlog | Medium |
| 4 | Form loader + live preview + custom UI per tool (F6) | Backlog | Large |
| 5 | TS TUI (Ink) + daemon RPC | Approved, not executed | Largest |
| 6 | Acceptance gate F7 (watcher, docs sync, full suite) | Backlog | Medium |

Suggested order: 2 → 1 → 4 → 3 → 5 → 6.

Locked conventions across all plans: input-gating rule (input mode swallows keystrokes), `./build.sh` + `cargo test` + `python3 -m unittest`, single-accent theme (monochrome + Blue Iris `#5B5BD6`), `tool.json` never written by tooling, `config/history.json` is gitignored runtime state.

---

## Legacy → Upgraded

| Area               | Original RPLKit  | RPLKit Upgraded     |
| ------------------ | ---------------- | ------------------- |
| Core               | Python           | C++ + Rust (native, builds today) |
| Architecture       | Monolithic tools | Modular (18 manifests) |
| Tool Runtime       | Python           | Python (9) + TypeScript (1) + Native (8) |
| System Module      | —                | ✓ (manifest registry + overlay) |
| Extensibility      | Limited          | Module-based (add folder + JSON) |
| Native Integration | Limited          | Native core + FFI + embed |
| Tool Development   | Python           | Python / TypeScript |

---

## Credits

**RPLKit Developer Tools**

Originally developed as an RPL developer toolkit.

### Upgrade

**Ndyndy — 12 RPL-1**

The upgraded architecture introduces the C++ / Rust core and modular System Module design while preserving the project's original purpose as a collection of practical developer tools.

---

## Status

> **In Development**

RPLKit is an evolving project. APIs, module interfaces, project structure, and supported runtimes may change as the architecture develops.

---

## License

License information will be added as the project is finalized.
