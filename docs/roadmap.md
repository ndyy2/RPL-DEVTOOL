# RPLKit — Backlog Planning (Belum Terealisasi)

> Status snapshot: branch `style/tui-refinement` @ `01668fb`, tree bersih.
> Suite terakhir: Rust 80 (9+50+21), Python 15, paritas `--list` 18/18/18.
> Dokumen ini dibuat 2026-09-27 dari planning yang sudah disetujui
> tetapi belum dieksekusi. Yang sudah DONE tidak diulang di sini
> (lihat activity PCC + `git log`).

## Ringkasan prioritas

| # | Plan | Status | Estimasi | Ketergantungan |
|---|---|---|---|---|
| 1 | Verifikasi sisa rafinasi style TUI | Verifikasi tertunda | Kecil | — |
| 2 | README bilingual (EN + ID) + refresh | Disetujui, belum eksekusi | Kecil | — |
| 3 | F5 Node worker persisten | PCC BACKLOG | Sedang | — |
| 4 | F6 form loader + live + custom UI | PCC BACKLOG | Besar | — |
| 5 | TS TUI (Ink) + daemon RPC | Disetujui, belum eksekusi | Terbesar | #4 (acuan perilaku) |
| 6 | F7 terima (watcher, docs, suite) | PCC BACKLOG | Sedang | Semua di atas |

Urutan saran: 2 → 1 → 4 → 3 → 5 → 6.

---

## 1. Verifikasi sisa rafinasi style TUI

**Konteks.** Todo internal mencatat F3 + verifikasi menggantung, tetapi
tree bersih dan HEAD (`01668fb`) tidak menyebut style di pesannya.
Kemungkinan sudah ter-commit tercampur — belum diverifikasi.

**Lingkup verifikasi:**
- `git show --stat` + diff `theme.rs`, `views/*`, `app.rs` sejak branch dibuat.
- Pastikan: token single-accent utuh, goldens hijau, `clippy` tanpa temuan
  baru di crate `rplkit-tui`, tidak ada file debug sisa (`examples/snap.rs`
  sudah dihapus saat itu — konfirmasi tak muncul lagi).
- Bila ada sisa (mis. footer/help belum sinkron aturan sekat input),
  selesaikan sebagai patch kecil, bukan fase baru.

**Gate:** `cargo test -p rplkit-tui` hijau + `cargo clippy -p rplkit-tui`
bersih + daftar eksplisit "tidak ada sisa".

---

## 2. README bilingual + refresh isi

**Keputusan terkunci:** dua berkas (`README.md` Inggris default +
`README.id.md` Indonesia), badge pindah bahasa di puncak tiap berkas,
terjemah + refresh (bukan terjemah buta).

**Lingkup:**
- Fase 1 — Audit 310 baris → tandai basi vs arsitektur kini
  (klaim C++/Rust aspirasional → kini nyata; "9 tools", "31+13 tests"
  → 18 tools / 5 modul / 70+15 test; tabel Legacy; status; License).
- Fase 2 — Refresh `README.md` (Inggris) sebagai dasar.
- Fase 3 — `README.id.md` 1:1 struktur; istilah teknis tetap Inggris
  (*fallback*, *toolchain*, *registry*, *embed*) dengan penjelasan
  Indonesia di kemunculan pertama; code block tak diterjemahkan.
- Fase 4 — Verifikasi: heading paralel, link/badge valid, path SVG ada,
  code block identik di kedua bahasa.

**Risiko:** kecil. Murni docs, tanpa sentuh kode.

---

## 3. F5 — Node worker persisten + fallback

**Konteks.** Kini tiap run TS = spawn `node` baru (mahal bila tool TS
bertambah; kini baru 1: `hello`). PCC: `cmuilx2qr000404jrcnaf8ih4`.

**Desain:**
- Protokol JSON-line over stdio: `{"id","tool","args"}` →
  `{"id","code","output"}`; worker preload `impl.ts` yang sudah di-strip.
- Chain `node-sys` → worker bila hidup, fallback spawn bila mati;
  worker mati saat idle (default 30 dtk, konfigurasional).
- Hanya aktif bila ≥1 tool TS.

**Gate:** 2 run TS beruntun tanpa spawn baru (saksikan PID sama) +
fallback spawn teruji via kill worker.

**Risiko jujur:** IPC menambah failure modes (hang, partial line,
versi node) → mitigasi timeout + fallback otomatis + opt-out via config.
**Boleh dipotong** tanpa merusak fondasi bila tool TS tetap 1.

---

## 4. F6 — Form loader + live preview + custom UI per tool

**Konteks.** PCC: `cmuilx2qr000504jr6x9prsa4` (tergabung dengan F7).
Prinsip disetujui: *loader generik berbasis format I/O, bukan form
hardcode; tool tak berubah; CLI positional tetap jalan.*

**4a. Skema format di `tool.json`:**

```json
"inputs": [
  {"name": "expression", "type": "string", "label": "Expression",
   "placeholder": "2 + 3 * 4", "required": true, "live": true},
  {"name": "count", "type": "integer", "default": "1", "min": 1, "max": 100},
  {"name": "unit", "type": "select", "options": ["C", "F", "K", "km", "mi", "kg", "lb"]}
],
"argv": ["{expression}"],
"live": true
```

Tipe: `string` · `text` · `integer` · `number` · `boolean` · `select`
(opsi statis) · `path`. Field kosong non-required → argumen di-skip
(rapat kanan). `"live": true` per tool, **default false**.

**4b. Aturan keras (dikunci test):** tool `live:false` (`git-clean`,
`project-init`, …) **nol** auto-run saat diketik. Live hanya untuk tool
murni/read-only. Validasi di TUI; tool tak pernah terima data kotor
dari form.

**4c. Custom UI per tool** (amandemen disetujui: layar penuh bebas +
auto-form default):

```json
"ui": {"apiVersion": 1, "component": "./ui.tsx", "kind": "screen"}
```

- Tanpa blok `ui` → auto-form dari `inputs` (semua 18 tool langsung jalan).
- Dengan `ui` → dynamic-import `./ui.tsx`; gagal → fallback auto-form +
  pesan jujur (bukan crash). Trust model eksplisit: *tool code = trusted
  local code* (sama seperti `impl.py`); `run()` via RPC satu-satunya
  jalan eksekusi; token tema yang sama (monokrom + Blue Iris).
- Level modul didukung (`modules/<nama>.module.json` boleh menunjuk UI).
- Contoh custom: 1 tool saja (`calculator`: keypad + live result);
  17 sisanya auto-form.

**Fase:** (0) parser + validasi skema + contoh calculator/converter →
(1) model form state + mapping template→argv (unit murni) →
(2) render Exec + navigasi field Tab → (3) live debounce 250ms ikut tick →
(4) skema 16 tool sisa (paritas run form ≡ CLI positional) →
(5) contoh `ui.tsx` kalkulator.

**Gate:** skema invalid ditolak berpesan; simulasi tick → run tepat 1×;
tool non-live 0×; overhead embed diukur (>100ms → naikkan debounce).

**Risiko jujur:** overhead live-run embed Python (modul di-cache
`sys.modules`, ukur bukan asumsi); select dinamis di luar scope
(`source: "static"` disiapkan); template → `Vec<String>` langsung justru
lebih benar dari `split_args` hari ini.

---

## 5. TS TUI (Ink) + daemon RPC

**Keputusan terkunci:** TS gantikan Rust · daemon RPC (stdio NDJSON,
bukan socket) · Ink + tsx. Rust bin CLI tak tersentuh sepanjang fase.

**Arsitektur:**

```text
rplkit tui ─┬─ spawn rplkit-daemon ──stdio NDJSON── JSON-RPC 2.0
            └─ tsx tui-ts/src/app.tsx ◀────────────┘
```

**Kontrak RPC** (`docs/rpc-contract.md`, `apiVersion: 1` di handshake):
`tools.list` · `tools.run` (+ notifikasi `output/append`) ·
`module.list/info/enable/disable` · `system.snapshot` · `history.list` ·
`tools.build-index` · `shutdown`. Daemon = adapter tipis di atas
Registry/chain yang ada (tanpa logika baru). Tool interaktif REPL:
TUI suspend + `rplkit --run` attached + resume (keterbatasan v1
yang didokumentasikan).

**Struktur `tui-ts/`:** `app.tsx` · `rpc.ts` · `theme/` ·
`components/` (Header, Panel, ToolCard, StatusBar, CommandPalette,
DataTable) · `screens/` (7 view) · `ToolHost` (loader `ui.tsx`,
error boundary, fallback auto-form).

**Fase:** (0) kontrak → (1) daemon + test pipe → (2) streaming run →
(3) client + theme → (4) 7 screens + ToolHost + contoh kalkulator →
(5) wiring + README → (6) sunset Rust TUI (beku → hapus setelah
1 rilis + goldens TS hijau).

**Risiko jujur:** Ink v6 + React 19 churn (pin + frozen lockfile);
orphan daemon (stdio child ikut mati + heartbeat); Windows di luar
scope; Fase 4 terbesar (potong Logs/Help dulu bila perlu — daemon
tetap berguna untuk `rplkit-web` nanti).

---

## 6. F7 — Terima (gerbang akhir, setelah semua di atas)

- [ ] Watcher dev (`--dev` + inotify: rescan tool yang berubah saja).
- [ ] Docs sinkron: kontrak, diagram, README (termasuk §4–5 bila jadi).
- [ ] Full suite hijau: Rust workspace + Python + paritas 3 core +
        goldens (Rust dan/atau TS sesuai fase sunset).
- [ ] Binary manifest (`tools.idx` biner via mmap): **TETAP DITUNDA**
        sampai tool ratusan (keputusan sadar, bukan lupa).

---

## Keputusan terkunci yang berlaku lintas plan

- `[decision]` Aturan sekat input: mode input menelan semua keystroke;
  navigasi hanya via non-teks (PCC: `cmuim274d000004ig3uhitxkt`).
- `[convention]` Build/test inti: `./build.sh`, `cargo test`,
  `python3 -m unittest` (PCC: `cmufy3mo7000104lbxqvoze24`).
- Token tema: monokrom + Blue Iris `#5B5BD6` satu-satunya aksen.
- `tool.json` tak pernah ditulis tooling; overlay di `config/modules.json`.
- `config/history.json` runtime-state, gitignored.
