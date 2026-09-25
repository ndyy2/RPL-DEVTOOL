# RPLKit Tool Contract (v2 + modules)

Satu-satunya kontrak yang perlu dipenuhi tool baru. Core tidak pernah
berubah saat tool ditambah — cukup folder + JSON di bawah ini.

## 1. Folder tool: `tools/<nama>/`

```
tools/system-info/
  tool.json   # manifest (wajib)
  impl.py     # entry Python (bila dipakai)
  impl.ts     # entry TypeScript (bila dipakai)
  native.rs   # (rencana) logika native per tool
```

## 2. `tool.json`

| Field | Wajib | Isi |
|---|---|---|
| `name` | ya | sama dengan nama folder |
| `version` | ya | semver string |
| `description` | ya | satu baris, tampil di `--list`/TUI |
| `module` | ya | nama modul (`system-tools`, …) |
| `group` | tidak | grup tampil di TUI Tools view (default = module) |
| `exec` | ya | rantai, subset berurutan dari `native, py-embed, py-sys, js-embed, node-sys` |
| `native.op` | bila exec memuat `native` | id op Rust (`calc`, `sysinfo`, …) |
| `entries.python` | bila exec memuat `py-*` | path relatif folder tool, biasanya `impl.py` |
| `entries.typescript` | bila exec memuat `js-embed`/`node-sys` | path relatif, biasanya `impl.ts` |

## 3. Kontrak `impl.py`

`main(argv: list[str]) -> int`, argv = argumen tool **tanpa** nama prog
(konsisten dengan cabang `sys.argv[1:]`). Return 0 sukses. Interaktif
via `run() -> None`; baris kosong = kembali. Tanpa dep non-stdlib
kecuali dinyatakan di deskripsi.

## 4. Kontrak `impl.ts`

Sintaks apa pun yang `swc` bisa turunkan ke JS (termasuk `enum`);
sisanya jatuh ke fallback `node`. Sediakan shim `process.argv`
(`[node, script, ...args]`) dan `process.exit(code)` — runner
menyediakannya, tool hanya memakai. Tanpa `npm install`.

## 5. Deskriptor modul: `modules/<nama>.module.json`

`{name, version, author, description, runtime, enabled_by_default}`.
`runtime` = runtime utama modul (`rust`/`mixed`/`python`); tool
individual boleh menimpa lewat `exec`-nya sendiri.

## 6. Overlay: `config/modules.json`

```json
{"modules": {"school-tools": {"enabled": false}}, "tools": {"qr_generator": {"enabled": true}}}
```

Registry merge: overlay modul → default deskriptor → override per tool
(default enabled bila tak disebut di mana pun). `tool.json` tidak
pernah ditulis oleh tooling.

## 7. History: `config/history.json` (dibuat saat runtime)

```json
{"runs": [{"tool": "calculator", "args": ["6 * 7"], "code": 0, "ms": 3, "by": "native", "at": 1790279000}]}
```

Cap 50 entri, terbaru dulu. Panel RECENT + Logs membaca file ini.

## 8. Token tema TUI (monochrome + satu aksen)

- Latar: `#0B0F16`, panel: `#11161F`, border: `#2A3342`, teks: `#C9D1D9`
- Teks redup: `#6E7681`, **aksen satu-satunya Blue Iris `#5B5BD6`**
- Aturan: aksen hanya untuk selection/focus/status. Dilarang warna
  kedua (hijau/merah/kuning) di UI — status sukses/gagal dibedakan
  lewat simbol (`✓`/`✗`) + teks, bukan warna.

## 9. Versi pin dependensi TUI (Fase 4)

`ratatui 0.30`, `crossterm 0.29`, `sysinfo 0.39`, `nucleo 0.5`
(dikunci di `Cargo.lock`).
