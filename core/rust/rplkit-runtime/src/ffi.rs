//! FFI — satu pintu C++ → runtime (`rplkit_tool_run`).
//!
//! Kontrak memori: string kembaliannya milik pemanggil dan WAJIB
//! dibebaskan via `rplkit_str_free`. Tak pernah mengembalikan pointer
//! ke memori statis kecuali `rplkit_version` (jangan di-free).
//! Tak pernah panic melewati batas (catch_unwind → JSON error).

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::exec_native;

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn ok_json(output: &str) -> String {
    format!("{{\"ok\":true,\"output\":\"{}\",\"error\":\"\"}}", escape_json(output))
}

fn err_json(error: &str) -> String {
    format!("{{\"ok\":false,\"output\":\"\",\"error\":\"{}\"}}", escape_json(error))
}

/// Parse array JSON berisi string: `["a", "b"]`.
/// Cukup untuk argv; bukan parser JSON umum.
pub fn parse_json_string_array(text: &str) -> Result<Vec<String>, String> {
    let t = text.trim();
    if !t.starts_with('[') || !t.ends_with(']') {
        return Err("args_json must be a JSON array of strings".into());
    }
    let inner = &t[1..t.len() - 1];
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_str = false;
    let mut escape = false;
    let mut saw_bracket = false;
    for c in inner.chars() {
        if in_str {
            if escape {
                match c {
                    '"' => cur.push('"'),
                    '\\' => cur.push('\\'),
                    'n' => cur.push('\n'),
                    'r' => cur.push('\r'),
                    't' => cur.push('\t'),
                    _ => return Err(format!("bad escape: \\{c}")),
                }
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_str = false;
                out.push(std::mem::take(&mut cur));
            } else {
                cur.push(c);
            }
        } else if c == '"' {
            in_str = true;
            saw_bracket = true;
        } else if c == ',' || c.is_whitespace() {
        } else {
            return Err(format!("unexpected char in args_json: {c}"));
        }
    }
    if in_str {
        return Err("unterminated string in args_json".into());
    }
    let _ = saw_bracket;
    Ok(out)
}

fn run_inner(op: *const c_char, args_json: *const c_char) -> String {
    if op.is_null() || args_json.is_null() {
        return err_json("null pointer");
    }
    let op = unsafe { CStr::from_ptr(op) };
    let aj = unsafe { CStr::from_ptr(args_json) };
    let (op, aj) = match (op.to_str(), aj.to_str()) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return err_json("arguments must be valid UTF-8"),
    };
    let args = match parse_json_string_array(aj) {
        Ok(a) => a,
        Err(e) => return err_json(&e),
    };
    // Fase 2: hanya native. Rantai embed/fallback menyusul Fase 3–5.
    match exec_native::run_op(op, &args) {
        Ok(o) => ok_json(&o),
        Err(e) => err_json(&e),
    }
}

/// Jalankan op native. Return JSON `{"ok","output","error"}` (wajib
/// `rplkit_str_free`). NULL hanya bila alokasi gagal total.
#[no_mangle]
pub extern "C" fn rplkit_tool_run(op: *const c_char, args_json: *const c_char) -> *mut c_char {
    let out = std::panic::catch_unwind(|| run_inner(op, args_json))
        .unwrap_or_else(|_| err_json("native panic (caught at FFI boundary)"));
    CString::new(out).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

/// Bebaskan string dari `rplkit_tool_run`. Aman dipanggil dengan NULL.
#[no_mangle]
pub extern "C" fn rplkit_str_free(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

/// Versi runtime (pointer statis — JANGAN di-free).
#[no_mangle]
pub extern "C" fn rplkit_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

// ---------------------------------------------------------------------------
// Registry + chain untuk C++ tipis (Fase 5).
// ---------------------------------------------------------------------------

use std::path::Path;

fn cstr_to_str<'a>(p: *const c_char) -> Result<&'a str, String> {
    if p.is_null() {
        return Err("null pointer".into());
    }
    unsafe { CStr::from_ptr(p) }
        .to_str()
        .map_err(|_| "arguments must be valid UTF-8".into())
}

fn return_json(s: String) -> *mut c_char {
    CString::new(s).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

/// Daftar tool AKTIF sebagai JSON array:
/// `[{"name","runtime","description","entry"}]`, runtime = native|python|typescript.
/// Menghormati overlay (sama seperti `--list`); untuk semua (incl.
/// disabled) gunakan registry::discover langsung.
#[no_mangle]
pub extern "C" fn rplkit_list_tools(repo: *const c_char) -> *mut c_char {
    let out = (|| -> Result<String, String> {
        let repo = cstr_to_str(repo)?;
        let tools = crate::registry::discover_enabled(Path::new(repo));
        let mut items = Vec::new();
        for t in &tools {
            let (runtime, entry) = if t.native_op.is_some() && t.py_entry.is_none() && t.ts_entry.is_none() {
                ("native", t.native_op.clone().unwrap_or_default())
            } else if let Some(e) = t.py_entry.as_deref() {
                ("python", e.to_string())
            } else if let Some(e) = t.ts_entry.as_deref() {
                ("typescript", e.to_string())
            } else if let Some(o) = t.native_op.clone() {
                ("native", o)
            } else {
                continue;
            };
            // entry absolut agar CWD C++ bebas.
            let entry_abs = if runtime == "native" {
                entry
            } else {
                Path::new(repo).join(&entry).to_string_lossy().into_owned()
            };
            items.push(format!(
                "{{\"name\":\"{}\",\"runtime\":\"{runtime}\",\"description\":\"{}\",\"entry\":\"{}\"}}",
                escape_json(&t.name),
                escape_json(&t.description),
                escape_json(&entry_abs)
            ));
        }
        Ok(format!("[{}]", items.join(",")))
    })();
    match out {
        Ok(s) => return_json(s),
        Err(e) => return_json(err_json(&e)),
    }
}

/// Jalankan tool via chain. Return JSON:
/// `{"code":n,"executed_by":"...","discover_ms":n,"attempts":[{step,ms,ok}]}`.
/// Output tool passthrough ke stdout (sama seperti Rust bin).
#[no_mangle]
pub extern "C" fn rplkit_run_tool(
    repo: *const c_char,
    name: *const c_char,
    args_json: *const c_char,
) -> *mut c_char {
    let out = std::panic::catch_unwind(|| {
        let repo = match cstr_to_str(repo) {
            Ok(r) => r,
            Err(e) => return err_json(&e),
        };
        let name = match cstr_to_str(name) {
            Ok(n) => n,
            Err(e) => return err_json(&e),
        };
        let aj = match cstr_to_str(args_json) {
            Ok(a) => a,
            Err(e) => return err_json(&e),
        };
        let args = match parse_json_string_array(aj) {
            Ok(a) => a,
            Err(e) => return err_json(&e),
        };
        let t0d = std::time::Instant::now();
        let tools = crate::registry::discover(Path::new(repo));
        let discover_ms = t0d.elapsed().as_millis();
        let tool = match crate::registry::find(&tools, name) {
            Some(t) => t,
            None => return err_json(&format!("unknown tool: {name}")),
        };
        let o = crate::chain::run_tool(Path::new(repo), tool, &args);
        let attempts = o
            .attempts
            .iter()
            .map(|a| {
                format!(
                    "{{\"step\":\"{}\",\"ms\":{},\"ok\":{}}}",
                    a.step,
                    a.ms,
                    if a.ok { "true" } else { "false" }
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"code\":{},\"executed_by\":\"{}\",\"discover_ms\":{discover_ms},\"attempts\":[{attempts}]}}",
            o.code, o.executed_by
        )
    })
    .unwrap_or_else(|_| err_json("native panic (caught at FFI boundary)"));
    return_json(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(op: &str, args_json: &str) -> serde_free_guard {
        let co = CString::new(op).unwrap();
        let ca = CString::new(args_json).unwrap();
        let ptr = rplkit_tool_run(co.as_ptr(), ca.as_ptr());
        assert!(!ptr.is_null());
        serde_free_guard(ptr)
    }

    struct serde_free_guard(*mut c_char);
    impl serde_free_guard {
        fn text(&self) -> String {
            unsafe { CStr::from_ptr(self.0) }.to_string_lossy().into_owned()
        }
    }
    impl Drop for serde_free_guard {
        fn drop(&mut self) {
            rplkit_str_free(self.0);
        }
    }

    #[test]
    fn ffi_calc_roundtrip() {
        let r = call("calc", "[\"6 * 7\"]");
        let t = r.text();
        assert!(t.contains("\"ok\":true") && t.contains("42"), "{t}");
    }

    #[test]
    fn ffi_unknown_op_is_json_error() {
        let r = call("nope", "[]");
        let t = r.text();
        assert!(t.contains("\"ok\":false"), "{t}");
    }

    #[test]
    fn ffi_bad_json_is_json_error() {
        let r = call("calc", "bukan-json");
        assert!(r.text().contains("\"ok\":false"));
    }

    #[test]
    fn ffi_null_is_safe() {
        rplkit_str_free(std::ptr::null_mut()); // must not crash
        let ptr = rplkit_tool_run(std::ptr::null(), std::ptr::null());
        assert!(!ptr.is_null());
        let guard = serde_free_guard(ptr);
        assert!(guard.text().contains("null pointer"), "{}", guard.text());
    }

    fn repo_cstring() -> CString {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..");
        CString::new(root.to_string_lossy().into_owned()).unwrap()
    }

    #[test]
    fn ffi_list_tools_sees_manifests() {
        let repo = repo_cstring();
        let ptr = rplkit_list_tools(repo.as_ptr());
        assert!(!ptr.is_null());
        let guard = serde_free_guard(ptr);
        let t = guard.text();
        assert!(t.contains("\"calculator\""), "{t}");
        assert!(t.contains("\"hello\""), "{t}");
        assert!(t.contains("\"typescript\""), "{t}");
    }

    #[test]
    fn ffi_run_tool_reports_runner() {
        let repo = repo_cstring();
        let name = CString::new("calculator").unwrap();
        let args = CString::new("[\"6 * 7\"]").unwrap();
        let ptr = rplkit_run_tool(repo.as_ptr(), name.as_ptr(), args.as_ptr());
        assert!(!ptr.is_null());
        let guard = serde_free_guard(ptr);
        let t = guard.text();
        assert!(t.contains("\"code\":0"), "{t}");
        assert!(t.contains("native"), "{t}");
    }

    #[test]
    fn ffi_run_unknown_tool_errors() {
        let repo = repo_cstring();
        let name = CString::new("tidak-ada").unwrap();
        let args = CString::new("[]").unwrap();
        let ptr = rplkit_run_tool(repo.as_ptr(), name.as_ptr(), args.as_ptr());
        let guard = serde_free_guard(ptr);
        assert!(guard.text().contains("unknown tool"), "{}", guard.text());
    }
}
