//! `rplkit` — thin CLI over [`rplkit_runtime`] (Fase 5).
//!
//! Semua discovery + eksekusi milik registry/chain; bin ini hanya
//! parsing argumen dan menampilkan hasil:
//! ```sh
//! rplkit --list
//! rplkit --run calculator "2 + 3 * 4"
//! rplkit --run hello dunia --verbose
//! ```

use rplkit_core::find_repo_root;
use rplkit_runtime::{modules, registry};
use std::env;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    let start = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for cand in [cwd, start] {
        let root = find_repo_root(&cand);
        if root.join("tools").is_dir() {
            return root;
        }
    }
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn display_runtime(t: &registry::ToolDef) -> &'static str {
    if t.py_entry.is_some() {
        "python"
    } else if t.ts_entry.is_some() {
        "typescript"
    } else {
        "native"
    }
}

fn display_entry(root: &std::path::Path, t: &registry::ToolDef) -> String {
    if let Some(e) = t.py_entry.as_deref().or(t.ts_entry.as_deref()) {
        return root.join(e).to_string_lossy().into_owned();
    }
    t.native_op.clone().unwrap_or_default()
}

fn help(prog: &str) {
    println!("RPLKit Developer Tools (native Rust runner)\n");
    println!("Usage:");
    println!("  {prog}                                 TUI (when stdout is a tty)");
    println!("  {prog} --list [--all] [--dev]          list enabled tools (--all: incl. disabled)");
    println!("  {prog} --run <tool> [args...] [--verbose] [--dev]");
    println!("                                         run a tool");
    println!("  {prog} <tool> [args...]                 run a tool (scripting short form)");
    println!("  {prog} module list [--all]              list modules");
    println!("  {prog} module info <name>               module details");
    println!("  {prog} module enable|disable <name>     toggle module (or single tool)");
    println!("  {prog} tools build-index               rebuild .cache/tools.idx");
    println!("  {prog} --help                          this help");
    println!();
    println!("--dev: selalu rescan manifest + diagnostik (abaikan index).");
}

fn print_tools(root: &std::path::Path, all: bool, dev: bool) {
    if all {
        if dev {
            eprintln!("index: bypassed (--dev)");
        }
        return print_defs(root, &registry::discover(root));
    }
    let reg = registry::Registry::build_opts(root, !dev);
    if dev {
        eprintln!("index: skipped (--dev), parsed {} manifest(s)", reg.len());
    }
    for t in reg.tools() {
        println!("{} [{}]  {}", t.name, t.runtime_label, t.description);
        println!("    entry: {}", t.entry_label);
    }
}

fn print_defs(root: &std::path::Path, defs: &[registry::ToolDef]) {
    for t in defs {
        println!("{} [{}]  {}", t.name, display_runtime(t), t.description);
        println!("    entry: {}", display_entry(root, t));
    }
}

fn cmd_module(root: &std::path::Path, args: &[String]) -> i32 {
    match args.first().map(|s| s.as_str()) {
        Some("list") => {
            let all = args.iter().any(|a| a == "--all");
            let descs = modules::load_descriptors(root);
            let overlay = modules::load_overlay(root);
            let tools = if all { registry::discover(root) } else { registry::discover_enabled(root) };
            println!("{:<16} {:<10} {:<8} {}", "MODULE", "RUNTIME", "STATUS", "TOOLS");
            for d in &descs {
                let n = tools.iter().filter(|t| t.module == d.name).count();
                let on = modules::is_enabled(d, &overlay, None);
                println!(
                    "{:<16} {:<10} {:<8} {} tool(s) v{}",
                    d.name,
                    d.runtime,
                    if on { "enabled" } else { "disabled" },
                    n,
                    d.version
                );
            }
            0
        }
        Some("info") => {
            let Some(name) = args.get(1) else {
                eprintln!("usage: rplkit module info <name>");
                return 2;
            };
            let descs = modules::load_descriptors(root);
            let overlay = modules::load_overlay(root);
            match descs.iter().find(|d| &d.name == name) {
                Some(d) => {
                    println!("{} v{} ({})", d.name, d.version, d.runtime);
                    println!("{}\nby {}", d.description, d.author);
                    println!(
                        "status: {}",
                        if modules::is_enabled(d, &overlay, None) { "enabled" } else { "disabled" }
                    );
                    println!("commands:");
                    for t in registry::discover(root).iter().filter(|t| &t.module == name) {
                        println!("  {:<16} {}", t.name, t.description);
                    }
                    0
                }
                None => {
                    eprintln!("unknown module: {name}");
                    1
                }
            }
        }
        Some("enable") | Some("disable") => {
            let (cmd, Some(name)) = (args[0].as_str(), args.get(1)) else {
                eprintln!("usage: rplkit module enable|disable <name>");
                return 2;
            };
            let on = cmd == "enable";
            let descs = modules::load_descriptors(root);
            if descs.iter().any(|d| &d.name == name) {
                return match modules::set_module_enabled(root, name, on) {
                    Ok(()) => {
                        println!("module {name} {}d", if on { "enable" } else { "disable" });
                        0
                    }
                    Err(e) => {
                        eprintln!("error: {e}");
                        1
                    }
                };
            }
            // Bukan modul: coba sebagai tool tunggal (override).
            if registry::discover(root).iter().any(|t| &t.name == name) {
                return match modules::set_tool_enabled(root, name, on) {
                    Ok(()) => {
                        println!("tool {name} {}d (override)", if on { "enable" } else { "disable" });
                        0
                    }
                    Err(e) => {
                        eprintln!("error: {e}");
                        1
                    }
                };
            }
            eprintln!("unknown module or tool: {name}");
            1
        }
        _ => {
            eprintln!("usage: rplkit module list|info|enable|disable [--all]");
            2
        }
    }
}

fn main() {
    let argv: Vec<String> = env::args().collect();
    let prog = argv.first().map(|s| s.as_str()).unwrap_or("rplkit");
    if argv.len() < 2 {
        // Dua mode (mock): tanpa argumen + tty → TUI; dialihkan/diskrip → bantuan.
        if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
            let root = repo_root();
            match rplkit_tui::run(&root) {
                Ok(c) => std::process::exit(c),
                Err(e) => {
                    eprintln!("tui error: {e}");
                    std::process::exit(1);
                }
            }
        }
        help(prog);
        return;
    }
    match argv[1].as_str() {
        "--help" | "-h" | "help" => help(prog),
        "--list" | "list" => {
            let root = repo_root();
            print_tools(
                &root,
                argv.iter().any(|a| a == "--all"),
                argv.iter().any(|a| a == "--dev"),
            );
        }
        "--run" | "run" => {
            if argv.len() < 3 {
                eprintln!("usage: {prog} --run <tool> [args...]");
                std::process::exit(2);
            }
            let verbose = argv.iter().any(|a| a == "--verbose");
            let dev = argv.iter().any(|a| a == "--dev");
            let args: Vec<String> = argv[3..]
                .iter()
                .filter(|a| a.as_str() != "--verbose" && a.as_str() != "--dev")
                .cloned()
                .collect();
            let root = repo_root();
            let reg = registry::Registry::build_opts(&root, !dev);
            match reg.handle(&argv[2]) {
                Some(h) => {
                    let t0 = std::time::Instant::now();
                    let o = h.execute_passthrough(&args);
                    rplkit_runtime::history::append(
                        &root,
                        rplkit_runtime::history::RunRecord {
                            tool: argv[2].clone(),
                            args: args.clone(),
                            code: o.code,
                            ms: t0.elapsed().as_millis(),
                            by: o.executed_by.to_string(),
                            at: rplkit_runtime::history::now_unix(),
                        },
                    );
                    if verbose {
                        eprintln!("executed_by: {}", o.executed_by);
                        eprintln!(
                            "discover: {}ms (index: {}, rescanned: {})",
                            reg.build_ms,
                            if reg.index_hit { "hit" } else { "miss" },
                            reg.rescanned
                        );
                        for a in &o.attempts {
                            eprintln!(
                                "  step {}: {}ms {}",
                                a.step,
                                a.ms,
                                if a.ok { "ok" } else { "skip/fail" }
                            );
                        }
                    }
                    std::process::exit(o.code);
                }
                None if registry::find(&registry::discover(&root), &argv[2]).is_some() => {
                    eprintln!("tool '{}' is disabled. Enable: {prog} module enable {}", argv[2], argv[2]);
                    std::process::exit(1);
                }
                None => {
                    eprintln!("unknown tool: {}", argv[2]);
                    std::process::exit(1);
                }
            }
        }
        "module" => {
            let root = repo_root();
            std::process::exit(cmd_module(&root, &argv[2..].to_vec()));
        }
        "tools" => {
            // `rplkit tools build-index` — tulis ulang .cache/tools.idx.
            if argv.get(2).map(|s| s.as_str()) == Some("build-index") {
                let root = repo_root();
                match rplkit_runtime::index::build_index(&root) {
                    Ok(n) => {
                        println!("indexed {n} tool(s) → .cache/tools.idx");
                        std::process::exit(0);
                    }
                    Err(e) => {
                        eprintln!("error: {e}");
                        std::process::exit(1);
                    }
                }
            }
            eprintln!("usage: {prog} tools build-index");
            std::process::exit(2);
        }
        other if other.starts_with('-') => {
            eprintln!("unknown command: {other}");
            help(prog);
            std::process::exit(2);
        }
        tool => {
            // Bentuk scripting: `rplkit <tool> [args...]`.
            let args: Vec<String> = argv[2..].to_vec();
            let root = repo_root();
            let reg = registry::Registry::build(&root);
            match reg.handle(tool) {
                Some(h) => {
                    let t0 = std::time::Instant::now();
                    let o = h.execute_passthrough(&args);
                    rplkit_runtime::history::append(
                        &root,
                        rplkit_runtime::history::RunRecord {
                            tool: tool.to_string(),
                            args: args.clone(),
                            code: o.code,
                            ms: t0.elapsed().as_millis(),
                            by: o.executed_by.to_string(),
                            at: rplkit_runtime::history::now_unix(),
                        },
                    );
                    std::process::exit(o.code);
                }
                None => {
                    eprintln!("unknown tool: {tool}");
                    std::process::exit(1);
                }
            }
        }
    }
}
