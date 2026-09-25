//! modules — deskriptor modul + overlay enable/disable (Fase 0 kontrak).
//!
//! Sumber: `modules/*.module.json` (statis) + `config/modules.json`
//! (overlay, boleh absen). `tool.json` tidak pernah ditulis tooling.
//! Penggabungan penuh dengan registry tool menyusul Fase 1.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Deskriptor satu modul (`modules/<nama>.module.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDesc {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub runtime: String,
    pub enabled_by_default: bool,
}

/// Overlay (`config/modules.json`): enabled per modul + override per tool.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Overlay {
    pub modules: HashMap<String, bool>,
    pub tools: HashMap<String, bool>,
}

fn req_string(text: &str, key: &str) -> Option<String> {
    rplkit_core::json_string_field(text, key).filter(|v| !v.is_empty())
}

fn req_bool(text: &str, key: &str) -> Option<bool> {
    let needle = format!("\"{key}\"");
    let i = text.find(&needle)? + needle.len();
    let after_colon = text[i..].split(':').nth(1)?.trim_start();
    if after_colon.starts_with("true") {
        Some(true)
    } else if after_colon.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn parse_desc(text: &str) -> Option<ModuleDesc> {
    Some(ModuleDesc {
        name: req_string(text, "name")?,
        version: req_string(text, "version").unwrap_or_else(|| "0.0.0".into()),
        author: req_string(text, "author").unwrap_or_else(|| "RPLKit".into()),
        description: req_string(text, "description").unwrap_or_default(),
        runtime: req_string(text, "runtime").unwrap_or_else(|| "mixed".into()),
        enabled_by_default: req_bool(text, "enabled_by_default").unwrap_or(true),
    })
}

/// Muat semua `modules/*.module.json`, terurut nama.
pub fn load_descriptors(repo_root: &Path) -> Vec<ModuleDesc> {
    let mut out = Vec::new();
    let entries = match fs::read_dir(repo_root.join("modules")) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let is_desc = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(".module.json"))
            .unwrap_or(false);
        if !is_desc {
            continue;
        }
        if let Ok(text) = fs::read_to_string(&path) {
            if let Some(d) = parse_desc(&text) {
                out.push(d);
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Muat overlay; absen/rusak = overlay kosong (semua default).
pub fn load_overlay(repo_root: &Path) -> Overlay {
    let path = repo_root.join("config").join("modules.json");
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return Overlay::default(),
    };
    Overlay {
        modules: bool_map(&text, "modules"),
        tools: bool_map(&text, "tools"),
    }
}

/// Parse `"section": {"name": {"enabled": bool}}` sederhana.
fn bool_map(text: &str, section: &str) -> HashMap<String, bool> {
    let mut map = HashMap::new();
    let needle = format!("\"{section}\"");
    let start = match text.find(&needle) {
        Some(i) => i + needle.len(),
        None => return map,
    };
    let body = &text[start..];
    // Cari pasangan "nama": { ... "enabled": bool ... } di dalam section.
    let mut i = 0;
    let bytes = body.as_bytes();
    // Lewati hingga '{' pertama section.
    while i < bytes.len() && bytes[i] != b'{' {
        if bytes[i] == b'}' {
            return map;
        }
        i += 1;
    }
    let mut depth = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            b'"' => {
                // Baca string; bila diikuti '{', itu nama entri.
                let mut j = i + 1;
                let mut name = String::new();
                while j < bytes.len() && bytes[j] != b'"' {
                    if bytes[j] == b'\\' {
                        j += 1;
                    } else {
                        name.push(bytes[j] as char);
                    }
                    j += 1;
                }
                let mut k = j + 1;
                while k < bytes.len() && ((bytes[k] as char).is_whitespace() || bytes[k] == b':') {
                    k += 1;
                }
                if k < bytes.len() && bytes[k] == b'{' {
                    // Cari "enabled" sebelum brace seimbang berikutnya.
                    let mut m = k;
                    let mut d = 0usize;
                    let mut val: Option<bool> = None;
                    while m < bytes.len() {
                        if bytes[m] == b'{' {
                            d += 1;
                        } else if bytes[m] == b'}' {
                            d -= 1;
                            if d == 0 {
                                break;
                            }
                        } else if body[m..].starts_with("\"enabled\"") {
                            let tail = body[m + 9..].trim_start_matches([' ', '\t', '\n', '\r', ':']);
                            let tail = tail.strip_prefix(':').unwrap_or(tail).trim_start();
                            if tail.starts_with("true") {
                                val = Some(true);
                            } else if tail.starts_with("false") {
                                val = Some(false);
                            }
                            break;
                        }
                        m += 1;
                    }
                    if let Some(v) = val {
                        map.insert(name.clone(), v);
                    }
                    i = m;
                    continue;
                }
                i = j;
            }
            _ => {}
        }
        i += 1;
    }
    map
}

/// Status efektif: override tool > overlay modul > default deskriptor.
pub fn is_enabled(desc: &ModuleDesc, overlay: &Overlay, tool: Option<&str>) -> bool {
    if let Some(t) = tool {
        if let Some(v) = overlay.tools.get(t) {
            return *v;
        }
    }
    overlay
        .modules
        .get(&desc.name)
        .copied()
        .unwrap_or(desc.enabled_by_default)
}

fn escape_key(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        if c == '"' || c == '\\' {
            o.push('\\');
        }
        o.push(c);
    }
    o
}

/// Tulis overlay kembali kanonis (kunci terurut). Membuat `config/` bila perlu.
pub fn write_overlay(repo_root: &Path, overlay: &Overlay) -> Result<(), String> {
    let mut s = String::from(
        "{\n  \"_comment\": \"Overlay enable/disable. tool.json tidak pernah disentuh oleh tooling.\",\n",
    );
    s.push_str("  \"modules\": {");
    let mut mk: Vec<&String> = overlay.modules.keys().collect();
    mk.sort();
    for (i, k) in mk.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "\n    \"{}\": {{\"enabled\": {}}}",
            escape_key(k),
            overlay.modules[*k]
        ));
    }
    s.push_str(if mk.is_empty() { "}, " } else { "\n  }, " });
    s.push_str("\"tools\": {");
    let mut tk: Vec<&String> = overlay.tools.keys().collect();
    tk.sort();
    for (i, k) in tk.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "\n    \"{}\": {{\"enabled\": {}}}",
            escape_key(k),
            overlay.tools[*k]
        ));
    }
    s.push_str(if tk.is_empty() { "}\n}\n" } else { "\n  }\n}\n" });
    let dir = repo_root.join("config");
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create config/: {e}"))?;
    fs::write(dir.join("modules.json"), s).map_err(|e| format!("cannot write overlay: {e}"))
}

/// Ubah status modul (tambah entri bila belum ada).
pub fn set_module_enabled(repo_root: &Path, name: &str, enabled: bool) -> Result<(), String> {
    let mut overlay = load_overlay(repo_root);
    overlay.modules.insert(name.to_string(), enabled);
    write_overlay(repo_root, &overlay)
}

/// Override status satu tool (menang atas status modulnya).
pub fn set_tool_enabled(repo_root: &Path, name: &str, enabled: bool) -> Result<(), String> {
    let mut overlay = load_overlay(repo_root);
    overlay.tools.insert(name.to_string(), enabled);
    write_overlay(repo_root, &overlay)
}

pub fn repo_fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_four_descriptors() {
        let descs = load_descriptors(&repo_fixtures());
        let names: Vec<&str> = descs.iter().map(|d| d.name.as_str()).collect();
        for expect in ["system-tools", "network-tools", "git-tools", "school-tools"] {
            assert!(names.contains(&expect), "missing {expect} in {names:?}");
        }
        let school = descs.iter().find(|d| d.name == "school-tools").unwrap();
        assert!(!school.enabled_by_default);
        assert_eq!(school.version, "0.8.1");
    }

    #[test]
    fn overlay_disables_school() {
        let descs = load_descriptors(&repo_fixtures());
        let overlay = load_overlay(&repo_fixtures());
        assert_eq!(overlay.modules.get("school-tools"), Some(&false));
        for d in &descs {
            let enabled = is_enabled(d, &overlay, None);
            assert_eq!(enabled, d.name != "school-tools", "{}", d.name);
        }
    }

    #[test]
    fn missing_overlay_means_defaults() {
        let overlay = load_overlay(Path::new("/tidak/ada"));
        assert!(overlay.modules.is_empty() && overlay.tools.is_empty());
    }

    #[test]
    fn tool_override_beats_module() {
        let desc = ModuleDesc {
            name: "system-tools".into(),
            version: "x".into(),
            author: "x".into(),
            description: "x".into(),
            runtime: "rust".into(),
            enabled_by_default: true,
        };
        let mut overlay = Overlay::default();
        overlay.tools.insert("system-info".into(), false);
        assert!(!is_enabled(&desc, &overlay, Some("system-info")));
        assert!(is_enabled(&desc, &overlay, Some("lainnya")));
    }

    fn temp_repo(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rplkit-modtest-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("config")).unwrap();
        dir
    }

    #[test]
    fn overlay_roundtrip_preserves_entries() {
        let repo = temp_repo("roundtrip");
        set_module_enabled(&repo, "git-tools", false).unwrap();
        set_tool_enabled(&repo, "qr_generator", true).unwrap();
        let re = load_overlay(&repo);
        assert_eq!(re.modules.get("git-tools"), Some(&false));
        assert_eq!(re.tools.get("qr_generator"), Some(&true));
        // Tulis lagi tanpa perubahan: isi stabil (kanonis).
        let before = std::fs::read_to_string(repo.join("config").join("modules.json")).unwrap();
        write_overlay(&repo, &re).unwrap();
        let after = std::fs::read_to_string(repo.join("config").join("modules.json")).unwrap();
        assert_eq!(before, after);
        let _ = std::fs::remove_dir_all(&repo);
    }
}
