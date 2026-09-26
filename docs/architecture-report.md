# Laporan Arsitektur — RPLKit Developer Tools

> Tanggal: 2026-09-26
> Repo: `RPL-DEVTOOL` (branch `style/tui-refinement`)
> Sumber: verifikasi langsung kode + knowledge-graph (`1098 nodes`, `3561 edges`)
> Cakupan graph: bersih untuk file yang dikutip (`no_recorded_issue`); satu `parse_partial` tercatat di `core/cpp/include/rplkit/runtime_ffi.h:24`.

## 1. Ringkasan Eksekutif

RPLKit adalah toolkit developer modular dengan prinsip:

> **Keep the core stable. Keep the tools replaceable.**

- **Core native (C++ + Rust)** mengelola lifecycle, registry, eksekusi, dan TUI.
- **System Module (Python `modules/`)** menjadi dev-runner tanpa build: discovery manifest, dispatch ke runtime Python/TypeScript/Native.
- **Tools (`tools/<nama>/`)** independen, dideklarasikan via `tool.json` v2 dengan execution chain (`native → py-embed → py-sys → js-embed → node-sys`).
- **Dua mode UX:** CLI (scripting/automation) + TUI `ratatui` (interactive).
- **Python core + 8 tool Python = stdlib-only**, tanpa install. QR PNG opsional (`qrcode[pil]`), selebihnya fallback ASCII-box.

Status proyek: `In Development` — API/struktur masih dapat berubah.

## 2. Filosofi & Tujuan

1. Pisahkan **infrastructure (Core)** dari **capability (Modules)** dan **functionality (Tools)**.
2. Tool baru = tambah folder + JSON, **tanpa ubah Core** (`docs/tool-contract.md`).
3. Core native untuk operasi sistem, manajemen proses/resource, integrasi native, dan utilitas long-running.
4. Rust untuk safety/reliabilitas, C++ untuk kapabilitas sistem mature.

```
Core
 │  provides infrastructure
 ▼
Modules
 │  provide runtime / integration
 ▼
Tools
    provide actual functionality
```

## 3. Gambaran Umum Arsitektur

```
                 RPLKit
                   │
           ┌───────┴───────┐
           │  Core System  │
           │  C++ / Rust   │
           └───────┬───────┘
                   │
            System Module
                   │
     ┌─────────────┼─────────────┐
     │             │             │
  Python       TypeScript      Native
   Tools          Tools        Modules
```

```
RPLKit
├── Core
│   ├── Runtime
│   ├── Module Manager
│   └── System Services
├── System Module
│   ├── Python Runtime
│   ├── TypeScript Runtime (+ Native Runtime)
│   └── Module Interface (ToolMeta)
└── Tools
    ├── Python Tools
    ├── TypeScript Tools
    └── Native Tools
```

Dua mode eksekusi:

```
        RPLKit
          │
  ┌───────┴────────┐
  │                │
 CLI              TUI
  │                │
scripting      interactive
automation     developer UX
```

## 4. Inventaris Lapisan & Komponen

| Lapisan | Direktori / File | Bahasa | Peran |
|---|---|---|---|
| Entry | `main.py:1-11` | Python | Thin entry, delegasi ke `core.runtime.Runtime().run()` |
| Core Python | `core/runtime.py:34-95` | Python | Lifecycle: `interactive()`, `list_tools()`, `run(argv)` |
| Core Python | `core/module_manager.py:18-63` | Python | Discovery + dispatch antar runtime |
| Core Python | `core/menu.py:13-15` | Python | Kompat `run_menu()` → `Runtime().interactive()` |
| Core Python | `core/system_services.py` | Python | `run_process()` untuk tool Python CLI |
| System Module | `modules/__init__.py:24-37` | Python | Kontrak `ToolMeta(name, runtime, entry, module, group)` + `NATIVE_BUILTINS` |
| System Module | `modules/manifests.py:89-100` | Python | Satu-satunya sumber discovery: `tools/*/tool.json`, overlay-aware |
| System Module | `modules/python_runtime.py:86-96` | Python | In-process `load(meta).run()`, isolasi exception |
| System Module | `modules/typescript_runtime.py:47-58` | Python | Subprocess `node <entry>`, butuh Node ≥22 |
| System Module | `modules/native_runtime.py:18-46` | Python | Dispatch ke biner `build/cpp/rplkit` → `core/rust/target/*/rplkit` |
| Core Rust | `core/rust/Cargo.toml:1-30` | Rust | Workspace `rplkit-core`, `rplkit-runtime`, `rplkit-tui` |
| Core Rust | `core/rust/rplkit-core/src/` | Rust | Primitif aman: `calc`, `base64`, `uuid`, `time`, `sys` |
| Core Rust | `core/rust/rplkit-runtime/src/` | Rust | `registry`, `exec_native/py/js`, `chain`, `history`, `ffi`, `modules` |
| Core Rust TUI | `core/rust/rplkit-tui/src/` | Rust | `app.rs`, `views.rs`, `theme.rs`, `main.rs` (`ratatui 0.30`, `crossterm 0.29`, `sysinfo 0.39`, `nucleo 0.5`) |
| Core C++ | `core/cpp/src/main.cpp`, `runtime.cpp`, `module_manager.cpp`, `system_services.cpp` | C++ | CLI + menu + FFI tipis ke `librplkit_runtime.a` |
| Tools | `tools/<nama>/{tool.json,impl.py,impl.ts}` | Python/TS | Fungsionalitas aktual, dinamis per folder |
| Config | `config/default.json`, `config/modules.json` | JSON | Path core, runtime, overlay enable/disable |
| Kontrak | `docs/tool-contract.md` | Markdown | Spesifikasi tool v2 + modul + tema TUI |
| Build | `build.sh:15-30` | Bash | `cargo build` → `cmake build` (wajib urut) |
| Test | `tests/test_smoke.py`, `tests/test_tools.py` | Python | `unittest discover`, Rust via `cargo test` |

Statistik graph (mode `full`):

- Node: `Function 316`, `Variable 259`, `Class 36`, `Method 79`, `Module 98`, `File 99`.
- Edge: `DEFINES 1348`, `CALLS 791`, `USAGE 774`, `IMPORTS 56`.
- Bahasa terindeks: Python 34 file, Rust 21, C 4, C++ 4, TOML 4, Bash 1, TypeScript 1.
- Entry point: 13 (1 `main.py`, 1 C++ `main`, 2 Rust TUI, 9 `tools/*/impl.py`).

## 5. Alur Eksekusi (Execution Chain)

Setiap `tool.json` mendeklarasikan rantai, contoh `tools/calculator/tool.json:5-9`:

```json
"exec": ["native", "py-embed", "py-sys"]
```

Contoh TS `tools/hello/tool.json:5-8`:

```json
"exec": ["js-embed", "node-sys"]
```

Semantik (dari `README.md:224-229` + `docs/tool-contract.md:25-28`):

1. Runner mencoba berurutan sesuai `exec`.
2. `native` = op Rust (`native.op`, mis. `calc`, `sysinfo`).
3. `py-embed` = PyO3 in-process, `py-sys` = `python3` sistem.
4. `js-embed` = `swc` + QuickJS (`rquickjs 0.14`), `node-sys` = fallback `node`.
5. Interpreter sistem hanya jalan bila rantai native/embed gagal.
6. `--verbose` menampilkan `executed_by` aktual.

Dispatch Python (`core/module_manager.py:46-63`):

- `python` tanpa args → `python_runtime.run(tool)` in-process.
- `python` dengan args → `run_process([which_python(), entry, args])`.
- `typescript` → `typescript_runtime.run(tool, repo_root, args)`.
- `native` → `native_runtime.run(tool, repo_root, args)`.

Dispatch native (`modules/native_runtime.py:18-25`):

```
build/cpp/rplkit
→ core/rust/target/debug/rplkit
→ core/rust/target/release/rplkit
```

Bila tidak ada: cetak instruksi build, return 1 (tanpa traceback).

## 6. Discovery, Registry & Overlay

Satu-satunya sumber: manifest (`modules/manifests.py:16-46`).

- Scan `tools/*/tool.json` tanpa eksekusi kode.
- Prioritas entry: `entries.python` → `entries.typescript` → native-only (`native.op`).
- De-duplikasi by nama di `ModuleManager.discover()` (`core/module_manager.py:23-34`).
- Hasil terurut nama, identik di 3 core (Python/C++/Rust).

Efektif enable (`modules/manifests.py:71-86`):

```
override per-tool (config/modules.json#tools)
 → overlay modul (config/modules.json#modules)
 → default deskriptor (modules/*.module.json#enabled_by_default)
 → default true
```

Contoh overlay (`config/modules.json:3-8`):

```json
{"modules": {"school-tools": {"enabled": false}}, "tools": {}}
```

Mapping modul (`config/default.json:1-13`):

```json
{"cores": {"cpp": "build/cpp/rplkit", "rust": "core/rust/target/release/rplkit", "python": "main.py"}}
```

History runtime (`docs/tool-contract.md:60-66`): `config/history.json`, cap 50 entri, dibaca panel RECENT + Logs:

```json
{"runs": [{"tool": "calculator", "args": ["6 * 7"], "code": 0, "ms": 3, "by": "native", "at": 1790279000}]}
```

## 7. Runtime Detail

### 7.1 Python Runtime (`modules/python_runtime.py`)

- Discovery manifest dulu, fallback legacy `tools/python/*.py` bila tanpa manifest.
- `load()` via `importlib.import_module(dotted)`; valid bila `run()` callable.
- `run()` menangkap `KeyboardInterrupt` dan `Exception` agar tool tidak crash-kan core.

### 7.2 TypeScript Runtime (`modules/typescript_runtime.py`)

- Sengaja subprocess untuk isolasi (“Keep the tools replaceable”).
- Dependensi: `node` di PATH, disarankan Node ≥22 (type-stripping bawaan).

### 7.3 Rust Runtime (`core/rust/rplkit-runtime/src/`)

- `registry.rs` — `find` (fan-in 26), `discover` (fan-in 15): hotspot utama.
- `exec_native.rs` — `run_op` (fan-in 14).
- `chain.rs` — implementasi fallback chain.
- `exec_py.rs` — PyO3 embed-first (`pyo3 0.29`).
- `exec_js` implisit via `rquickjs 0.14` + `swc_core 80` untuk transpile TS.
- `history.rs` — cap 50, format di atas.
- `ffi.rs` — jembatan ke C++ (`librplkit_runtime.a`).
- `modules.rs` — `load_overlay`, `discover_enabled`.

### 7.4 C++ Core (`core/cpp/src/`)

- `main.cpp` — entry CLI.
- `runtime.cpp` — cerminan `core/runtime.py`.
- `module_manager.cpp` — `native_builtins()` + discovery manifest.
- `system_services.cpp` — operasi sistem level.

### 7.5 TUI (`core/rust/rplkit-tui/src/`)

- Tanpa argumen + tty → workspace terminal (`ratatui`, monokrom + satu aksen Blue Iris `#5B5BD6`).
- Views: Overview (system + recent), Tools (search `/`), Exec (timing + `executed_by` + copy), Modules (toggle `Space`), palette (`Ctrl+K`), Logs, Help.
- Tanpa tty (pipe/script) → teks bantuan, tidak hang.
- Token tema (`docs/tool-contract.md:68-74`): bg `#0B0F16`, panel `#11161F`, border `#2A3342`, teks `#C9D1D9`, redup `#6E7681`. Status via simbol `✓/✗`, bukan warna kedua.
- Keybinding: `1/2/3` Tools·Modules·Logs, `?` Help, `Enter` run/detail, `Space` enable/disable, `Ctrl+R`/`Ctrl+Y` run-again/copy, `Esc` back, `q` quit.
- Hotspot graph: `App::new` fan-in 37 — pusat konstruksi TUI.

## 8. Kontrak Tool (v2)

Dari `docs/tool-contract.md:6-48`:

```
tools/<nama>/
  tool.json   # wajib
  impl.py     # bila pakai py-*
  impl.ts     # bila pakai js-embed/node-sys
  native.rs   # (rencana) logika native per tool
```

| Field | Wajib | Isi |
|---|---|---|
| `name` | ya | sama dengan nama folder |
| `version` | ya | semver |
| `description` | ya | tampil di `--list`/TUI |
| `module` | ya | mis. `system-tools`, `dev-tools` |
| `group` | tidak | grup TUI, default = module |
| `exec` | ya | subset berurutan `native, py-embed, py-sys, js-embed, node-sys` |
| `native.op` | bila ada `native` | id op Rust |
| `entries.python` | bila ada `py-*` | relatif, biasanya `impl.py` |
| `entries.typescript` | bila ada `js-*`/`node-sys` | relatif, biasanya `impl.ts` |

- `impl.py`: `main(argv: list[str]) -> int`, `run() -> None` interaktif, baris kosong = kembali, stdlib-only.
- `impl.ts`: sintaks yang bisa diturunkan `swc` (termasuk `enum`), shim `process.argv` + `process.exit(code)`, tanpa `npm install`.
- Deskriptor modul: `modules/<nama>.module.json` (`name, version, author, description, runtime, enabled_by_default`).

## 9. Struktur Direktori (aktual)

```
rplkit/
├── core/
│   ├── cpp/          # CLI + menu + FFI tipis ke librplkit_runtime.a
│   └── rust/         # workspace: rplkit-core + rplkit-runtime + rplkit-tui
├── modules/          # System Module (Python dev-runner, tanpa build)
│   ├── manifests.py
│   ├── python_runtime.py
│   ├── typescript_runtime.py
│   └── native_runtime.py
├── tools/<nama>/
│   ├── tool.json
│   ├── impl.py / impl.ts
├── config/default.json
├── config/modules.json
├── build.sh
└── README.md
```

File-tree terindeks: 111 entri. Direktori `build/`, `target/`, `__pycache__/`, `assets/*.svg`, `config/history.json` sengaja tidak diindeks (gitignore/skip-list).

## 10. Dependensi

Python (`requirements.txt`): stdlib-only untuk core + 8 tool. Opsional `qrcode[pil]`.

Rust workspace (`core/rust/Cargo.toml:10-30`):

- `pyo3 0.29` (embed Python)
- `rquickjs 0.14` (embed JS)
- `serde 1` + `serde_json 1`
- `swc_core 80` (transpile TS)
- `ratatui 0.30`, `crossterm 0.29`, `sysinfo 0.39`, `nucleo 0.5` (TUI, pin di `Cargo.lock`)

Build order wajib (`build.sh`):

```sh
./build.sh            # debug: cargo + cmake
./build.sh --release  # rilis
python3 -m unittest discover -s tests
cargo test --manifest-path core/rust/Cargo.toml
```

## 11. Analisis Graph: Hotspot, Boundary, Cluster

Hotspot (fan-in tertinggi):

- `Lexer::new` (42), `App::new` (37) — konstruksi inti Rust/TUI.
- `registry::find` (26), `registry::discover` (15), `exec_native::run_op` (14) — jalur eksekusi terpanas.
- `builtins.len` (44), `print` (27) — utilitas Python.

Boundary antar package:

- `runtime → module_manager` (10 calls) — dispatch terpusat.
- `test_tools → rust` (7), `converter → str` (7) — tool memanggil primitif core.
- `rust → cpp` (3) — FFI tipis, sesuai desain.

Cluster Leiden (12):

- `core 59 (cohesion 0.69)`: `new/find/discover/run_tool_captured`.
- `core 56 (0.85)`: `new/on_key/chrome/render_detail` — TUI.
- `modules 56 (0.78)`: `print/run/interactive` — System Module Python.
- `core 40 (0.72)`: `run_op/snapshot/calc` — eksekusi native.

Interpretasi: kohesi tinggi di TUI dan runtime; System Module Python terpisah jelas dari core native — sesuai filosofi.

## 12. Kualitas, Risiko & Rekomendasi

**Kekuatan:**

- Kontrak tool tunggal, discovery tanpa eksekusi kode.
- Execution chain native-first dengan fallback terdokumentasi + `executed_by`.
- Overlay enable/disable tanpa menyentuh `tool.json`.
- TUI tidak hang di pipe, tema satu-aksen terdokumentasi.

**Gap / risiko:**

1. Isolasi modul embed belum sandbox penuh (`README.md:157` unchecked).
2. Manajemen runtime (`README.md:158`) belum selesai — kandidat: version pin, health check.
3. `parse_partial` di `runtime_ffi.h:24` — verifikasi manual bila ubah FFI.
4. Duplikasi logika discovery di 3 core (Python/Rust/C++) — perlu conformance test agar `--list` tetap identik.
5. Fallback legacy `tools/python/*.py` masih ada di kode meski dokumen menyebut dihapus Fase 7 — putuskan: hapus atau dokumentasikan.

**Rekomendasi berikutnya:**

- [ ] Sandbox embed (PyO3 + QuickJS) + batas resource.
- [ ] Conformance test `discover --list` lintas core.
- [ ] Hapus/arsipkan path legacy bila Fase 7 final.
- [ ] Tambah diagram sekuens `CLI → ModuleManager → chain → executed_by → history`.

## 13. Referensi File Kunci

- `main.py:6-7` — entry.
- `core/runtime.py:38-95` — lifecycle.
- `core/module_manager.py:23-63` — discovery + dispatch.
- `modules/__init__.py:24-54` — `ToolMeta` + `NATIVE_BUILTINS`.
- `modules/manifests.py:89-110` — discovery + overlay.
- `docs/tool-contract.md:1-79` — kontrak lengkap.
- `config/default.json:1-13`, `config/modules.json:1-9` — konfigurasi.
- `build.sh:1-32` — urutan build.
- `core/rust/Cargo.toml:1-30` — dependensi Rust.

---
*Dibuat otomatis dari pembacaan sumber + graph. Klaim cakupan bersifat best-effort, bukan jaminan kelengkapan.*
