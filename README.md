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
* [x] Support Python-based tools (`tools/python/`, 8 tools, stdlib-only)
* [x] Support TypeScript-based tools (`tools/typescript/hello.ts` via node runtime)
* [x] Expand module API (`modules/` ToolMeta interface + manifests)
* [ ] Improve module isolation
* [x] Add module discovery (C++/Rust/Python `--list` from `tools/*` + manifests)
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

The exact structure may evolve as development continues.

```text
rplkit/
│
├── core/
│   ├── cpp/
│   └── rust/
│
├── modules/
│   ├── python/
│   ├── typescript/
│   └── native/
│
├── tools/
│   ├── python/
│   └── typescript/
│
├── config/
│
└── README.md
```

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
