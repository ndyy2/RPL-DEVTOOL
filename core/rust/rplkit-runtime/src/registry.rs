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

fn load_one(repo_root: &Path, manifest: &Path) -> Option<ToolDef> {
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

/// Daftar tool yang AKTIF: overlay (`config/modules.json`) digabung di
/// atas default deskriptor (`modules/*.module.json`). Lihat `modules::`.
pub fn discover_enabled(repo_root: &Path) -> Vec<ToolDef> {
    let descs = crate::modules::load_descriptors(repo_root);
    let overlay = crate::modules::load_overlay(repo_root);
    discover(repo_root)
        .into_iter()
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
