# RPLKit Developer Tools

**Upgraded by Ndyndy — 12 RPL-1**

<p align="center">
  <img src="assets/architecture.svg" alt="RPLKit animated architecture: native C++/Rust core, System Module, and replaceable tools" width="900" />
</p>

> A modern, modular developer toolkit rebuilt from Python into a native C++ and Rust-based core.

RPLKit Developer Tools is an upgraded version of the original **RPLKit**, redesigned to provide a faster, more extensible, and modular foundation for developer utilities.

The project moves its core architecture away from the original Python-based implementation toward **C++ and Rust**, while introducing a **System Module** that allows individual tools to remain flexible and easy to develop.

---

## Overview

RPLKit was originally built as a collection of developer-oriented utilities using Python.

This upgraded version focuses on three major improvements:

* Native **C++ and Rust core**
* Modular **System Module architecture**
* Support for writing tools using **Python or TypeScript**

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

The core runtime is built primarily with:

* **C++** — system-level functionality and native runtime
* **Rust** — safe, performant components and supporting infrastructure

The native core is responsible for managing the application lifecycle, modules, communication, and system-level operations.

---

## System Module

The **System Module** is the main architectural improvement introduced in the upgraded RPLKit.

Instead of implementing every tool directly inside the core application, tools are treated as independent modules.

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
│   ├── TypeScript Runtime
│   └── Module Interface
│
└── Tools
    ├── Python Tools
    ├── TypeScript Tools
    └── Native Tools
```

This allows developers to create new tools without tightly coupling them to the core.

---

## Multi-Language Tools

RPLKit is designed to support multiple languages at the tool level.

### Python

Python can be used when rapid development and scripting capabilities are preferred.

```text
RPLKit Core
    │
    └── Python Module
            │
            └── tool.py
```

### TypeScript

TypeScript can be used for tools that benefit from the JavaScript ecosystem and type safety.

```text
RPLKit Core
    │
    └── TypeScript Module
            │
            └── tool.ts
```

This approach keeps the core native while allowing developers to choose the most appropriate language for individual tools.

---

## Why C++ + Rust?

The upgrade does not simply replace Python for the sake of performance.

The native core provides a stronger foundation for:

* System-level operations
* Process management
* Resource control
* Native integrations
* Runtime management
* Modular execution
* Long-running developer utilities

Rust is used where memory safety and reliability are important, while C++ provides mature native-system capabilities and ecosystem compatibility.

---

## Project Goals

RPLKit aims to become a small but extensible developer platform rather than a static collection of scripts.

### Current Direction

* [x] Replace the original Python-based core
* [x] Introduce C++ / Rust architecture
* [x] Design modular system
* [x] Introduce System Module
* [x] Support Python-based tools (8 tools, stdlib-only, embed-first via PyO3)
* [x] Support TypeScript-based tools (`tools/hello/impl.ts`, swc + QuickJS embed)
* [x] Expand module API (`modules/` ToolMeta interface + `tool.json` v2 manifests)
* [x] Native-first execution chain (native → embed → sistem fallback + `executed_by`)
* [ ] Improve module isolation (sandboxing embed)
* [x] Add module discovery (`tools/*/tool.json`, identik di 3 core)
* [ ] Improve runtime management
* [ ] Expand developer utilities

---

## Design Philosophy

RPLKit follows a simple principle:

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

This makes the project easier to extend as the number of developer tools grows.

---

## Project Structure

```text
rplkit/
│
├── core/
│   ├── cpp/          # CLI + menu + FFI tipis ke librplkit_runtime.a
│   └── rust/         # workspace: rplkit-core + rplkit-runtime (runner)
│       ├── rplkit-core/     # primitif aman: calc, base64, uuid, time
│       └── rplkit-runtime/  # registry, exec_native/py/js, chain, FFI
│
├── modules/          # System Module (Python dev-runner, tanpa build)
│   ├── manifests.py  # scan tools/*/tool.json
│   ├── python_runtime.py
│   ├── typescript_runtime.py
│   └── native_runtime.py
│
├── tools/<nama>/     # folder dinamis per tool
│   ├── tool.json     # manifest v2: exec chain + entries
│   ├── native.rs     # (rencana) logika native per tool
│   ├── impl.py       # implementasi Python (embed-first via PyO3)
│   └── impl.ts       # implementasi TypeScript (swc + QuickJS)
│
├── config/default.json
├── build.sh          # cargo build → cmake build (wajib urut)
└── README.md
```

### Execution chain

Setiap tool mendeklarasikan rantai di `tool.json`, mis.
`"exec": ["native", "py-embed", "py-sys"]`. Runner mencoba berurutan —
interpreter sistem (`python3`/`node`) hanya jalan bila rantai native
gagal. `--verbose` menampilkan `executed_by` yang sebenarnya mengeksekusi.

### Build

```sh
./build.sh            # debug: cargo + cmake
./build.sh --release  # rilis
python3 -m unittest discover -s tests   # Python (tanpa pytest)
cargo test --manifest-path core/rust/Cargo.toml  # Rust
```

### Dua mode: CLI + TUI

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

Tanpa argumen + tty → terminal workspace (`ratatui`, monokrom + satu
aksen Blue Iris): Overview (system + recent), Tools (search `/`),
Exec (timing + `executed_by` + copy), Modules (toggle `Space`),
palette (`Ctrl+K`), Logs, Help. Tanpa tty (pipe/script) → teks bantuan,
tak pernah hang.

```sh
rplkit                              # TUI
rplkit system-info                   # langsung jalan (scripting)
rplkit module list                   # modul + status
rplkit module disable school-tools   # overlay config/modules.json
```

Keybinding: `1/2/3` Tools·Modules·Logs, `?` Help, `Enter` run/detail,
`Space` enable/disable, `Ctrl+R`/`Ctrl+Y` (Exec) run-again/copy,
`Esc` back, `q` quit.

---

## Legacy → Upgraded

| Area               | Original RPLKit  | RPLKit Upgraded     |
| ------------------ | ---------------- | ------------------- |
| Core               | Python           | C++ + Rust          |
| Architecture       | Monolithic tools | Modular             |
| Tool Runtime       | Python           | Python + TypeScript |
| System Module      | —                | ✓                   |
| Extensibility      | Limited          | Module-based        |
| Native Integration | Limited          | Native core         |
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
