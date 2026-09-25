//! `rplkit` — thin CLI over [`rplkit_core`].
//!
//! Mirrors the C++ core (`core/cpp`) commands so both native cores stay
//! behaviour-compatible:
//! ```sh
//! rplkit --list
//! rplkit --run calc "2 + 3 * 4"
//! rplkit --run b64enc hello
//! rplkit --run uuid
//! rplkit --run timestamp
//! ```

use rplkit_core::{b64_decode, b64_encode, calc, discover_tools, find_repo_root, iso_now, unix_now, uuid4};
use std::env;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    let start = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    // current_exe lives in core/rust/target/...; also try cwd.
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for cand in [cwd, start] {
        let root = find_repo_root(&cand);
        if root.join("tools").is_dir() {
            return root;
        }
    }
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn help(prog: &str) {
    println!("RPLKit Developer Tools (native Rust core)\n");
    println!("Usage:");
    println!("  {prog} --list                 list discovered tools");
    println!("  {prog} --run <tool> [args...]  run a tool");
    println!("  {prog} --help                 this help");
}

fn main() {
    let argv: Vec<String> = env::args().collect();
    let prog = argv.first().map(|s| s.as_str()).unwrap_or("rplkit");
    if argv.len() < 2 {
        help(prog);
        return;
    }
    match argv[1].as_str() {
        "--help" | "-h" | "help" => help(prog),
        "--list" | "list" => {
            let root = repo_root();
            for t in discover_tools(&root) {
                println!("{} [{}]  {}", t.name, t.runtime, t.description);
                println!("    entry: {}", t.entry);
            }
        }
        "--run" | "run" => {
            if argv.len() < 3 {
                eprintln!("usage: {prog} --run <tool> [args...]");
                std::process::exit(2);
            }
            let rest: Vec<&str> = argv[3..].iter().map(|s| s.as_str()).collect();
            let code = dispatch(&argv[2], &rest);
            std::process::exit(code);
        }
        other => {
            eprintln!("unknown command: {other}");
            help(prog);
            std::process::exit(2);
        }
    }
}

fn dispatch(tool: &str, args: &[&str]) -> i32 {
    match tool {
        "calc" => {
            if args.is_empty() {
                eprintln!("usage: rplkit --run calc \"<expression>\"");
                return 2;
            }
            match calc(args[0]) {
                Ok(v) => {
                    println!("{v}");
                    0
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    1
                }
            }
        }
        "b64enc" => {
            let input = args.first().copied().unwrap_or("");
            println!("{}", b64_encode(input.as_bytes()));
            0
        }
        "b64dec" => {
            let input = args.first().copied().unwrap_or("");
            match b64_decode(input) {
                Ok(bytes) => match String::from_utf8(bytes) {
                    Ok(s) => {
                        println!("{s}");
                        0
                    }
                    Err(_) => {
                        eprintln!("error: decoded bytes are not valid UTF-8");
                        1
                    }
                },
                Err(e) => {
                    eprintln!("error: {e}");
                    1
                }
            }
        }
        "uuid" => {
            let mut n: usize = args.first().and_then(|s| s.parse().ok()).unwrap_or(1);
            n = n.clamp(1, 100);
            for _ in 0..n {
                println!("{}", uuid4());
            }
            0
        }
        "timestamp" => {
            println!("{}", unix_now());
            println!("{}", iso_now());
            0
        }
        _ => {
            // Script tools need python3/node at runtime; the Rust core
            // reports the resolved entry so the caller can dispatch.
            let root = repo_root();
            match discover_tools(&root).into_iter().find(|t| t.name == tool) {
                Some(t) if t.runtime != "native" => {
                    println!("entry: {}", t.entry);
                    println!("runtime: {}", t.runtime);
                    eprintln!("note: run script tools with the C++ core or `python3 {}`", t.entry);
                    0
                }
                _ => {
                    eprintln!("unknown tool: {tool}");
                    1
                }
            }
        }
    }
}
