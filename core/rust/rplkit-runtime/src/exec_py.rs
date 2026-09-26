//! exec_py — jalankan `impl.py` tool secara embedded (PyO3, Fase 3).
//!
//! Kontrak tool: `main(argv: list[str]) -> int`, argv = argumen tool
//! TANPA nama prog (konsisten dengan cabang `sys.argv[1:]` di impl.py).
//! Output lewat stdout asli (passthrough, identik dengan subprocess).
//!
//! Gagal inisialisasi/panic → `EmbedError::Runner` (pemicu fallback
//! `py-sys` di chain). `main` return nonzero / exception →
//! `EmbedError::ToolFailed` (rantai berhenti, itu salah tool).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use pyo3::prelude::*;
use pyo3::types::PyModule;

/// Cache modul per proses: path absolut → (modul, mtime_nanos saat import).
/// Hit: panggil `main` langsung tanpa re-parse/re-exec file.
/// mtime beda: import ulang otomatis (aman untuk `--dev`/edit live).
/// `Py<T>` GIL-independent sehingga aman dibagikan antar attach.
static MODULE_CACHE: Mutex<Option<HashMap<PathBuf, (Py<PyModule>, u128)>>> =
    Mutex::new(None);

fn file_mtime_nanos(path: &Path) -> Option<u128> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_nanos())
}

/// Jumlah entri cache (untuk test + diagnostik).
#[allow(dead_code)]
pub fn cache_len() -> usize {
    MODULE_CACHE
        .lock()
        .map(|g| g.as_ref().map(|m| m.len()).unwrap_or(0))
        .unwrap_or(0)
}

#[derive(Debug)]
pub enum EmbedError {
    Runner(String),
    ToolFailed(i32),
}

impl std::fmt::Display for EmbedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbedError::Runner(m) => write!(f, "python runner unavailable: {m}"),
            EmbedError::ToolFailed(c) => write!(f, "tool failed with code {c}"),
        }
    }
}

/// Jalankan file tool embedded. RZ: seluruh panic PyO3 ditangkap pemanggil
/// (chain) via catch_unwind — fungsi ini sendiri tidak mem-panic-kan selain
/// bug internal.
pub fn run_embedded(py_file: &Path, argv: &[String]) -> Result<i32, EmbedError> {
    run_embedded_inner(py_file, argv, false).map(|(c, _)| c)
}

/// Varian capture untuk TUI: stdout+stderr tool dialihkan ke StringIO dan
/// dikembalikan (tidak bocor ke terminal). Selalu restore stream.
pub fn run_embedded_captured(
    py_file: &Path,
    argv: &[String],
) -> Result<(i32, String), EmbedError> {
    run_embedded_inner(py_file, argv, true)
}

fn run_embedded_inner(
    py_file: &Path,
    argv: &[String],
    capture: bool,
) -> Result<(i32, String), EmbedError> {
    // Stat dulu (tanpa GIL): hilang → Runner error; mtime untuk cache.
    let abspath: PathBuf = std::fs::canonicalize(py_file).unwrap_or_else(|_| py_file.to_path_buf());
    let mtime = file_mtime_nanos(&abspath)
        .ok_or_else(|| EmbedError::Runner(format!("cannot read {}: missing", py_file.display())))?;
    let stem = abspath
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("tool");
    let modname = format!("rplkit_tool_{stem}");
    let path_s = abspath.to_string_lossy().into_owned();
    let argv: Vec<String> = argv.to_vec();

    Python::attach(|py| -> Result<(i32, String), EmbedError> {
        // sys.path[0] = dir tool agar import relatif/saudara tetap jalan.
        let sys = py.import("sys").map_err(|e| EmbedError::Runner(e.to_string()))?;
        if let Some(parent) = abspath.parent().and_then(|p| p.to_str()) {
            let path = sys.getattr("path").map_err(|e| EmbedError::Runner(e.to_string()))?;
            let _ = path.call_method1("insert", (0, parent));
        }
        // Redirect opsional untuk mode capture.
        let io = py.import("io").map_err(|e| EmbedError::Runner(e.to_string()))?;
        let buf_out = io
            .call_method0("StringIO")
            .map_err(|e| EmbedError::Runner(e.to_string()))?;
        let buf_err = io
            .call_method0("StringIO")
            .map_err(|e| EmbedError::Runner(e.to_string()))?;
        let (old_out, old_err) = if capture {
            let o = sys.getattr("stdout").map_err(|e| EmbedError::Runner(e.to_string()))?;
            let e = sys.getattr("stderr").map_err(|e| EmbedError::Runner(e.to_string()))?;
            sys.setattr("stdout", &buf_out)
                .map_err(|e| EmbedError::Runner(e.to_string()))?;
            sys.setattr("stderr", &buf_err)
                .map_err(|e| EmbedError::Runner(e.to_string()))?;
            (Some(o), Some(e))
        } else {
            (None, None)
        };

        // Modul: cache hit (mtime sama) atau import (sekali per perubahan).
        let module: Py<PyModule> = {
            let guard = MODULE_CACHE
                .lock()
                .map_err(|_| EmbedError::Runner("module cache poisoned".into()))?;
            if let Some(hit) = guard.as_ref().and_then(|m| m.get(&abspath)).filter(|(_, t)| *t == mtime) {
                hit.0.clone_ref(py)
            } else {
                drop(guard);
                let code = std::fs::read_to_string(&abspath)
                    .map_err(|e| EmbedError::Runner(format!("cannot read {}: {e}", py_file.display())))?;
                let code_c = std::ffi::CString::new(code)
                    .map_err(|e| EmbedError::Runner(e.to_string()))?;
                let path_c = std::ffi::CString::new(path_s)
                    .map_err(|e| EmbedError::Runner(e.to_string()))?;
                let name_c = std::ffi::CString::new(modname)
                    .map_err(|e| EmbedError::Runner(e.to_string()))?;
                let module: Bound<'_, PyModule> =
                    PyModule::from_code(py, &code_c, &path_c, &name_c)
                        .map_err(|e| EmbedError::Runner(format!("load failed: {e}")))?;
                let mut guard = MODULE_CACHE
                    .lock()
                    .map_err(|_| EmbedError::Runner("module cache poisoned".into()))?;
                let handle: Py<PyModule> = module.clone().into();
                guard.get_or_insert_with(HashMap::new).insert(abspath.clone(), (module.into(), mtime));
                handle
            }
        };
        let result = (|| -> Result<i32, EmbedError> {
            let main = module
                .bind(py)
                .getattr("main")
                .map_err(|e| EmbedError::Runner(format!("no main(argv): {e}")))?;
            match main.call1((argv,)) {
                Ok(v) => v.extract().map_err(|e| EmbedError::Runner(format!("bad return: {e}"))),
                Err(e) => {
                    e.print(py);
                    Err(EmbedError::ToolFailed(1))
                }
            }
            .and_then(|c: i32| {
                if c == 0 {
                    Ok(c)
                } else {
                    Err(EmbedError::ToolFailed(c))
                }
            })
        })();

        // Restore stream SEBELUM return, di semua jalur.
        let mut captured = String::new();
        if capture {
            let out: String = buf_out
                .call_method0("getvalue")
                .and_then(|v| v.extract())
                .unwrap_or_default();
            let err: String = buf_err
                .call_method0("getvalue")
                .and_then(|v| v.extract())
                .unwrap_or_default();
            captured.push_str(&out);
            captured.push_str(&err);
            if let (Some(o), Some(e)) = (old_out, old_err) {
                let _ = sys.setattr("stdout", o);
                let _ = sys.setattr("stderr", e);
            }
        }
        result.map(|c| (c, captured))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tools_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("tools")
    }

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn embedded_runs_calculator_main() {
        let f = tools_dir().join("calculator").join("impl.py");
        let code = run_embedded(&f, &s(&["6 * 7"])).expect("embed must run");
        assert_eq!(code, 0);
    }

    #[test]
    fn embedded_tool_error_is_not_runner_error() {
        let f = tools_dir().join("calculator").join("impl.py");
        match run_embedded(&f, &s(&["1/0"])) {
            Err(EmbedError::ToolFailed(1)) => {}
            other => panic!("expected ToolFailed(1), got {other:?}"),
        }
    }

    #[test]
    fn missing_file_is_runner_error() {
        match run_embedded(Path::new("/tidak/ada/impl.py"), &s(&[])) {
            Err(EmbedError::Runner(_)) => {}
            other => panic!("expected Runner, got {other:?}"),
        }
    }

    #[test]
    fn module_cache_reuses_state_across_runs() {
        // Global counter membuktikan modul TIDAK di-import ulang.
        let dir = std::env::temp_dir().join(format!("rplkit-pycache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("impl.py");
        std::fs::write(
            &f,
            "C = [0]\ndef main(argv):\n    C[0] += 1\n    print(C[0])\n    return 0\n",
        )
        .unwrap();
        let (c1, o1) = run_embedded_captured(&f, &s(&[])).expect("run 1");
        let (c2, o2) = run_embedded_captured(&f, &s(&[])).expect("run 2");
        assert_eq!((c1, c2), (0, 0));
        assert!(o1.contains('1'), "{o1}");
        assert!(o2.contains('2'), "{o2}");
        assert!(cache_len() >= 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
