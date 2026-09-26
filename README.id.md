# RPLKit Developer Tools

**Ditingkatkan oleh Ndyndy — 12 RPL-1**

> **Bahasa Indonesia** (Anda di sini) | [🇬🇧 English](README.md)

<p align="center">
  <img src="assets/architecture.svg" alt="Arsitektur animasi RPLKit: core native C++/Rust, System Module, dan tools yang dapat diganti" width="900" />
</p>

> Toolkit developer modern dan modular dengan core native C++ dan Rust.

RPLKit Developer Tools adalah versi tingkat dari **RPLKit** orisinal, dirancang ulang agar lebih cepat, lebih mudah diperluas (*extensible* — mudah ditambah kemampuannya), dan modular sebagai fondasi utilitas developer.

Core-nya **benar-benar diimplementasikan secara native di C++ dan Rust** (bukan aspirasi — sudah bisa di-*build* dan dijalankan hari ini), dengan **System Module** yang membuat masing-masing tool tetap fleksibel dan mudah dikembangkan.

---

## Gambaran Umum

RPLKit pada awalnya dibangun sebagai kumpulan utilitas berorientasi developer menggunakan Python.

Versi tingkat ini berfokus pada tiga peningkatan besar:

* Core native **C++ dan Rust** (workspace: `rplkit-core` + `rplkit-runtime` + `rplkit-tui`)
* Arsitektur **System Module** yang modular (registry berbasis manifest di `modules/`)
* Dukungan penulisan tool memakai **Python atau TypeScript** (**18 tool**, **5 modul**)

Tujuannya adalah memisahkan sistem inti aplikasi dari masing-masing tool, sehingga fungsionalitas baru dapat ditambahkan tanpa mengubah seluruh aplikasi.

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

## Arsitektur

<p align="center">
  <img src="assets/architecture-chart.svg" alt="Bagan arsitektur RPLKit: entry point, runtime Rust/C++, TUI, alur data tools dan config" width="1200" />
</p>

### Core

Runtime inti dibangun dengan:

* **C++** — CLI, menu, dan lapisan FFI tipis di atas `librplkit_runtime.a`
* **Rust** — primitif yang aman (`rplkit-core`), mesin eksekusi (`rplkit-runtime`), workspace interaktif (`rplkit-tui`)
* **Python (dev-runner)** — `main.py` + `core/` mencerminkan lifecycle native sehingga `rplkit --list` identik di ketiga core

Core native mengelola lifecycle aplikasi, registry modul, rantai eksekusi (*execution chain* — urutan percobaan runner), dan operasi level sistem.

---

## System Module

**System Module** adalah peningkatan arsitektur utama di RPLKit yang ditingkatkan.

Alih-alih mengimplementasikan setiap tool di dalam aplikasi inti, tool diperlakukan sebagai modul independen yang ditemukan (*discovery* — pemindaian otomatis) dari manifest:

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
    └── Native Tools (8 op via Rust core)
```

Ini memungkinkan developer membuat tool baru tanpa terikat (*tightly coupled* — ketergantungan kaku) pada core.

---

## Multi-Language Tools

### Python

Python dipakai untuk pengembangan cepat dan kemampuan scripting.

```text
RPLKit Core
    │
    └── Python Module
            │
            └── tool.py
```

### TypeScript

TypeScript dipakai untuk tool yang diuntungkan ekosistem JavaScript dan keamanan tipe (*type safety* — tipe data diperiksa).

```text
RPLKit Core
    │
    └── TypeScript Module
            │
            └── tool.ts
```

Pendekatan ini menjaga core tetap native sekaligus membebaskan developer memilih bahasa paling tepat untuk tiap tool.

---

## Mengapa C++ + Rust?

Peningkatan ini bukan sekadar mengganti Python demi performa.

Core native memberi fondasi lebih kuat untuk:

* Operasi level sistem
* Manajemen proses
* Kontrol resource
* Integrasi native
* Manajemen runtime
* Eksekusi modular
* Utilitas developer yang berjalan lama (*long-running* — hidup lama di background)

Rust dipakai di bagian yang penting keamanan memori dan reliabilitasnya, sementara C++ memberi kapabilitas sistem native yang matang dan kompatibilitas ekosistem. Python tetap bahasa tool kelas satu (embed-first via PyO3, stdlib-only — hanya pustaka bawaan).

---

## Tujuan Proyek

RPLKit bertujuan menjadi platform developer kecil namun dapat diperluas, bukan sekadar kumpulan skrip statis.

### Arah Saat Ini

* [x] Mengganti core berbasis Python orisinal
* [x] Menghadirkan arsitektur C++ / Rust (ter-build via `./build.sh`)
* [x] Merancang sistem modular
* [x] Menghadirkan System Module
* [x] Mendukung tool berbasis Python (9 tool, stdlib-only, embed-first via PyO3)
* [x] Mendukung tool berbasis TypeScript (`tools/hello/impl.ts`, embed swc + QuickJS)
* [x] Memperluas API modul (interface ToolMeta di `modules/` + manifest `tool.json` v2)
* [x] Rantai eksekusi native-first (native → embed → fallback sistem + `executed_by`)
* [x] Menambah discovery modul (`tools/*/tool.json`, identik di 3 core, paritas 18/18)
* [ ] Meningkatkan isolasi modul (sandboxing embed — pembatasan aman)
* [ ] Meningkatkan manajemen runtime (mis. Node worker persisten, health check)
* [ ] Memperluas utilitas developer (form, custom UI per tool)

---

## Filosofi Desain

> **Jaga core tetap stabil. Jaga tool tetap dapat diganti.**

Core menyediakan infrastruktur.

Modul menyediakan runtime / integrasi.

Tool tetap independen.

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

## Struktur Proyek

```text
rplkit/
│
├── core/
│   ├── cpp/          # CLI + menu + FFI tipis ke librplkit_runtime.a
│   ├── __init__.py, runtime.py, module_manager.py, menu.py, system_services.py
│   └── rust/         # workspace: rplkit-core + rplkit-runtime + rplkit-tui
│       ├── rplkit-core/     # primitif aman: calc, base64, uuid, time, sys
│       ├── rplkit-runtime/  # registry, exec_native/py/js, chain, history, FFI
│       └── rplkit-tui/      # workspace ratatui (Overview/Tools/Exec/Modules/Logs/Help)
│
├── modules/          # System Module (Python dev-runner, tanpa perlu build)
│   ├── __init__.py       # ToolMeta + NATIVE_BUILTINS
│   ├── manifests.py      # memindai tools/*/tool.json (satu-satunya sumber)
│   ├── python_runtime.py
│   ├── typescript_runtime.py
│   └── native_runtime.py
│
├── tools/<nama>/     # 18 folder dinamis per tool
│   ├── tool.json     # manifest v2: exec chain + entries
│   ├── impl.py       # entry Python (embed-first via PyO3)
│   └── impl.ts       # entry TypeScript (swc + QuickJS)
│
├── modules/*.module.json  # 5 deskriptor modul (dev/git/network/school/system)
├── config/default.json    # path biner core + runtime
├── config/modules.json    # overlay enable/disable (tooling tak pernah menulis tool.json)
├── docs/tool-contract.md  # kontrak tool v2
├── docs/architecture-report.md  # laporan arsitektur hasil generate
├── docs/roadmap.md    # planning backlog yang disetujui namun belum dieksekusi
├── build.sh           # cargo build → cmake build (urutan wajib)
└── README.md / README.id.md
```

### Execution chain

Setiap tool mendeklarasikan rantai di `tool.json`, mis.
`"exec": ["native", "py-embed", "py-sys"]`. Runner mencoba berurutan —
interpreter sistem (`python3`/`node`) hanya jalan bila rantai native/embed
gagal. `--verbose` menampilkan `executed_by` yang sebenarnya mengeksekusi.

### Build & test

```sh
./build.sh            # debug: cargo + cmake
./build.sh --release  # rilis
python3 -m unittest discover -s tests   # Python: 18 test, stdlib saja (tanpa pytest)
cargo test --manifest-path core/rust/Cargo.toml  # Rust: 9 + 55 + 21 = 85 test
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

Tanpa argumen + tty → workspace terminal (`ratatui`, monokrom + satu
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

## Yang Baru

Pekerjaan yang baru mendarat (lihat `git log`):

* **Runner embedded-first + modul dinamis + workspace TUI** — Python embed-first PyO3, jalur TypeScript swc + QuickJS, manifest `tool.json` v2, pelaporan `executed_by`.
* **Sekat input, optimasi registry, index, bridge, cache** — penguatan penanganan input TUI, discovery/registry lebih cepat, kerja paritas lintas core.
* **Core native C++/Rust + System Module** — lifecycle, module manager, system services pindah ke native; 8 tool Python orisinal dipertahankan dan tumbuh menjadi **18 tool / 5 modul**.
* **Docs** — SVG arsitektur animasi, `docs/architecture-report.md` hasil generate, snapshot backlog `docs/roadmap.md`.

Snapshot terverifikasi: **18 manifest tool**, **5 deskriptor modul**, Python **18/18 hijau**, Rust **85/85 hijau (9 core + 55 runtime + 21 TUI)**, paritas `--list` Python **18 tool**.

---

## Roadmap (Perencanaan)

Detail penuh: [`docs/roadmap.md`](docs/roadmap.md). Bedah arsitektur: [`docs/architecture-report.md`](docs/architecture-report.md).

| # | Rencana | Status | Ukuran |
|---|---|---|---|
| 1 | Verifikasi sisa rafinasi style TUI | Verifikasi tertunda | Kecil |
| 2 | README bilingual (EN + ID) + refresh | **Selesai oleh perubahan ini** | Kecil |
| 3 | Node worker persisten + fallback (F5) | Backlog | Sedang |
| 4 | Form loader + live preview + custom UI per tool (F6) | Backlog | Besar |
| 5 | TS TUI (Ink) + daemon RPC | Disetujui, belum eksekusi | Terbesar |
| 6 | Gerbang terima F7 (watcher, sinkron docs, full suite) | Backlog | Sedang |

Urutan saran: 2 → 1 → 4 → 3 → 5 → 6.

Konvensi terkunci lintas rencana: aturan sekat input (mode input menelan semua keystroke), `./build.sh` + `cargo test` + `python3 -m unittest`, tema satu aksen (monokrom + Blue Iris `#5B5BD6`), `tool.json` tak pernah ditulis tooling, `config/history.json` adalah runtime-state yang di-gitignore.

---

## Legacy → Upgraded

| Area               | RPLKit Orisinal  | RPLKit Upgraded     |
| ------------------ | ---------------- | ------------------- |
| Core               | Python           | C++ + Rust (native, sudah ter-build) |
| Arsitektur         | Monolithic tools | Modular (18 manifest) |
| Tool Runtime       | Python           | Python (9) + TypeScript (1) + Native (8) |
| System Module      | —                | ✓ (registry manifest + overlay) |
| Ekstensibilitas    | Terbatas         | Berbasis modul (tambah folder + JSON) |
| Integrasi Native   | Terbatas         | Core native + FFI + embed |
| Pengembangan Tool  | Python           | Python / TypeScript |

---

## Kredit

**RPLKit Developer Tools**

Awalnya dikembangkan sebagai toolkit developer RPL.

### Peningkatan

**Ndyndy — 12 RPL-1**

Arsitektur tingkat menghadirkan core C++ / Rust dan desain System Module modular sambil menjaga tujuan awal proyek sebagai kumpulan tool developer yang praktis.

---

## Status

> **Dalam Pengembangan**

RPLKit adalah proyek yang terus berkembang. API, interface modul, struktur proyek, dan runtime yang didukung dapat berubah seiring perkembangan arsitektur.

---

## Lisensi

Informasi lisensi akan ditambahkan setelah proyek difinalisasi.
