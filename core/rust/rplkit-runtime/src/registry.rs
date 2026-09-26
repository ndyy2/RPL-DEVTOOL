//! Registry — satu-satunya sumber daftar tool (Fase 1).
//!
//! Memindai `tools/*/tool.json` (skema v2: name, version, description,
//! exec chain, native op, entries). Tanpa mengeksekusi kode tool.

use std::fs;
use std::path::{Path, PathBuf};

/// Satu tool bermanifest beserta rantai eksekusinya.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolDef {
    pub name: String,
    pub version: String,
    pub description: String,
    /// Modul pemilik (`tool.json#module`), mis. "system-tools".
    pub module: String,
    /// Grup tampil TUI; default = module bila kosong.
    pub group: Option<String>,
    /// Rantai eksekusi, mis. ["native", "py-embed", "py-sys"].
    pub exec: Vec<String>,
    /// Op Rust untuk langkah "native", mis. "calc".
    pub native_op: Option<String>,
    /// Entry script relatif repo root, mis. "tools/calculator/impl.py".
    pub py_entry: Option<String>,
    pub ts_entry: Option<String>,
    pub dir: PathBuf,
}

impl ToolDef {
    pub fn group_name(&self) -> &str {
        self.group.as_deref().filter(|g| !g.is_empty()).unwrap_or(&self.module)
    }
}

fn string_field(text: &str, key: &str) -> Option<String> {
    rplkit_core::json_string_field(text, key).filter(|v| !v.is_empty())
}

/// Ekstrak array string JSON sederhana: `"key": ["a", "b"]`.
fn string_array_field(text: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{key}\"");
    let start = match text.find(&needle) {
        Some(i) => i + needle.len(),
        None => return Vec::new(),
    };
    let bytes = text.as_bytes();
    let mut i = start;
    while i < bytes.len() && bytes[i] != b'[' {
        if bytes[i] == b'}' {
            return Vec::new();
        }
        i += 1;
    }
    if i >= bytes.len() {
        return Vec::new();
    }
    i += 1;
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_str = false;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_str {
            if c == '\\' && i + 1 < bytes.len() {
                cur.push(bytes[i + 1] as char);
                i += 2;
                continue;
            }
            if c == '"' {
                in_str = false;
                out.push(std::mem::take(&mut cur));
            } else {
                cur.push(c);
            }
        } else if c == '"' {
            in_str = true;
        } else if c == ']' {
            break;
        }
        i += 1;
    }
    out
}

pub(crate) fn load_one(repo_root: &Path, manifest: &Path) -> Option<ToolDef> {
    let text = fs::read_to_string(manifest).ok()?;
    let name = string_field(&text, "name")?;
    let dir = manifest.parent()?.to_path_buf();
    // entries.python / entries.typescript dibaca sebagai sub-objek:
    // cari "entries" lalu field di dalamnya.
    let entries_block = text
        .find("\"entries\"")
        .map(|i| text[i..].to_string())
        .unwrap_or_default();
    let py_entry = string_field(&entries_block, "python").and_then(|rel| {
        let full = dir.join(&rel);
        full.is_file()
            .then(|| {
                full.strip_prefix(repo_root)
                    .ok()
                    .map(|p| p.to_string_lossy().into_owned())
            })
            .flatten()
    });
    let ts_entry = string_field(&entries_block, "typescript").and_then(|rel| {
        let full = dir.join(&rel);
        full.is_file()
            .then(|| {
                full.strip_prefix(repo_root)
                    .ok()
                    .map(|p| p.to_string_lossy().into_owned())
            })
            .flatten()
    });
    let native_block = text
        .find("\"native\"")
        .map(|i| text[i..].to_string())
        .unwrap_or_default();
    Some(ToolDef {
        name,
        version: string_field(&text, "version").unwrap_or_else(|| "0.0.0".into()),
        description: string_field(&text, "description").unwrap_or_default(),
        module: string_field(&text, "module").unwrap_or_else(|| "general".into()),
        group: string_field(&text, "group"),
        exec: string_array_field(&text, "exec"),
        native_op: string_field(&native_block, "op"),
        py_entry,
        ts_entry,
        dir,
    })
}

/// Semua tool bermanifest di `<repo>/tools/*/tool.json`, terurut nama.
/// Mentah (tanpa filter overlay) — untuk `--all` dan tooling.
pub fn discover(repo_root: &Path) -> Vec<ToolDef> {
    let mut out = Vec::new();
    let entries = match fs::read_dir(repo_root.join("tools")) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let manifest = entry.path().join("tool.json");
        if manifest.is_file() {
            if let Some(t) = load_one(repo_root, &manifest) {
                out.push(t);
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

pub fn find<'a>(tools: &'a [ToolDef], name: &str) -> Option<&'a ToolDef> {
    tools.iter().find(|t| t.name == name)
}

// ---------------------------------------------------------------------------
// Bentuk terkompilasi: discovery sekali, resolve sekali (Fase 1 optimasi).
//
// Alur lama per eksekusi: scan fs → parse → overlay → find linear →
// match string per langkah. Alur baru: Registry::build SEKALI per proses,
// lalu ToolHandle::execute tanpa parsing/lookup string apa pun.
// ---------------------------------------------------------------------------

use std::collections::HashMap;

/// Satu langkah eksekusi yang sudah di-resolve. Path absolut (repo
/// di-join saat build) sehingga executor tak lagi menyentuh makna string.
#[derive(Debug, Clone, PartialEq)]
pub enum Executor {
    Native(String),
    PyEmbed(PathBuf),
    PySys(PathBuf),
    JsEmbed(PathBuf),
    NodeSys(PathBuf),
    /// String exec tak dikenal — dipertahankan sebagai error runtime
    /// (perilaku lama: "unknown chain step"), bukan error build.
    Unknown(String),
}

impl Executor {
    /// Nama langkah untuk jejak verbose. 'static agar murah (tanpa alokasi).
    pub fn step_name(&self) -> &'static str {
        match self {
            Executor::Native(_) => "native",
            Executor::PyEmbed(_) => "py-embed",
            Executor::PySys(_) => "py-sys",
            Executor::JsEmbed(_) => "js-embed",
            Executor::NodeSys(_) => "node-sys",
            Executor::Unknown(_) => "unknown",
        }
    }
}

/// Tool terkompilasi: metadata tampil + rantai siap eksekusi.
#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub version: String,
    pub description: String,
    pub module: String,
    pub group: String,
    /// Label runtime untuk --list (aturan sama seperti FFI/bin).
    pub runtime_label: &'static str,
    /// Label entry untuk --list (path absolut, atau op id untuk native).
    pub entry_label: String,
    pub chain: Vec<Executor>,
}

/// Registry proses: HashMap nama→indeks + Vec tool. Dibangun SEKALI
/// per proses via build() (bukan per eksekusi).
#[derive(Debug, Default)]
pub struct Registry {
    map: HashMap<String, usize>,
    tools: Vec<Tool>,
    /// Repo root yang dipakai build (untuk pesan error).
    pub repo: PathBuf,
    /// Biaya discovery+kompilasi saat build (Fase 0 instrumentasi).
    pub build_ms: u128,
    /// True bila index file dipakai (penuh/parsial); false = full rescan.
    pub index_hit: bool,
    /// Manifest yang di-parse-ulang saat build ini.
    pub rescanned: usize,
}

/// Kompilasi rantai `tool.json#exec` menjadi [`Executor`].
/// Dipakai [`Registry::build`] dan wrapper kompatibilitas chain.
pub fn compile_chain(tool: &ToolDef, repo_root: &Path) -> Vec<Executor> {
    let steps: Vec<&str> = if !tool.exec.is_empty() {
        tool.exec.iter().map(|s| s.as_str()).collect()
    } else if tool.py_entry.is_some() {
        vec!["py-embed", "py-sys"]
    } else if tool.ts_entry.is_some() {
        vec!["js-embed", "node-sys"]
    } else {
        vec!["native"]
    };
    steps
        .into_iter()
        .map(|s| match s {
            "native" => Executor::Native(tool.native_op.clone().unwrap_or_default()),
            "py-embed" => Executor::PyEmbed(abs_entry(repo_root, tool.py_entry.as_deref())),
            "py-sys" => Executor::PySys(abs_entry(repo_root, tool.py_entry.as_deref())),
            "js-embed" => Executor::JsEmbed(abs_entry(repo_root, tool.ts_entry.as_deref())),
            "node-sys" => Executor::NodeSys(abs_entry(repo_root, tool.ts_entry.as_deref())),
            other => Executor::Unknown(other.to_string()),
        })
        .collect()
}

fn abs_entry(repo_root: &Path, rel: Option<&str>) -> PathBuf {
    match rel {
        Some(r) => repo_root.join(r),
        None => PathBuf::new(),
    }
}

impl Registry {
    /// Bangun registry dari manifest + overlay. Satu-satunya tempat
    /// scan filesystem + parse + kompilasi terjadi.
    pub fn build(repo_root: &Path) -> Self {
        Self::build_opts(repo_root, true)
    }

    /// Varian dengan opsi: `use_index=false` (`--dev`) selalu rescan.
    pub fn build_opts(repo_root: &Path, use_index: bool) -> Self {
        let t0 = std::time::Instant::now();
        let (raw, stats) = if use_index {
            crate::index::load_indexed(repo_root)
        } else {
            let defs = discover(repo_root);
            let n = defs.len();
            (
                defs,
                crate::index::IndexStats { used_index: false, rescanned: n, total: n },
            )
        };
        let defs = apply_overlay(raw, repo_root);
        let mut tools = Vec::with_capacity(defs.len());
        let mut map = HashMap::with_capacity(defs.len());
        for def in &defs {
            let (runtime_label, entry_label) = display_for(def, repo_root);
            map.insert(def.name.clone(), tools.len());
            tools.push(Tool {
                name: def.name.clone(),
                version: def.version.clone(),
                description: def.description.clone(),
                module: def.module.clone(),
                group: def.group_name().to_string(),
                runtime_label,
                entry_label,
                chain: compile_chain(def, repo_root),
            });
        }
        Self {
            map,
            tools,
            repo: repo_root.to_path_buf(),
            build_ms: t0.elapsed().as_millis(),
            index_hit: stats.used_index,
            rescanned: stats.rescanned,
        }
    }

    /// Registry kosong (untuk inisialisasi struct sebelum refresh).
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn tools(&self) -> &[Tool] {
        &self.tools
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Lookup O(1) — pengganti find() linear di jalur eksekusi.
    pub fn get(&self, name: &str) -> Option<&Tool> {
        self.map.get(name).and_then(|&i| self.tools.get(i))
    }

    /// Resolve sekali menjadi handle milik pemanggil (boleh disimpan,
    /// mis. TUI Exec view untuk live-preview).
    pub fn handle(&self, name: &str) -> Option<ToolHandle> {
        self.get(name).map(|t| ToolHandle {
            name: t.name.clone(),
            chain: t.chain.clone(),
        })
    }
}

/// Aturan label --list, SAMA dengan yang dipakai FFI/bin (paritas
/// by construction, bukan duplikasi aturan).
fn display_for(def: &ToolDef, repo_root: &Path) -> (&'static str, String) {
    if def.native_op.is_some() && def.py_entry.is_none() && def.ts_entry.is_none() {
        ("native", def.native_op.clone().unwrap_or_default())
    } else if let Some(e) = def.py_entry.as_deref() {
        ("python", repo_root.join(e).to_string_lossy().into_owned())
    } else if let Some(e) = def.ts_entry.as_deref() {
        ("typescript", repo_root.join(e).to_string_lossy().into_owned())
    } else if let Some(o) = def.native_op.clone() {
        ("native", o)
    } else {
        ("unknown", String::new())
    }
}

/// Handle tereksekusi: rantai milik sendiri, tanpa referensi ke Registry.
/// Aman disimpan lintas refresh (mis. TUI).
#[derive(Debug, Clone)]
pub struct ToolHandle {
    pub name: String,
    pub chain: Vec<Executor>,
}

impl ToolHandle {
    /// Eksekusi via chain terkompilasi. Semantik IDENTIK dengan
    /// run_tool_captured lama (termasuk aturan berhenti/fallback).
    pub fn execute(&self, args: &[String]) -> crate::chain::Captured {
        crate::chain::execute_compiled(&self.chain, args)
    }

    /// Varian passthrough (output ke stdout, untuk REPL/CLI).
    /// Semantik IDENTIK dengan run_tool lama.
    pub fn execute_passthrough(&self, args: &[String]) -> crate::chain::Outcome {
        crate::chain::execute_passthrough(&self.chain, args)
    }
}

/// Daftar tool yang AKTIF: overlay (`config/modules.json`) digabung di
/// atas default deskriptor (`modules/*.module.json`). Lihat `modules::`.
pub fn discover_enabled(repo_root: &Path) -> Vec<ToolDef> {
    apply_overlay(discover(repo_root), repo_root)
}

/// Terapkan filter overlay pada definisi yang sudah dimuat (tanpa scan
/// ulang) — dipakai jalur index.
pub fn apply_overlay(defs: Vec<ToolDef>, repo_root: &Path) -> Vec<ToolDef> {
    let descs = crate::modules::load_descriptors(repo_root);
    let overlay = crate::modules::load_overlay(repo_root);
    defs.into_iter()
        .filter(|t| {
            let desc = descs.iter().find(|d| d.name == t.module);
            match desc {
                Some(d) => crate::modules::is_enabled(d, &overlay, Some(&t.name)),
                None => {
                    // Modul tak dikenal: default aktif kecuali overlay mematikan.
                    crate::modules::is_enabled(
                        &crate::modules::ModuleDesc {
                            name: t.module.clone(),
                            version: String::new(),
                            author: String::new(),
                            description: String::new(),
                            runtime: String::new(),
                            enabled_by_default: true,
                        },
                        &overlay,
                        Some(&t.name),
                    )
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
    }

    #[test]
    fn discovers_nine_tools() {
        let tools = discover(&repo_root());
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        for expect in [
            "calculator",
            "converter",
            "encoder",
            "json_formatter",
            "uuid_generator",
            "timestamp",
            "qr_generator",
            "file_utils",
            "hello",
        ] {
            assert!(names.contains(&expect), "missing {expect} in {names:?}");
        }
    }

    #[test]
    fn chains_match_plan() {
        let tools = discover(&repo_root());
        let calc = find(&tools, "calculator").expect("calc");
        assert_eq!(calc.exec, vec!["native", "py-embed", "py-sys"]);
        assert_eq!(calc.native_op.as_deref(), Some("calc"));
        assert!(calc.py_entry.as_deref().unwrap().ends_with("impl.py"));
        let qr = find(&tools, "qr_generator").expect("qr");
        assert_eq!(qr.exec, vec!["py-embed", "py-sys"]);
        assert!(qr.native_op.is_none());
        let hello = find(&tools, "hello").expect("hello");
        assert_eq!(hello.exec, vec!["js-embed", "node-sys"]);
        assert!(hello.ts_entry.as_deref().unwrap().ends_with("impl.ts"));
    }

    #[test]
    fn tools_carry_module_names() {
        let tools = discover(&repo_root());
        let calc = find(&tools, "calculator").expect("calc");
        assert_eq!(calc.module, "dev-tools");
        assert_eq!(calc.group_name(), "dev-tools");
    }

    #[test]
    fn registry_build_resolves_all() {
        let reg = Registry::build(&repo_root());
        assert_eq!(reg.len(), discover_enabled(&repo_root()).len());
        assert!(reg.len() >= 18);
        // Lookup O(1) + label paritas dengan --list lama.
        let calc = reg.get("calculator").expect("calc");
        assert_eq!(calc.runtime_label, "python");
        assert!(calc.entry_label.ends_with("impl.py"));
        assert!(reg.get("tidak-ada").is_none());
    }

    #[test]
    fn handle_executes_without_rescan() {
        let reg = Registry::build(&repo_root());
        let h = reg.handle("calculator").expect("handle");
        assert_eq!(h.name, "calculator");
        assert!(matches!(h.chain.first(), Some(Executor::Native(op)) if op == "calc"));
        let c = h.execute(&["6 * 7".to_string()]);
        assert_eq!(c.code, 0);
        assert!(c.output.contains("42"), "{}", c.output);
        assert_eq!(c.attempts.len(), 1);
        assert_eq!(c.attempts[0].step, "native");
        assert!(c.attempts[0].ok);
        assert!(reg.handle("tidak-ada").is_none());
    }

    #[test]
    fn unknown_exec_string_preserved_as_runtime_error() {
        let def = ToolDef {
            name: "weird".into(),
            version: "1".into(),
            description: "t".into(),
            module: "m".into(),
            group: None,
            exec: vec!["bogus-step".into()],
            native_op: None,
            py_entry: None,
            ts_entry: None,
            dir: PathBuf::new(),
        };
        let chain = compile_chain(&def, Path::new("/x"));
        assert_eq!(chain, vec![Executor::Unknown("bogus-step".into())]);
        let c = crate::chain::execute_compiled(&chain, &[]);
        assert_eq!(c.code, 1);
        assert!(c.output.contains("unknown chain step"), "{}", c.output);
        assert_eq!(c.attempts.len(), 1);
        assert!(!c.attempts[0].ok);
    }

    fn temp_tool_repo() -> PathBuf {
        let repo = std::env::temp_dir().join(format!("rplkit-regtest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&repo);
        for tool in ["tool-a", "tool-b"] {
            let dir = repo.join("tools").join(tool);
            std::fs::create_dir_all(&dir).unwrap();
            let module = if tool == "tool-a" { "m1" } else { "m2" };
            std::fs::write(
                dir.join("tool.json"),
                format!(
                    "{{\"name\": \"{tool}\", \"version\": \"1\", \"description\": \"t\", \
                     \"module\": \"{module}\", \"exec\": [\"native\"], \
                     \"native\": {{\"op\": \"{tool}\"}}}}"
                ),
            )
            .unwrap();
        }
        let mdir = repo.join("modules");
        std::fs::create_dir_all(&mdir).unwrap();
        for m in ["m1", "m2"] {
            std::fs::write(
                mdir.join(format!("{m}.module.json")),
                format!(
                    "{{\"name\": \"{m}\", \"version\": \"1\", \"description\": \"d\", \"runtime\": \"rust\"}}"
                ),
            )
            .unwrap();
        }
        repo
    }

    #[test]
    fn discover_enabled_honors_overlay() {
        let repo = temp_tool_repo();
        // Tanpa overlay: keduanya aktif (default deskriptor).
        assert_eq!(discover_enabled(&repo).len(), 2);
        crate::modules::set_module_enabled(&repo, "m2", false).unwrap();
        let names: Vec<String> = discover_enabled(&repo).iter().map(|t| t.name.clone()).collect();
        assert_eq!(names, vec!["tool-a".to_string()]);
        // Override per tool mengalahkan modul.
        crate::modules::set_tool_enabled(&repo, "tool-b", true).unwrap();
        assert_eq!(discover_enabled(&repo).len(), 2);
        let _ = std::fs::remove_dir_all(&repo);
    }
}
