//! index — Manifest Index: discovery sekali, rescan parsial (Fase 2).
//!
//! `tools/*/tool.json` relatif statis; mem-parse semuanya tiap proses
//! adalah kerja berulang. Index (`.cache/tools.idx`, gitignored)
//! menyimpan hasil parse + hash per manifest:
//!
//! - startup: cocokkan mtime (stat saja); hanya manifest berubah yang
//!   dibaca-ulang (hash) dan di-parse-ulang (parsial, bukan full rescan);
//! - index hilang/rusak/versi beda → full rescan + tulis ulang (best-effort);
//! - `rplkit tools build-index` memaksa tulis ulang eksplisit;
//! - `--dev` melewatkan index (selalu rescan + diagnostik).
//!
//! Implementasi lazy tetap: index hanya menyimpan METADATA, tak pernah
//! meng-import/men-init implementasi tool.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::registry::ToolDef;

const INDEX_VERSION: u32 = 1;

fn index_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".cache").join("tools.idx")
}

/// FNV-1a 64 (stdlib-only; cukup untuk deteksi perubahan, bukan keamanan).
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn file_mtime_nanos(path: &Path) -> Option<u128> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_nanos())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolEntry {
    name: String,
    version: String,
    description: String,
    module: String,
    group: Option<String>,
    exec: Vec<String>,
    native_op: Option<String>,
    py_entry: Option<String>,
    ts_entry: Option<String>,
    /// Direktori tool relatif repo root (untuk deteksi hapus).
    dir_rel: String,
    manifest_hash: u64,
    manifest_mtime: u128,
}

#[derive(Debug, Serialize, Deserialize)]
struct IndexFile {
    version: u32,
    generated: u64,
    tools: HashMap<String, ToolEntry>,
}

fn entry_from_def(def: &ToolDef, repo_root: &Path, hash: u64, mtime: u128) -> ToolEntry {
    ToolEntry {
        name: def.name.clone(),
        version: def.version.clone(),
        description: def.description.clone(),
        module: def.module.clone(),
        group: def.group.clone(),
        exec: def.exec.clone(),
        native_op: def.native_op.clone(),
        py_entry: def.py_entry.clone(),
        ts_entry: def.ts_entry.clone(),
        dir_rel: def
            .dir
            .strip_prefix(repo_root)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
        manifest_hash: hash,
        manifest_mtime: mtime,
    }
}

fn def_from_entry(e: &ToolEntry, repo_root: &Path) -> ToolDef {
    ToolDef {
        name: e.name.clone(),
        version: e.version.clone(),
        description: e.description.clone(),
        module: e.module.clone(),
        group: e.group.clone(),
        exec: e.exec.clone(),
        native_op: e.native_op.clone(),
        py_entry: e.py_entry.clone(),
        ts_entry: e.ts_entry.clone(),
        dir: if e.dir_rel.is_empty() {
            repo_root.join("tools").join(&e.name)
        } else {
            repo_root.join(&e.dir_rel)
        },
    }
}

/// Statistik pemuatan index (untuk verbose + test).
#[derive(Debug, Clone, PartialEq)]
pub struct IndexStats {
    /// true bila index dipakai (penuh atau parsial).
    pub used_index: bool,
    /// Jumlah manifest yang di-parse-ulang kali ini.
    pub rescanned: usize,
    pub total: usize,
}

/// Muat definisi tool mentah (TANPA filter overlay) via index bila valid.
/// Selalu benar: jatuh ke rescan (parsial/penuh) bila ada keraguan.
pub fn load_indexed(repo_root: &Path) -> (Vec<ToolDef>, IndexStats) {
    let manifest_of = |dir: &Path| dir.join("tool.json");
    // Daftar direktori tool saat ini (readdir murah, tanpa parse).
    let mut current: HashMap<String, PathBuf> = HashMap::new();
    if let Ok(entries) = std::fs::read_dir(repo_root.join("tools")) {
        for entry in entries.flatten() {
            let dir = entry.path();
            if dir.is_dir() && manifest_of(&dir).is_file() {
                if let Some(name) = dir.file_name().and_then(|n| n.to_str()) {
                    current.insert(name.to_string(), dir);
                }
            }
        }
    }

    let cached: Option<IndexFile> = std::fs::read_to_string(index_path(repo_root))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .filter(|idx: &IndexFile| idx.version == INDEX_VERSION);

    let Some(idx) = cached else {
        // Tanpa index: full rescan + tulis (best-effort).
        let defs = super::registry::discover(repo_root);
        let stats = IndexStats { used_index: false, rescanned: defs.len(), total: defs.len() };
        persist(repo_root, &defs);
        return (defs, stats);
    };

    let mut defs = Vec::new();
    let mut rescanned = 0usize;
    let mut touched = false;
    let mut fresh: HashMap<String, ToolEntry> = HashMap::new();
    for (dirname, dir) in &current {
        let manifest = manifest_of(dir);
        let mtime = file_mtime_nanos(&manifest).unwrap_or(0);
        let hit = idx.tools.get(dirname).filter(|e| e.manifest_mtime == mtime);
        if let Some(e) = hit {
            defs.push(def_from_entry(e, repo_root));
            fresh.insert(dirname.clone(), e.clone());
            continue;
        }
        // mtime beda/hilang: baca + hash; hash sama → pakai cache.
        let bytes = std::fs::read(&manifest).unwrap_or_default();
        let hash = fnv1a(&bytes);
        if let Some(e) = idx.tools.get(dirname).filter(|e| e.manifest_hash == hash) {
            let mut e2 = e.clone();
            if e2.manifest_mtime != mtime {
                e2.manifest_mtime = mtime;
                touched = true;
            }
            defs.push(def_from_entry(&e2, repo_root));
            fresh.insert(dirname.clone(), e2);
            continue;
        }
        // Parsial: parse ulang manifest ini saja.
        rescanned += 1;
        if let Some(d) = super::registry::load_one(repo_root, &manifest) {
            fresh.insert(
                dirname.clone(),
                entry_from_def(&d, repo_root, hash, mtime),
            );
            defs.push(d);
        }
    }
    // Hapus entri tool yang direktorinya sudah tak ada (implisit: tak masuk fresh).
    let stats = IndexStats { used_index: true, rescanned, total: defs.len() };
    if rescanned > 0 || touched || fresh.len() != idx.tools.len() {
        persist_entries(repo_root, fresh);
    }
    (defs, stats)
}

/// Tulis ulang index dari hasil scan penuh. Dipakai CLI `tools build-index`.
pub fn build_index(repo_root: &Path) -> Result<usize, String> {
    let defs = super::registry::discover(repo_root);
    persist(repo_root, &defs);
    Ok(defs.len())
}

fn persist(repo_root: &Path, defs: &[ToolDef]) {
    let mut tools = HashMap::new();
    for d in defs {
        let manifest = d.dir.join("tool.json");
        let bytes = std::fs::read(&manifest).unwrap_or_default();
        tools.insert(
            d.dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&d.name)
                .to_string(),
            entry_from_def(d, repo_root, fnv1a(&bytes), file_mtime_nanos(&manifest).unwrap_or(0)),
        );
    }
    persist_entries(repo_root, tools);
}

fn persist_entries(repo_root: &Path, tools: HashMap<String, ToolEntry>) {
    let idx = IndexFile {
        version: INDEX_VERSION,
        generated: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        tools,
    };
    if let Ok(text) = serde_json::to_string_pretty(&idx) {
        let _ = std::fs::create_dir_all(repo_root.join(".cache"));
        let _ = std::fs::write(index_path(repo_root), text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_repo(tag: &str) -> PathBuf {
        let repo = std::env::temp_dir().join(format!("rplkit-idxtest-{}", tag));
        let _ = std::fs::remove_dir_all(&repo);
        std::fs::create_dir_all(repo.join("tools")).unwrap();
        std::fs::create_dir_all(repo.join("modules")).unwrap();
        repo
    }

    fn write_tool(repo: &Path, name: &str, desc: &str) {
        let dir = repo.join("tools").join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("tool.json"),
            format!(
                "{{\"name\": \"{name}\", \"version\": \"1\", \"description\": \"{desc}\", \
                 \"module\": \"m\", \"exec\": [\"native\"], \"native\": {{\"op\": \"calc\"}}}}"
            ),
        )
        .unwrap();
    }

    #[test]
    fn second_load_uses_index_without_rescan() {
        let repo = temp_repo("hit");
        write_tool(&repo, "tool-a", "A");
        write_tool(&repo, "tool-b", "B");
        let (first, s1) = load_indexed(&repo);
        assert!(!s1.used_index);
        assert_eq!(s1.rescanned, 2);
        assert!(index_path(&repo).is_file());
        let (second, s2) = load_indexed(&repo);
        assert!(s2.used_index);
        assert_eq!(s2.rescanned, 0);
        assert_eq!(second.len(), 2);
        assert_eq!(first.len(), second.len());
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn changed_manifest_rescans_partially() {
        let repo = temp_repo("partial");
        write_tool(&repo, "tool-a", "A");
        write_tool(&repo, "tool-b", "B");
        let _ = load_indexed(&repo);
        // Ubah SATU manifest (pastikan mtime beda: tulis ulang + konten beda).
        std::thread::sleep(std::time::Duration::from_millis(1100));
        write_tool(&repo, "tool-b", "B-CHANGED");
        let (defs, stats) = load_indexed(&repo);
        assert!(stats.used_index);
        assert_eq!(stats.rescanned, 1, "{stats:?}");
        let b = defs.iter().find(|d| d.name == "tool-b").unwrap();
        assert_eq!(b.description, "B-CHANGED");
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn corrupt_index_falls_back_to_full_rescan() {
        let repo = temp_repo("corrupt");
        write_tool(&repo, "tool-a", "A");
        let _ = load_indexed(&repo);
        std::fs::write(index_path(&repo), "{bukan json").unwrap();
        let (defs, stats) = load_indexed(&repo);
        assert!(!stats.used_index);
        assert_eq!(defs.len(), 1);
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn removed_tool_dir_disappears() {
        let repo = temp_repo("removed");
        write_tool(&repo, "tool-a", "A");
        write_tool(&repo, "tool-b", "B");
        let _ = load_indexed(&repo);
        std::fs::remove_dir_all(repo.join("tools").join("tool-b")).unwrap();
        let (defs, stats) = load_indexed(&repo);
        assert!(stats.used_index);
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "tool-a");
        let _ = std::fs::remove_dir_all(&repo);
    }
}
