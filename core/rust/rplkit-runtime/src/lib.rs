//! rplkit-runtime — runner sesungguhnya RPLKit.
//!
//! Rantai eksekusi per tool (dideklarasikan di `tools/*/tool.json`):
//! native op (Rust) → embed Python (PyO3) / embed JS (swc + QuickJS)
//! → fallback interpreter sistem (python3/node subprocess).
//!
//! C++ (`core/cpp`) hanya CLI + menu + FFI tipis ke crate ini
//! (lihat Fase 5). Di-link statis: `crate-type = ["staticlib", "rlib"]`.

pub fn runtime_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub mod registry;
pub mod modules;
pub mod history;
pub mod exec_native;
pub mod exec_py;
pub mod chain;
pub mod ffi;

// ---------------------------------------------------------------------------
// Embedded Python (PyO3) — Fase 3 pakai penuh, Fase 0: smoke link + version.
// ---------------------------------------------------------------------------

/// Versi interpreter Python yang di-embed, mis. "3.14.7".
/// Err bila libpython tak bisa diinisialisasi (pemicu fallback sistem).
pub fn py_version() -> Result<String, String> {
    pyo3::Python::attach(|py| {
        let v = py.version_info();
        Ok(format!("{}.{}.{}", v.major, v.minor, v.patch))
    })
}

// ---------------------------------------------------------------------------
// Embedded JS (swc strip + QuickJS) — Fase 4 pakai penuh.
// ---------------------------------------------------------------------------

use swc_core::{
    common::{sync::Lrc, FileName, Globals, SourceMap, GLOBALS, Mark},
    ecma::{
        ast::EsVersion,
        codegen::{text_writer::JsWriter, Config as CodegenConfig, Emitter},
        parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax},
        transforms::{base::resolver, typescript::strip},
    },
};

/// Strip tipe TypeScript → JavaScript polos (erasable-syntax only).
/// Gagal bila sintaks tak didukung — pemicu fallback `node`.
pub fn ts_strip(src: &str) -> Result<String, String> {
    GLOBALS.set(&Globals::new(), || {
        let cm: Lrc<SourceMap> = Default::default();
        let fname: Lrc<FileName> = FileName::Custom("tool.ts".into()).into();
        let fm = cm.new_source_file(fname, src.to_owned());
        let lexer = Lexer::new(
            Syntax::Typescript(TsSyntax {
                tsx: false,
                decorators: true,
                dts: false,
                no_early_errors: false,
                disallow_ambiguous_jsx_like: false,
            }),
            EsVersion::Es2022,
            StringInput::from(&*fm),
            None,
        );
        let mut parser = Parser::new_from(lexer);
        let program = parser
            .parse_program()
            .map_err(|e| format!("ts parse: {e:?}"))?;
        let unresolved = Mark::new();
        let top_level = Mark::new();
        let program = program
            .apply(resolver(unresolved, top_level, true))
            .apply(strip(unresolved, top_level));
        let mut buf = Vec::new();
        {
            let mut emitter = Emitter {
                cfg: CodegenConfig::default(),
                comments: None,
                cm: cm.clone(),
                wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
            };
            emitter
                .emit_program(&program)
                .map_err(|e| format!("ts emit: {e:?}"))?;
        }
        String::from_utf8(buf).map_err(|e| format!("ts utf8: {e}"))
    })
}

use rquickjs::{Array, Context, Function, Object, Runtime};

/// Eksekusi JS di QuickJS dengan shim `process.argv`/`process.exit` dan
/// `console.log` yang ditangkap. Return (captured_stdout, exit_code).
/// `process.exit(n)` mencatat kode; tool mengikuti template hello.ts
/// (exit dipanggil di akhir), didokumentasikan di kontrak tool.
pub fn js_eval(src: &str, argv: &[String]) -> Result<(String, i32), String> {
    let rt = Runtime::new().map_err(|e| format!("js runtime: {e:?}"))?;
    let ctx = Context::full(&rt).map_err(|e| format!("js context: {e:?}"))?;
    ctx.with(|ctx| {
        let out = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
        let out2 = out.clone();
        let print = Function::new(ctx.clone(), move |s: String| {
            out2.borrow_mut().push_str(&s);
            out2.borrow_mut().push('\n');
        })
        .map_err(|e| format!("js print: {e:?}"))?;
        ctx.globals()
            .set("print", print)
            .map_err(|e| format!("js global: {e:?}"))?;

        let args = Array::new(ctx.clone()).map_err(|e| format!("js argv: {e:?}"))?;
        for (i, a) in argv.iter().enumerate() {
            args.set(i, a.clone())
                .map_err(|e| format!("js argv: {e:?}"))?;
        }
        let code_cell = std::rc::Rc::new(std::cell::Cell::new(0i32));
        let code2 = code_cell.clone();
        let exit = Function::new(ctx.clone(), move |c: i32| {
            code2.set(c);
        })
        .map_err(|e| format!("js exit: {e:?}"))?;
        let process = Object::new(ctx.clone()).map_err(|e| format!("js process: {e:?}"))?;
        process
            .set("argv", args)
            .map_err(|e| format!("js process: {e:?}"))?;
        process
            .set("exit", exit)
            .map_err(|e| format!("js process: {e:?}"))?;
        ctx.globals()
            .set("process", process)
            .map_err(|e| format!("js global: {e:?}"))?;

        ctx.eval::<(), _>("var console={log:(...a)=>print(a.map(String).join(' '))};")
            .map_err(|e| format!("js console: {e:?}"))?;
        ctx.eval::<(), _>(src.as_bytes())
            .map_err(|e| format!("js eval: {e:?}"))?;
        let result = (out.borrow().clone(), code_cell.get());
        Ok(result)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_present() {
        assert!(!runtime_version().is_empty());
    }

    #[test]
    fn embedded_python_reports_version() {
        let v = py_version().expect("libpython must initialize");
        assert!(v.starts_with("3."), "unexpected: {v}");
    }

    const HELLO_TS: &str = include_str!("../../../../tools/hello/impl.ts");

    #[test]
    fn ts_strip_removes_types() {
        let js = ts_strip(HELLO_TS).expect("strip must succeed");
        assert!(js.contains("process.argv"), "argv lost:\n{js}");
        assert!(!js.contains(": string"), "types remain:\n{js}");
    }

    #[test]
    fn ts_lowwers_enums_to_runnable_js() {
        // Kontrak tool TS: sintaks apa pun yang swc bisa turunkan ke JS
        // (termasuk enum) jalan embedded; sisanya jatuh ke fallback node.
        let js = ts_strip("enum E { A, B }\nconsole.log(E.A);").expect("strip enum");
        let argv = ["node".to_string(), "t".to_string()];
        let (out, code) = js_eval(&js, &argv).expect("eval enum");
        assert_eq!(code, 0);
        assert!(out.contains('0'), "output was: {out}");
    }

    #[test]
    fn js_runs_stripped_hello() {
        let js = ts_strip(HELLO_TS).expect("strip must succeed");
        let argv = ["node".to_string(), "hello".to_string(), "dunia".to_string()];
        let (out, code) = js_eval(&js, &argv).expect("eval must succeed");
        assert_eq!(code, 0);
        assert!(out.contains("hello, dunia!"), "output was: {out}");
    }
}
