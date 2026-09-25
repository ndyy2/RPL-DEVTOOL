//! history — RECENT + Logs persistence (`config/history.json`).
//!
//! Format: `{"runs": [{tool, args, code, ms, by, at}]}`. Cap 50,
//! terbaru dulu. File dibuat saat runtime; absen = riwayat kosong.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Satu baris riwayat eksekusi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunRecord {
    pub tool: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub code: i32,
    pub ms: u128,
    #[serde(default)]
    pub by: String,
    pub at: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct HistoryFile {
    #[serde(default)]
    runs: Vec<RunRecord>,
}

pub fn history_path(repo_root: &Path) -> PathBuf {
    repo_root.join("config").join("history.json")
}

/// Muat riwayat (terbaru dulu). Gagal = kosong, tak pernah melempar.
pub fn load(repo_root: &Path) -> Vec<RunRecord> {
    let text = match std::fs::read_to_string(history_path(repo_root)) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str::<HistoryFile>(&text)
        .map(|h| h.runs)
        .unwrap_or_default()
}

/// Tambah satu run ke depan, cap 50. Gagal tulis = diabaikan (non-fatal).
pub fn append(repo_root: &Path, record: RunRecord) {
    let mut runs = load(repo_root);
    runs.insert(0, record);
    runs.truncate(50);
    let file = HistoryFile { runs };
    if let Ok(text) = serde_json::to_string_pretty(&file) {
        let _ = std::fs::create_dir_all(repo_root.join("config"));
        let _ = std::fs::write(history_path(repo_root), text);
    }
}

pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// "2s ago" / "1m ago" / "8m ago" untuk panel RECENT.
pub fn ago_text(now: u64, at: u64) -> String {
    let d = now.saturating_sub(at);
    if d < 60 {
        format!("{d}s ago")
    } else if d < 3600 {
        format!("{}m ago", d / 60)
    } else if d < 86400 {
        format!("{}h ago", d / 3600)
    } else {
        format!("{}d ago", d / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_repo(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rplkit-hist-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn rec(tool: &str) -> RunRecord {
        RunRecord { tool: tool.into(), args: vec![], code: 0, ms: 3, by: "native".into(), at: 1 }
    }

    #[test]
    fn append_load_roundtrip_capped() {
        let repo = temp_repo("roundtrip");
        for i in 0..55 {
            append(&repo, rec(&format!("tool-{i}")));
        }
        let runs = load(&repo);
        assert_eq!(runs.len(), 50);
        assert_eq!(runs[0].tool, "tool-54");
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn missing_file_is_empty() {
        assert!(load(Path::new("/tidak/ada")).is_empty());
    }

    #[test]
    fn ago_text_vectors() {
        assert_eq!(ago_text(100, 98), "2s ago");
        assert_eq!(ago_text(1000, 940), "1m ago");
        assert_eq!(ago_text(10000, 2800), "2h ago");
    }
}
