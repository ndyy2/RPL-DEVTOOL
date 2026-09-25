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
}

/// Hasil capture untuk TUI (output selalu string, timing per langkah akhir).
pub struct Captured {
    pub code: i32,
    pub executed_by: &'static str,
    pub output: String,
    pub ms: u128,
}

/// Varian capture: output tool dikembalikan (untuk Exec view + copy),
/// bukan passthrough. Timing diukur per langkah yang berhasil/gagal.
pub fn run_tool_captured(repo_root: &Path, tool: &ToolDef, args: &[String]) -> Captured {
    use std::time::Instant;
    let finish = |code: i32, by: &'static str, output: String, t0: Instant| Captured {
        code,
        executed_by: by,
        output,
        ms: t0.elapsed().as_millis(),
    };
    let mut last_err = String::from("empty chain");
    for step in default_chain(tool) {
        match step {
            "native" => {
                let t0 = Instant::now();
                let op = match tool.native_op.as_deref() {
                    Some(o) => o,
                    None => {
                        last_err = "native step without op".into();
                        continue;
                    }
                };
                match exec_native::run_op(op, args) {
                    Ok(out) => return finish(0, "native", out, t0),
                    Err(e) => return finish(1, "native", format!("error: {e}"), t0),
                }
            }
            "py-embed" => {
                let rel = match tool.py_entry.as_deref() {
                    Some(r) => r,
                    None => {
                        last_err = "py-embed without entry".into();
                        continue;
                    }
                };
                let abs = repo_root.join(rel);
                let t0 = Instant::now();
                let r = std::panic::catch_unwind(|| exec_py::run_embedded_captured(&abs, args));
                match r {
                    Ok(Ok((0, out))) => return finish(0, "py-embed", out, t0),
                    Ok(Ok((c, out))) => return finish(c, "py-embed", out, t0),
                    Ok(Err(exec_py::EmbedError::ToolFailed(c))) => {
                        return finish(c, "py-embed", String::new(), t0)
                    }
                    Ok(Err(e)) => {
                        last_err = e.to_string();
                        continue;
                    }
                    Err(_) => {
                        last_err = "python interpreter panicked".into();
                        continue;
                    }
                }
            }
            "py-sys" => {
                let rel = match tool.py_entry.as_deref() {
                    Some(r) => r,
                    None => {
                        last_err = "py-sys without entry".into();
                        continue;
                    }
                };
                let t0 = Instant::now();
                match sys_capture("python3", &repo_root.join(rel), args) {
                    Ok((c, out)) => return finish(c, "py-sys", out, t0),
                    Err(e) => {
                        last_err = e;
                        continue;
                    }
                }
            }
            "js-embed" => {
                let rel = match tool.ts_entry.as_deref() {
                    Some(r) => r,
                    None => {
                        last_err = "js-embed without entry".into();
                        continue;
                    }
                };
                let src = match std::fs::read_to_string(repo_root.join(rel)) {
                    Ok(s) => s,
                    Err(e) => {
                        last_err = format!("cannot read {rel}: {e}");
                        continue;
                    }
                };
                let t0 = Instant::now();
                let js = match crate::ts_strip(&src) {
                    Ok(j) => j,
                    Err(e) => {
                        last_err = format!("ts strip failed ({e})");
                        continue;
                    }
                };
                let mut argv = vec!["node".to_string(), rel.to_string()];
                argv.extend_from_slice(args);
                match crate::js_eval(&js, &argv) {
                    Ok((out, code)) => return finish(code, "js-embed", out, t0),
                    Err(e) => {
                        last_err = format!("js eval failed ({e})");
                        continue;
                    }
                }
            }
            "node-sys" => {
                let rel = match tool.ts_entry.as_deref() {
                    Some(r) => r,
                    None => {
                        last_err = "node-sys without entry".into();
                        continue;
                    }
                };
                let t0 = Instant::now();
                match sys_capture("node", &repo_root.join(rel), args) {
                    Ok((c, out)) => return finish(c, "node-sys", out, t0),
                    Err(e) => {
                        last_err = e;
                        continue;
                    }
                }
            }
            _ => {
                last_err = format!("unknown chain step: {step}");
                continue;
            }
        }
    }
    Captured { code: 1, executed_by: "none", output: format!("error: {last_err}"), ms: 0 }
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

fn default_chain(tool: &ToolDef) -> Vec<&'static str> {
    if !tool.exec.is_empty() {
        return tool.exec.iter().map(|s| match s.as_str() {
            "native" => "native",
            "py-embed" => "py-embed",
            "py-sys" => "py-sys",
            "js-embed" => "js-embed",
            "node-sys" => "node-sys",
            _ => "unknown",
        }).collect();
    }
    if tool.py_entry.is_some() {
        vec!["py-embed", "py-sys"]
    } else if tool.ts_entry.is_some() {
        vec!["js-embed", "node-sys"]
    } else {
        vec!["native"]
    }
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
/// tiap langkah embed).
pub fn run_tool(repo_root: &Path, tool: &ToolDef, args: &[String]) -> Outcome {
    let mut last_err = String::from("empty chain");
    for step in default_chain(tool) {
        match step {
            "native" => {
                let op = match tool.native_op.as_deref() {
                    Some(o) => o,
                    None => {
                        last_err = "native step without op".into();
                        continue;
                    }
                };
                match exec_native::run_op(op, args) {
                    Ok(out) => {
                        println!("{out}");
                        return Outcome { code: 0, executed_by: "native" };
                    }
                    Err(e) => {
                        eprintln!("error: {e}");
                        return Outcome { code: 1, executed_by: "native" };
                    }
                }
            }
            "py-embed" => {
                let rel = match tool.py_entry.as_deref() {
                    Some(r) => r,
                    None => {
                        last_err = "py-embed without entry".into();
                        continue;
                    }
                };
                let abs = repo_root.join(rel);
                // argv untuk main(): argumen tool saja, TANPA nama prog
                // (impl.py memakai sys.argv[1:] bila argv None — konsisten).
                let argv: Vec<String> = args.to_vec();
                let r = std::panic::catch_unwind(|| exec_py::run_embedded(&abs, &argv));
                match r {
                    Ok(Ok(0)) => return Outcome { code: 0, executed_by: "py-embed" },
                    Ok(Ok(c)) => return Outcome { code: c, executed_by: "py-embed" },
                    Ok(Err(exec_py::EmbedError::ToolFailed(c))) => {
                        return Outcome { code: c, executed_by: "py-embed" }
                    }
                    Ok(Err(e)) => {
                        last_err = e.to_string();
                        eprintln!("py-embed unavailable ({last_err}), trying fallback…");
                        continue;
                    }
                    Err(_) => {
                        last_err = "python interpreter panicked".into();
                        eprintln!("py-embed panicked, trying fallback…");
                        continue;
                    }
                }
            }
            "py-sys" => {
                let rel = match tool.py_entry.as_deref() {
                    Some(r) => r,
                    None => {
                        last_err = "py-sys without entry".into();
                        continue;
                    }
                };
                match sys_command("python3", &repo_root.join(rel), args) {
                    Ok(c) => return Outcome { code: c, executed_by: "py-sys" },
                    Err(e) => {
                        last_err = e;
                        continue;
                    }
                }
            }
            "js-embed" => {
                let rel = match tool.ts_entry.as_deref() {
                    Some(r) => r,
                    None => {
                        last_err = "js-embed without entry".into();
                        continue;
                    }
                };
                let src = match std::fs::read_to_string(repo_root.join(rel)) {
                    Ok(s) => s,
                    Err(e) => {
                        last_err = format!("cannot read {rel}: {e}");
                        continue;
                    }
                };
                let js = match crate::ts_strip(&src) {
                    Ok(j) => j,
                    Err(e) => {
                        last_err = format!("ts strip failed ({e}), trying node…");
                        eprintln!("{last_err}");
                        continue;
                    }
                };
                let mut argv = vec!["node".to_string(), rel.to_string()];
                argv.extend_from_slice(args);
                match crate::js_eval(&js, &argv) {
                    Ok((out, code)) => {
                        print!("{out}");
                        return Outcome { code, executed_by: "js-embed" };
                    }
                    Err(e) => {
                        last_err = format!("js eval failed ({e}), trying node…");
                        eprintln!("{last_err}");
                        continue;
                    }
                }
            }
            "node-sys" => {
                let rel = match tool.ts_entry.as_deref() {
                    Some(r) => r,
                    None => {
                        last_err = "node-sys without entry".into();
                        continue;
                    }
                };
                match sys_command("node", &repo_root.join(rel), args) {
                    Ok(c) => return Outcome { code: c, executed_by: "node-sys" },
                    Err(e) => {
                        last_err = e;
                        continue;
                    }
                }
            }
            _ => {
                last_err = format!("unknown chain step: {step}");
                continue;
            }
        }
    }
    eprintln!("error: tool could not run ({last_err})");
    Outcome { code: 1, executed_by: "none" }
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
}
