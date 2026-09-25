//! exec_native — 6 op native + converter (Fase 2).
//!
//! Dipanggil langsung (Rust bin) maupun via FFI dari C++ (ffi.rs).
//! Tiap op: `run_op(op, args) -> Result<output_stdout, error>`.

use std::path::Path;

// ---------------------------------------------------------------------------
// Angka: samakan tampilan C++ (4 bukan 4.0).
// ---------------------------------------------------------------------------

fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

// ---------------------------------------------------------------------------
// codec: b64enc b64dec hexenc hexdec urlenc urldec rot13
// ---------------------------------------------------------------------------

fn hex_encode(bytes: &[u8]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(H[(b >> 4) as usize] as char);
        s.push(H[(b & 15) as usize] as char);
    }
    s
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

fn hex_decode(text: &str) -> Result<Vec<u8>, String> {
    let t: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if t.len() % 2 != 0 {
        return Err("odd-length hex string".into());
    }
    let mut out = Vec::with_capacity(t.len() / 2);
    for pair in t.chunks(2) {
        match (hex_val(pair[0]), hex_val(pair[1])) {
            (Some(hi), Some(lo)) => out.push((hi << 4) | lo),
            _ => return Err("invalid hex character".into()),
        }
    }
    Ok(out)
}

fn url_encode(text: &str) -> String {
    let mut s = String::new();
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            s.push(b as char);
        } else {
            s.push_str(&format!("%{b:02X}"));
        }
    }
    s
}

fn url_decode(text: &str) -> Result<String, String> {
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err("truncated % escape".into());
            }
            match (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                (Some(hi), Some(lo)) => out.push((hi << 4) | lo),
                _ => return Err("invalid % escape".into()),
            }
            i += 3;
        } else if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|e| format!("invalid UTF-8: {e}"))
}

fn rot13(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'a'..= 'z' => (((c as u8 - b'a' + 13) % 26) + b'a') as char,
            'A'..= 'Z' => (((c as u8 - b'A' + 13) % 26) + b'A') as char,
            _ => c,
        })
        .collect()
}

fn decode_to_string(bytes: Vec<u8>, what: &str) -> Result<String, String> {
    String::from_utf8(bytes).map_err(|e| format!("invalid {what}: decoded bytes are not UTF-8 ({e})"))
}

fn codec(args: &[String]) -> Result<String, String> {
    if args.len() < 2 {
        return Err("usage: codec <b64enc|b64dec|hexenc|hexdec|urlenc|urldec|rot13> <text>".into());
    }
    let text = args[1..].join(" ");
    match args[0].to_lowercase().as_str() {
        "b64enc" => Ok(rplkit_core::b64_encode(text.as_bytes())),
        "b64dec" => rplkit_core::b64_decode(&text)
            .map_err(|e| e.to_string())
            .and_then(|b| decode_to_string(b, "base64")),
        "hexenc" => Ok(hex_encode(text.as_bytes())),
        "hexdec" => hex_decode(&text).and_then(|b| decode_to_string(b, "hex")),
        "urlenc" => Ok(url_encode(&text)),
        "urldec" => url_decode(&text),
        "rot13" => Ok(rot13(&text)),
        other => Err(format!("unknown codec command: {other}")),
    }
}

// ---------------------------------------------------------------------------
// timestamp (+ ISO parse minimal)
// ---------------------------------------------------------------------------

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(y) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Parse "YYYY-MM-DDTHH:MM:SSZ" atau offset "+HH:MM"/"+HHMM"/"Z" → unix.
fn iso_to_unix(text: &str) -> Result<i64, String> {
    let s = text.trim();
    let err = || format!("invalid ISO-8601: {text}");
    if s.len() < 19 {
        return Err(err());
    }
    let num = |a: usize, b: usize| -> Result<i64, String> {
        s.get(a..b).ok_or_else(err)?.parse().map_err(|_| err())
    };
    if &s[4..5] != "-" || &s[7..8] != "-" || &s[10..11] != "T" || &s[13..14] != ":"
        || &s[16..17] != ":"
    {
        return Err(err());
    }
    let (y, mo, d, h, mi, se) = (num(0, 4)?, num(5, 7)?, num(8, 10)?, num(11, 13)?, num(14, 16)?, num(17, 19)?);
    if !(1..=12).contains(&mo) || d < 1 || d > days_in_month(y, mo) || h > 23 || mi > 59 || se > 60 {
        return Err(err());
    }
    let mut ts = days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60 + se;
    let tz = &s[19..];
    if tz.is_empty() || tz == "Z" {
    } else if tz.len() == 6 && (tz.starts_with('+') || tz.starts_with('-')) && &tz[3..4] == ":" {
        let sign = if tz.starts_with('-') { -1 } else { 1 };
        let oh: i64 = tz[1..3].parse().map_err(|_| err())?;
        let om: i64 = tz[4..6].parse().map_err(|_| err())?;
        ts -= sign * (oh * 3600 + om * 60);
    } else {
        return Err(err());
    }
    Ok(ts)
}

fn timestamp(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Ok(format!("{}\n{}", rplkit_core::unix_now(), rplkit_core::iso_now()));
    }
    let raw = args.join(" ");
    if let Ok(v) = raw.parse::<f64>() {
        if v < 0.0 {
            return Err("negative timestamps need the python fallback".into());
        }
        return Ok(rplkit_core::iso_from_unix(v as u64));
    }
    Ok(iso_to_unix(&raw)?.to_string())
}

// ---------------------------------------------------------------------------
// converter: suhu / panjang / berat (cermin tools/converter/impl.py)
// ---------------------------------------------------------------------------

const LEN_TO_M: &[(&str, f64)] = &[
    ("mm", 0.001),
    ("cm", 0.01),
    ("m", 1.0),
    ("km", 1000.0),
    ("in", 0.0254),
    ("ft", 0.3048),
    ("mi", 1609.344),
];

const W_TO_KG: &[(&str, f64)] = &[
    ("g", 0.001),
    ("kg", 1.0),
    ("lb", 0.45359237),
    ("oz", 0.028349523125),
];

fn lookup(table: &[(&str, f64)], unit: &str) -> Option<f64> {
    let u = unit.to_lowercase();
    table.iter().find(|(k, _)| *k == u).map(|(_, v)| *v)
}

fn convert(value: f64, from: &str, to: &str) -> Result<f64, String> {
    let fu = from.to_uppercase();
    let tu = to.to_uppercase();
    if ["C", "F", "K"].contains(&fu.as_str()) || ["C", "F", "K"].contains(&tu.as_str()) {
        let c = match fu.as_str() {
            "C" => value,
            "F" => (value - 32.0) * 5.0 / 9.0,
            "K" => value - 273.15,
            _ => return Err(format!("unsupported temperature unit: {from}")),
        };
        return match tu.as_str() {
            "C" => Ok(c),
            "F" => Ok(c * 9.0 / 5.0 + 32.0),
            "K" => Ok(c + 273.15),
            _ => Err(format!("unsupported temperature unit: {to}")),
        };
    }
    if let (Some(a), Some(b)) = (lookup(LEN_TO_M, from), lookup(LEN_TO_M, to)) {
        return Ok(value * a / b);
    }
    if let (Some(a), Some(b)) = (lookup(W_TO_KG, from), lookup(W_TO_KG, to)) {
        return Ok(value * a / b);
    }
    Err(format!("unsupported units: {from} -> {to}"))
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

/// Jalankan satu op native. `args` = argv tanpa nama tool.
pub fn run_op(op: &str, args: &[String]) -> Result<String, String> {
    match op {
        "calc" => {
            let expr = args.first().ok_or("usage: calc \"<expression>\"")?;
            rplkit_core::calc(expr)
                .map(fmt_num)
                .map_err(|e| e.to_string())
        }
        "codec" => codec(args),
        "uuid" => {
            let mut n: usize = args.first().and_then(|s| s.parse().ok()).unwrap_or(1);
            n = n.clamp(1, 100);
            Ok((0..n).map(|_| rplkit_core::uuid4()).collect::<Vec<_>>().join("\n"))
        }
        "timestamp" => timestamp(args),
        "file-info" => {
            let path = args.first().ok_or("usage: file-info <path>")?;
            let (exists, size, lines) = rplkit_core::file_info(Path::new(path));
            Ok(format!(
                "exists: {}\nsize: {size} bytes\nlines: {lines}",
                if exists { "yes" } else { "no" }
            ))
        }
        "convert" => {
            if args.len() != 3 {
                return Err("usage: convert <value> <from> <to>".into());
            }
            let value: f64 = args[0].parse().map_err(|_| "value must be a number".to_string())?;
            convert(value, &args[1], &args[2]).map(fmt_num)
        }
        "sysinfo" => Ok(op_sysinfo()),
        "process" => {
            let n: usize = args.first().and_then(|s| s.parse().ok()).unwrap_or(10);
            Ok(op_process(n))
        }
        "memory" => Ok(op_memory()),
        "disk" => Ok(op_disk()),
        "portcheck" => op_portcheck(args),
        "dns" => op_dns(args),
        "gitclean" => op_gitclean(args),
        "gitstatus" => op_gitstatus(args),
        _ => Err(format!("unknown native op: {op}")),
    }
}

// ---------------------------------------------------------------------------
// system-tools (via rplkit_core::sys — sumber yang sama dengan TUI Overview)
// ---------------------------------------------------------------------------

use rplkit_core::sys as sysmod;

fn op_sysinfo() -> String {
    let s = sysmod::snapshot();
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "System Information\n");
    let _ = writeln!(out, "OS             {}", s.os);
    let _ = writeln!(out, "Kernel         {}", s.kernel);
    let _ = writeln!(out, "Architecture   {}", s.arch);
    let _ = writeln!(out, "Hostname       {}", s.hostname);
    let _ = writeln!(
        out,
        "CPU            {} × {} ({:.0}%)",
        s.cpu_count,
        s.cpus.first().map(|c| c.name.as_str()).unwrap_or("?"),
        s.global_cpu_pct
    );
    let _ = writeln!(
        out,
        "Memory         {} / {}",
        sysmod::human_bytes(s.mem.used),
        sysmod::human_bytes(s.mem.total)
    );
    out.trim_end().to_string()
}

fn op_process(n: usize) -> String {
    use std::fmt::Write as _;
    let mut out = String::from("MEM      CPU%  COMMAND\n");
    for p in sysmod::top_processes(n) {
        let _ = writeln!(
            out,
            "{:<8} {:>5.1}  {}",
            sysmod::human_bytes(p.memory),
            p.cpu_pct,
            p.name
        );
    }
    out.trim_end().to_string()
}

fn op_memory() -> String {
    let m = sysmod::snapshot().mem;
    format!(
        "Memory   {} / {}\nSwap     {} / {}",
        sysmod::human_bytes(m.used),
        sysmod::human_bytes(m.total),
        sysmod::human_bytes(m.used_swap),
        sysmod::human_bytes(m.total_swap)
    )
}

fn op_disk() -> String {
    use std::fmt::Write as _;
    let s = sysmod::snapshot();
    let mut out = String::new();
    for d in &s.disks {
        let used = d.total.saturating_sub(d.available);
        let pct = if d.total > 0 { used * 100 / d.total } else { 0 };
        let _ = writeln!(
            out,
            "{}  {} / {} ({pct}%)  on {} [{}]",
            d.name,
            sysmod::human_bytes(used),
            sysmod::human_bytes(d.total),
            d.mount,
            d.fs
        );
    }
    out.trim_end().to_string()
}

// ---------------------------------------------------------------------------
// network-tools (std::net only)
// ---------------------------------------------------------------------------

fn parse_target(t: &str) -> (String, u16) {
    // [host:]port — host default 127.0.0.1.
    if let Some(i) = t.rfind(':') {
        if let Ok(p) = t[i + 1..].parse::<u16>() {
            let host = &t[..i];
            return (if host.is_empty() { "127.0.0.1".into() } else { host.into() }, p);
        }
    }
    (String::from("127.0.0.1"), t.parse().unwrap_or(0))
}

fn op_portcheck(args: &[String]) -> Result<String, String> {
    use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
    use std::time::Duration;
    if args.is_empty() {
        return Err("usage: portcheck [host:]port [...]".into());
    }
    let mut lines = Vec::new();
    for t in args {
        let (host, port) = parse_target(t);
        if port == 0 {
            return Err(format!("invalid target: {t}"));
        }
        let state = match format!("{host}:{port}").parse::<SocketAddr>() {
            Ok(addr) => {
                if TcpStream::connect_timeout(&addr, Duration::from_millis(800)).is_ok() {
                    "open"
                } else {
                    "closed"
                }
            }
            Err(_) => {
                // Hostname: coba resolve dulu.
                match (host.as_str(), port).to_socket_addrs() {
                    Ok(mut it) => match it.next() {
                        Some(addr) => {
                            if TcpStream::connect_timeout(&addr, Duration::from_millis(800)).is_ok() {
                                "open"
                            } else {
                                "closed"
                            }
                        }
                        None => "unresolvable",
                    },
                    Err(_) => "unresolvable",
                }
            }
        };
        lines.push(format!("{host}:{port}  {state}"));
    }
    Ok(lines.join("\n"))
}

fn op_dns(args: &[String]) -> Result<String, String> {
    use std::net::ToSocketAddrs;
    if args.is_empty() {
        return Err("usage: dns <host> [...]".into());
    }
    let mut lines = Vec::new();
    for h in args {
        match (h.as_str(), 0).to_socket_addrs() {
            Ok(addrs) => {
                let ips: Vec<String> =
                    addrs.map(|a| a.ip().to_string()).collect::<std::collections::HashSet<_>>().into_iter().collect();
                if ips.is_empty() {
                    lines.push(format!("{h}  unresolvable"));
                } else {
                    lines.push(format!("{h}  {}", ips.join(", ")));
                }
            }
            Err(e) => lines.push(format!("{h}  error: {e}")),
        }
    }
    Ok(lines.join("\n"))
}

// ---------------------------------------------------------------------------
// git-tools (git CLI; error rapi bila git/repo absen)
// ---------------------------------------------------------------------------

fn git_cmd(dir: &str, args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| format!("cannot run git (is it installed?): {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("git {} failed: {}", args.join(" "), err.trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

fn op_gitstatus(args: &[String]) -> Result<String, String> {
    let dir = args.first().map(|s| s.as_str()).unwrap_or(".");
    let branch = git_cmd(dir, &["branch", "--show-current"])?;
    let short = git_cmd(dir, &["status", "--short"])?;
    if short.is_empty() {
        Ok(format!("branch: {branch}\nclean"))
    } else {
        Ok(format!("branch: {branch}\n{short}"))
    }
}

fn op_gitclean(args: &[String]) -> Result<String, String> {
    let force = args.iter().any(|a| a == "--force");
    let dir = args.iter().find(|a| *a != "--force").map(|s| s.as_str()).unwrap_or(".");
    if force {
        let out = git_cmd(dir, &["clean", "-fdx"])?;
        return Ok(if out.is_empty() { "nothing to clean".into() } else { out });
    }
    let preview = git_cmd(dir, &["clean", "-ndx"])?;
    if preview.is_empty() {
        Ok("nothing to clean".into())
    } else {
        Ok(format!("dry-run (pass --force to delete):\n{preview}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn calc_vectors() {
        assert_eq!(run_op("calc", &s(&["2 + 3 * 4"])).unwrap(), "14");
        assert_eq!(run_op("calc", &s(&["2 ** 10"])).unwrap(), "1024");
        assert_eq!(run_op("calc", &s(&["10 / 4"])).unwrap(), "2.5");
        assert!(run_op("calc", &s(&["__import__('os')"])).is_err());
        assert!(run_op("calc", &s(&["1/0"])).is_err());
    }

    #[test]
    fn codec_roundtrips() {
        assert_eq!(run_op("codec", &s(&["b64enc", "halo"])).unwrap(), "aGFsbw==");
        assert_eq!(run_op("codec", &s(&["b64dec", "aGFsbw=="])).unwrap(), "halo");
        assert_eq!(run_op("codec", &s(&["hexenc", "halo"])).unwrap(), "68616c6f");
        assert_eq!(run_op("codec", &s(&["hexdec", "68616c6f"])).unwrap(), "halo");
        assert_eq!(run_op("codec", &s(&["urlenc", "a b/c?"])).unwrap(), "a%20b%2Fc%3F");
        assert_eq!(run_op("codec", &s(&["urldec", "a%20b"])).unwrap(), "a b");
        assert_eq!(run_op("codec", &s(&["rot13", "abc"])).unwrap(), "nop");
        assert!(run_op("codec", &s(&["b64dec", "!!!"])).is_err());
    }

    #[test]
    fn convert_vectors() {
        assert!((run_op("convert", &s(&["100", "C", "F"])).unwrap().parse::<f64>().unwrap() - 212.0).abs() < 1e-9);
        assert!((run_op("convert", &s(&["1", "km", "m"])).unwrap().parse::<f64>().unwrap() - 1000.0).abs() < 1e-9);
        assert!((run_op("convert", &s(&["1", "kg", "g"])).unwrap().parse::<f64>().unwrap() - 1000.0).abs() < 1e-9);
        assert!(run_op("convert", &s(&["1", "km", "kg"])).is_err());
    }

    #[test]
    fn timestamp_vectors() {
        assert_eq!(run_op("timestamp", &s(&["0"])).unwrap(), "1970-01-01T00:00:00Z");
        assert_eq!(run_op("timestamp", &s(&["1970-01-01T00:00:00Z"])).unwrap(), "0");
        assert!(run_op("timestamp", &s(&["bukan-tanggal"])).is_err());
        assert!(run_op("timestamp", &[]).unwrap().contains('Z'));
    }

    #[test]
    fn uuid_shape_and_file_info() {
        let one = run_op("uuid", &[]).unwrap();
        assert_eq!(one.len(), 36);
        assert_eq!(run_op("uuid", &s(&["3"])).unwrap().lines().count(), 3);
        let info = run_op("file-info", &s(&["Cargo.toml"])).unwrap_or_else(|_| {
            run_op("file-info", &s(&["../Cargo.toml"])).unwrap()
        });
        assert!(info.contains("exists: yes"), "{info}");
    }

    #[test]
    fn sysinfo_block() {
        let out = run_op("sysinfo", &[]).unwrap();
        for key in ["OS", "Kernel", "Architecture", "Memory"] {
            assert!(out.contains(key), "missing {key}:\n{out}");
        }
    }

    #[test]
    fn process_memory_disk_smoke() {
        let ps = run_op("process", &s(&["3"])).unwrap();
        assert!(ps.lines().count() >= 2, "{ps}");
        let mem = run_op("memory", &[]).unwrap();
        assert!(mem.contains("Memory") && mem.contains("Swap"), "{mem}");
        let disk = run_op("disk", &[]).unwrap();
        assert!(disk.contains('/'), "{disk}");
    }

    #[test]
    fn portcheck_closed_and_invalid() {
        // Port bebas: bind ephemeral lalu lepas → hampir pasti closed.
        let free: u16 = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let out = run_op("portcheck", &s(&[&format!("127.0.0.1:{free}")])).unwrap();
        assert!(out.contains("closed"), "{out}");
        assert!(run_op("portcheck", &[]).is_err());
        assert!(run_op("portcheck", &s(&["bukan-target"])).is_err());
    }

    #[test]
    fn dns_localhost() {
        let out = run_op("dns", &s(&["localhost"])).unwrap();
        assert!(out.contains("127.0.0.1"), "{out}");
        assert!(run_op("dns", &[]).is_err());
    }

    fn temp_git_repo(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rplkit-gittest-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let git = |a: &[&str]| {
            std::process::Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(a)
                .output()
                .expect("git must exist for this test")
        };
        assert!(git(&["init", "-q"]).status.success());
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        std::fs::write(dir.join("a.txt"), "x").unwrap();
        git(&["add", "."]);
        assert!(git(&["commit", "-qm", "init"]).status.success());
        dir
    }

    #[test]
    fn git_status_and_clean() {
        let dir = temp_git_repo("status");
        let ds = dir.to_string_lossy().to_string();
        let st = run_op("gitstatus", &s(&[&ds])).unwrap();
        assert!(st.contains("clean") || st.contains("branch"), "{st}");
        std::fs::write(dir.join("junk.tmp"), "z").unwrap();
        let dry = run_op("gitclean", &s(&[&ds])).unwrap();
        assert!(dry.contains("junk.tmp"), "{dry}");
        assert!(dry.starts_with("dry-run"), "{dry}");
        assert!(dir.join("junk.tmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn git_outside_repo_errors() {
        let dir = std::env::temp_dir().join(format!("rplkit-nogit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let ds = dir.to_string_lossy().to_string();
        assert!(run_op("gitstatus", &s(&[&ds])).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
