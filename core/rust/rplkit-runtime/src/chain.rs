//! chain — orkestrasi rantai eksekusi per tool (Fase 3–4).
//!
//! Urutan dari `tool.json#exec`, default per runtime bila kosong.
//! Aturan: runner-OK-tapi-tool-gagal → berhenti (itu salah tool);
//! runner-tidak-tersedia → lanjut ke langkah berikut (fallback).
//! Output tool passthrough ke stdout (identik subprocess).

use std::path::Path;
use std::process::Command;

use crate::registry::ToolDef;
use crate::{exec_native, exec_py};

/// Hasil eksekusi satu tool.
pub struct Outcome {
    pub code: i32,
    pub executed_by: &'static str,
    /// Jejak langkah (sama seperti Captured::attempts).
    pub attempts: Vec<Attempt>,
}

/// Hasil capture untuk TUI (output selalu string, timing per langkah akhir).
pub struct Captured {
    pub code: i32,
    pub executed_by: &'static str,
    pub output: String,
    pub ms: u128,
    /// Jejak langkah yang dicoba, berurutan (Fase 0 instrumentasi).
    /// `ms` = biaya langkah itu; `ok` = langkah menghasilkan hasil
    /// (bukan sekadar dilewati karena tak tersedia).
    pub attempts: Vec<Attempt>,
}

/// Satu langkah rantai yang dicoba.
#[derive(Debug, Clone, PartialEq)]
pub struct Attempt {
    pub step: &'static str,
    pub ms: u128,
    pub ok: bool,
}

/// Varian capture: output tool dikembalikan (untuk Exec view + copy),
/// bukan passthrough. Timing diukur per langkah yang berhasil/gagal.
///
/// Menerima rantai TERKOMPILASI (tanpa match string). Untuk kompatibilitas,
/// lihat [`run_tool_captured`] yang mengkompilasi dari `&ToolDef`.
pub fn execute_compiled(chain: &[crate::registry::Executor], args: &[String]) -> Captured {
    use crate::registry::Executor;
    use std::time::Instant;
    let mut attempts: Vec<Attempt> = Vec::new();
    macro_rules! done {
        ($code:expr, $by:expr, $output:expr, $t0:expr, $step:expr) => {{
            attempts.push(Attempt { step: $step, ms: $t0.elapsed().as_millis(), ok: $code == 0 });
            let ms = $t0.elapsed().as_millis();
            return Captured {
                code: $code,
                executed_by: $by,
                output: $output,
                ms,
                attempts: std::mem::take(&mut attempts),
            };
        }};
    }
    macro_rules! skipped {
        ($step:expr) => {
            attempts.push(Attempt { step: $step, ms: 0, ok: false });
        };
    }
    let mut last_err = String::from("empty chain");
    for step in chain {
        match step {
            Executor::Native(op) => {
                let t0 = Instant::now();
                if op.is_empty() {
                    last_err = "native step without op".into();
                    skipped!("native");
                    continue;
                }
                match exec_native::run_op(op, args) {
                    Ok(out) => done!(0, "native", out, t0, "native"),
                    Err(e) => done!(1, "native", format!("error: {e}"), t0, "native"),
                }
            }
            Executor::PyEmbed(abs) => {
                let t0 = Instant::now();
                if abs.as_os_str().is_empty() {
                    last_err = "py-embed without entry".into();
                    skipped!("py-embed");
                    continue;
                }
                let r = std::panic::catch_unwind(|| exec_py::run_embedded_captured(abs, args));
                match r {
                    Ok(Ok((0, out))) => done!(0, "py-embed", out, t0, "py-embed"),
                    Ok(Ok((c, out))) => done!(c, "py-embed", out, t0, "py-embed"),
                    Ok(Err(exec_py::EmbedError::ToolFailed(c))) => {
                        done!(c, "py-embed", String::new(), t0, "py-embed")
                    }
                    Ok(Err(e)) => {
                        last_err = e.to_string();
                        skipped!("py-embed");
                        continue;
                    }
                    Err(_) => {
                        last_err = "python interpreter panicked".into();
                        skipped!("py-embed");
                        continue;
                    }
                }
            }
            Executor::PySys(abs) => {
                let t0 = Instant::now();
                if abs.as_os_str().is_empty() {
                    last_err = "py-sys without entry".into();
                    skipped!("py-sys");
                    continue;
                }
                match sys_capture("python3", abs, args) {
                    Ok((c, out)) => done!(c, "py-sys", out, t0, "py-sys"),
                    Err(e) => {
                        last_err = e;
                        skipped!("py-sys");
                        continue;
                    }
                }
            }
            Executor::JsEmbed(abs) => {
                let t0 = Instant::now();
                if abs.as_os_str().is_empty() {
                    last_err = "js-embed without entry".into();
                    skipped!("js-embed");
                    continue;
                }
                let src = match std::fs::read_to_string(abs) {
                    Ok(s) => s,
                    Err(e) => {
                        last_err = format!("cannot read {}: {e}", abs.display());
                        skipped!("js-embed");
                        continue;
                    }
                };
                let js = match crate::ts_strip(&src) {
                    Ok(j) => j,
                    Err(e) => {
                        last_err = format!("ts strip failed ({e})");
                        skipped!("js-embed");
                        continue;
                    }
                };
                let mut argv = vec!["node".to_string(), abs.to_string_lossy().into_owned()];
                argv.extend_from_slice(args);
                match crate::js_eval(&js, &argv) {
                    Ok((out, code)) => done!(code, "js-embed", out, t0, "js-embed"),
                    Err(e) => {
                        last_err = format!("js eval failed ({e})");
                        skipped!("js-embed");
                        continue;
                    }
                }
            }
            Executor::NodeSys(abs) => {
                let t0 = Instant::now();
                if abs.as_os_str().is_empty() {
                    last_err = "node-sys without entry".into();
                    skipped!("node-sys");
                    continue;
                }
                match sys_capture("node", abs, args) {
                    Ok((c, out)) => done!(c, "node-sys", out, t0, "node-sys"),
                    Err(e) => {
                        last_err = e;
                        skipped!("node-sys");
                        continue;
                    }
                }
            }
            Executor::Unknown(other) => {
                last_err = format!("unknown chain step: {other}");
                skipped!("unknown");
                continue;
            }
        }
    }
    Captured {
        code: 1,
        executed_by: "none",
        output: format!("error: {last_err}"),
        ms: 0,
        attempts,
    }
}

/// Kompatibilitas: kompilasi satu-kali dari `&ToolDef` lalu eksekusi.
/// Jalur panas (TUI/CLI berulang) sebaiknya memakai [`crate::registry::ToolHandle`].
pub fn run_tool_captured(repo_root: &Path, tool: &ToolDef, args: &[String]) -> Captured {
    execute_compiled(&crate::registry::compile_chain(tool, repo_root), args)
}

fn sys_capture(program: &str, entry: &Path, args: &[String]) -> Result<(i32, String), String> {
    let out = Command::new(program)
        .arg(entry)
        .args(args)
        .output()
        .map_err(|e| format!("cannot spawn {program}: {e}"))?;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    let err = String::from_utf8_lossy(&out.stderr);
    if !err.is_empty() {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&err);
    }
    Ok((out.status.code().unwrap_or(1), text))
}

fn sys_command(program: &str, entry: &Path, args: &[String]) -> Result<i32, String> {
    let status = Command::new(program)
        .arg(entry)
        .args(args)
        .status()
        .map_err(|e| format!("cannot spawn {program}: {e}"))?;
    Ok(status.code().unwrap_or(1))
}

/// Jalankan tool mengikuti rantainya. Tak pernah panic (catch_unwind di
/// tiap langkah embed). Varian passthrough: output mengalir ke stdout
/// (untuk REPL interaktif); perilaku cetak IDENTIK dengan versi lama.
pub fn run_tool(repo_root: &Path, tool: &ToolDef, args: &[String]) -> Outcome {
    execute_passthrough(&crate::registry::compile_chain(tool, repo_root), args)
}

/// Inti passthrough atas rantai TERKOMPILASI (dipakai [`run_tool`] dan
/// [`crate::registry::ToolHandle::execute_passthrough`]).
pub fn execute_passthrough(chain: &[crate::registry::Executor], args: &[String]) -> Outcome {
    use crate::registry::Executor;
    use std::time::Instant;
    let mut attempts: Vec<Attempt> = Vec::new();
    macro_rules! done {
        ($code:expr, $by:expr, $t0:expr, $step:expr) => {{
            attempts.push(Attempt { step: $step, ms: $t0.elapsed().as_millis(), ok: $code == 0 });
            return Outcome {
                code: $code,
                executed_by: $by,
                attempts: std::mem::take(&mut attempts),
            };
        }};
    }
    macro_rules! skipped {
        ($step:expr) => {
            attempts.push(Attempt { step: $step, ms: 0, ok: false });
        };
    }
    let mut last_err = String::from("empty chain");
    for step in chain {
        match step {
            Executor::Native(op) => {
                let t0 = Instant::now();
                if op.is_empty() {
                    last_err = "native step without op".into();
                    skipped!("native");
                    continue;
                }
                match exec_native::run_op(&op, args) {
                    Ok(out) => {
                        println!("{out}");
                        done!(0, "native", t0, "native");
                    }
                    Err(e) => {
                        eprintln!("error: {e}");
                        done!(1, "native", t0, "native");
                    }
                }
            }
            Executor::PyEmbed(abs) => {
                let t0 = Instant::now();
                if abs.as_os_str().is_empty() {
                    last_err = "py-embed without entry".into();
                    skipped!("py-embed");
                    continue;
                }
                // argv untuk main(): argumen tool saja, TANPA nama prog
                // (impl.py memakai sys.argv[1:] bila argv None — konsisten).
                let argv: Vec<String> = args.to_vec();
                let r = std::panic::catch_unwind(|| exec_py::run_embedded(&abs, &argv));
                match r {
                    Ok(Ok(0)) => done!(0, "py-embed", t0, "py-embed"),
                    Ok(Ok(c)) => done!(c, "py-embed", t0, "py-embed"),
                    Ok(Err(exec_py::EmbedError::ToolFailed(c))) => {
                        done!(c, "py-embed", t0, "py-embed")
                    }
                    Ok(Err(e)) => {
                        last_err = e.to_string();
                        eprintln!("py-embed unavailable ({last_err}), trying fallback…");
                        skipped!("py-embed");
                        continue;
                    }
                    Err(_) => {
                        last_err = "python interpreter panicked".into();
                        eprintln!("py-embed panicked, trying fallback…");
                        skipped!("py-embed");
                        continue;
                    }
                }
            }
            Executor::PySys(abs) => {
                let t0 = Instant::now();
                if abs.as_os_str().is_empty() {
                    last_err = "py-sys without entry".into();
                    skipped!("py-sys");
                    continue;
                }
                match sys_command("python3", &abs, args) {
                    Ok(c) => done!(c, "py-sys", t0, "py-sys"),
                    Err(e) => {
                        last_err = e;
                        skipped!("py-sys");
                        continue;
                    }
                }
            }
            Executor::JsEmbed(abs) => {
                let t0 = Instant::now();
                if abs.as_os_str().is_empty() {
                    last_err = "js-embed without entry".into();
                    skipped!("js-embed");
                    continue;
                }
                let src = match std::fs::read_to_string(&abs) {
                    Ok(s) => s,
                    Err(e) => {
                        last_err = format!("cannot read {}: {e}", abs.display());
                        skipped!("js-embed");
                        continue;
                    }
                };
                let js = match crate::ts_strip(&src) {
                    Ok(j) => j,
                    Err(e) => {
                        last_err = format!("ts strip failed ({e}), trying node…");
                        eprintln!("{last_err}");
                        skipped!("js-embed");
                        continue;
                    }
                };
                let mut argv = vec!["node".to_string(), abs.to_string_lossy().into_owned()];
                argv.extend_from_slice(args);
                match crate::js_eval(&js, &argv) {
                    Ok((out, code)) => {
                        print!("{out}");
                        done!(code, "js-embed", t0, "js-embed");
                    }
                    Err(e) => {
                        last_err = format!("js eval failed ({e}), trying node…");
                        eprintln!("{last_err}");
                        skipped!("js-embed");
                        continue;
                    }
                }
            }
            Executor::NodeSys(abs) => {
                let t0 = Instant::now();
                if abs.as_os_str().is_empty() {
                    last_err = "node-sys without entry".into();
                    skipped!("node-sys");
                    continue;
                }
                match sys_command("node", &abs, args) {
                    Ok(c) => done!(c, "node-sys", t0, "node-sys"),
                    Err(e) => {
                        last_err = e;
                        skipped!("node-sys");
                        continue;
                    }
                }
            }
            Executor::Unknown(other) => {
                last_err = format!("unknown chain step: {other}");
                skipped!("unknown");
                continue;
            }
        }
    }
    eprintln!("error: tool could not run ({last_err})");
    Outcome { code: 1, executed_by: "none", attempts }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry;

    fn repo_root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
    }

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn chain_prefers_native_for_calc() {
        let tools = registry::discover(&repo_root());
        let calc = registry::find(&tools, "calculator").expect("calc");
        let o = run_tool(&repo_root(), calc, &s(&["6 * 7"]));
        assert_eq!((o.code, o.executed_by), (0, "native"));
        assert_eq!(o.attempts.len(), 1);
        assert_eq!(o.attempts[0].step, "native");
        assert!(o.attempts[0].ok);
    }

    #[test]
    fn chain_runs_python_via_embed() {
        let tools = registry::discover(&repo_root());
        let qr = registry::find(&tools, "qr_generator").expect("qr");
        // qr_generator main(argv) dengan 1 arg teks → cetak box, code 0.
        let o = run_tool(&repo_root(), qr, &s(&["halo"]));
        assert_eq!((o.code, o.executed_by), (0, "py-embed"));
    }

    #[test]
    fn chain_runs_ts_via_embed() {
        let tools = registry::discover(&repo_root());
        let hello = registry::find(&tools, "hello").expect("hello");
        let o = run_tool(&repo_root(), hello, &s(&["dunia"]));
        assert_eq!((o.code, o.executed_by), (0, "js-embed"));
    }

    #[test]
    fn chain_unknown_tool_reports_none() {
        let empty: Vec<registry::ToolDef> = Vec::new();
        assert!(registry::find(&empty, "x").is_none());
    }

    #[test]
    fn capture_native_returns_output_and_timing() {
        let tools = registry::discover(&repo_root());
        let calc = registry::find(&tools, "calculator").expect("calc");
        let c = run_tool_captured(&repo_root(), calc, &s(&["6 * 7"]));
        assert_eq!((c.code, c.executed_by), (0, "native"));
        assert!(c.output.contains("42"), "{}", c.output);
    }

    #[test]
    fn capture_py_embed_collects_stdout() {
        // qr_generator tanpa native op → rantai py-embed.
        let tools = registry::discover(&repo_root());
        let qr = registry::find(&tools, "qr_generator").expect("qr");
        assert!(!qr.exec.contains(&"native".to_string()));
        let c = run_tool_captured(&repo_root(), qr, &s(&["hi"]));
        assert_eq!(c.code, 0);
        assert_eq!(c.executed_by, "py-embed");
        assert!(c.output.contains("hi"), "{}", c.output);
    }

    #[test]
    fn capture_js_embed_collects_console() {
        let tools = registry::discover(&repo_root());
        let hello = registry::find(&tools, "hello").expect("hello");
        let c = run_tool_captured(&repo_root(), hello, &s(&["dunia"]));
        assert_eq!(c.code, 0);
        assert_eq!(c.executed_by, "js-embed");
        assert!(c.output.contains("hello, dunia!"), "{}", c.output);
    }

    #[test]
    fn capture_falls_back_to_py_sys() {
        // Tool fixture tanpa embed: exec langsung py-sys.
        let repo = std::env::temp_dir().join(format!("rplkit-captest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&repo);
        let dir = repo.join("tools").join("echo-tool");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("impl.py"), "def main(argv):\n    print('CAP-OK')\n    return 0\n\n\nif __name__ == '__main__':\n    raise SystemExit(main(__import__('sys').argv[1:]))\n").unwrap();
        std::fs::write(
            dir.join("tool.json"),
            "{\"name\": \"echo-tool\", \"version\": \"1\", \"description\": \"t\", \
             \"module\": \"general\", \"exec\": [\"py-sys\"], \"entries\": {\"python\": \"impl.py\"}}",
        )
        .unwrap();
        let tools = registry::discover(&repo);
        let tool = registry::find(&tools, "echo-tool").expect("echo");
        // Satu arg agar impl tidak masuk mode interaktif.
        let c = run_tool_captured(&repo, tool, &s(&["x"]));
        assert_eq!((c.code, c.executed_by), (0, "py-sys"));
        assert!(c.output.contains("CAP-OK"), "{}", c.output);
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn attempts_record_skipped_steps() {
        use crate::registry::ToolDef;
        // Tanpa entry: kedua langkah dilewati, jejak lengkap.
        let def = ToolDef {
            name: "hantu".into(),
            version: "1".into(),
            description: "t".into(),
            module: "m".into(),
            group: None,
            exec: vec!["py-embed".into(), "py-sys".into()],
            native_op: None,
            py_entry: None,
            ts_entry: None,
            dir: std::path::PathBuf::new(),
        };
        let c = run_tool_captured(std::path::Path::new("/x"), &def, &[]);
        assert_eq!(c.code, 1);
        let steps: Vec<&str> = c.attempts.iter().map(|a| a.step).collect();
        assert_eq!(steps, vec!["py-embed", "py-sys"]);
        assert!(c.attempts.iter().all(|a| !a.ok));
    }
}
