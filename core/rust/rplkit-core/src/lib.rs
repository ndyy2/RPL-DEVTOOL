//! rplkit-core — memory-safe core components for RPLKit.
//!
//! Counterpart to `core/cpp`: the same responsibilities (tool manifests,
//! safe calculation, encodings, ids, time) implemented in safe Rust with
//! no external dependencies and no `unsafe`.
//!
//! Build & test (requires a Rust toolchain):
//! ```sh
//! cargo build --manifest-path core/rust/Cargo.toml
//! cargo test --manifest-path core/rust/Cargo.toml
//! ```

use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod sys;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Core error type. Every fallible API returns `Result<T, CoreError>`.
#[derive(Debug, Clone, PartialEq)]
pub enum CoreError {
    InvalidInput(String),
    Io(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::InvalidInput(m) => write!(f, "invalid input: {m}"),
            CoreError::Io(m) => write!(f, "io error: {m}"),
        }
    }
}

impl std::error::Error for CoreError {}

// ---------------------------------------------------------------------------
// Module interface: tool manifests
// ---------------------------------------------------------------------------

/// One entry in the System Module registry.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolManifest {
    pub name: String,
    /// "native" | "python" | "typescript"
    pub runtime: String,
    /// builtin id for native, script path otherwise
    pub entry: String,
    pub description: String,
}

impl ToolManifest {
    pub fn native(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            runtime: "native".to_string(),
            entry: name.to_string(),
            description: description.to_string(),
        }
    }
}

/// Builtins always available, even with no `tools/` on disk.
pub fn native_builtins() -> Vec<ToolManifest> {
    vec![
        ToolManifest::native("calc", "Safe arithmetic evaluator (Rust core)"),
        ToolManifest::native("b64enc", "Base64 encode (Rust core)"),
        ToolManifest::native("b64dec", "Base64 decode (Rust core)"),
        ToolManifest::native("uuid", "Random UUID v4 (Rust core)"),
        ToolManifest::native("timestamp", "Unix + ISO-8601 UTC time (Rust core)"),
    ]
}

/// Minimal `"key": "value"` extractor for flat manifest JSON.
/// Manifests are written by us (`{"name":..,"description":..}`), so a tiny
/// string scanner is sufficient and keeps the crate dependency-free.
pub fn json_string_field(text: &str, key: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let needle = format!("\"{key}\"");
    let mut i = text.find(needle.as_str())? + needle.len();
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r') {
        i += 1;
    }
    if bytes.get(i) != Some(&b':') {
        return None;
    }
    i += 1;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r') {
        i += 1;
    }
    if bytes.get(i) != Some(&b'"') {
        return None;
    }
    i += 1;
    let mut out = String::new();
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '\\' && i + 1 < bytes.len() {
            out.push(bytes[i + 1] as char);
            i += 2;
            continue;
        }
        if c == '"' {
            break;
        }
        out.push(c);
        i += 1;
    }
    Some(out)
}

fn describe_script(script: &Path, runtime: &str, stem: &str) -> String {
    let manifest = script.with_extension("json");
    if let Ok(text) = fs::read_to_string(&manifest) {
        if let Some(d) = json_string_field(&text, "description") {
            if !d.is_empty() {
                return d;
            }
        }
    }
    format!("{runtime} tool: {stem}")
}

fn scan_dir(dir: &Path, runtime: &str, ext: &str, out: &mut Vec<ToolManifest>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some(ext) {
            continue;
        }
        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) if s != "__init__" => s.to_string(),
            _ => continue,
        };
        out.push(ToolManifest {
            name: stem.clone(),
            runtime: runtime.to_string(),
            entry: path.to_string_lossy().into_owned(),
            description: describe_script(&path, runtime, &stem),
        });
    }
}

/// Discover tools under `<repo>/tools/{python,typescript}` plus the legacy
/// flat `tools/*.py` layout. New layout wins on name clashes.
pub fn discover_tools(repo_root: &Path) -> Vec<ToolManifest> {
    let mut tools = native_builtins();
    scan_dir(&repo_root.join("tools").join("python"), "python", "py", &mut tools);
    scan_dir(
        &repo_root.join("tools").join("typescript"),
        "typescript",
        "ts",
        &mut tools,
    );
    // Legacy flat layout.
    let mut legacy: Vec<ToolManifest> = Vec::new();
    scan_dir(&repo_root.join("tools"), "python", "py", &mut legacy);
    for t in legacy {
        if !tools.iter().any(|u| u.name == t.name) {
            tools.push(t);
        }
    }
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    tools
}

/// Walk up from `start` (max 5 levels) looking for `tools/`, `config/`,
/// `main.py`, or `pcc.conf.json`.
pub fn find_repo_root(start: &Path) -> PathBuf {
    let mut cur: PathBuf = start.into();
    for _ in 0..5 {
        if cur.join("tools").is_dir()
            || cur.join("config").is_dir()
            || cur.join("main.py").is_file()
            || cur.join("pcc.conf.json").is_file()
        {
            return cur;
        }
        match cur.parent() {
            Some(p) => cur = p.to_path_buf(),
            None => break,
        }
    }
    start.to_path_buf()
}

// ---------------------------------------------------------------------------
// Safe calculator: + - * / // % **, parens, unary +/-
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Tok {
    Num(f64),
    Plus,
    Minus,
    Star,
    Slash,
    SlashSlash,
    Percent,
    StarStar,
    LParen,
    RParen,
    End,
}

struct Lexer<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(s: &'a str) -> Self {
        Self { bytes: s.as_bytes(), pos: 0 }
    }

    fn ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn next(&mut self) -> Result<Tok, CoreError> {
        self.ws();
        if self.pos >= self.bytes.len() {
            return Ok(Tok::End);
        }
        let c = self.bytes[self.pos];
        if c.is_ascii_digit() || c == b'.' {
            return self.number();
        }
        match c {
            b'+' => {
                self.pos += 1;
                Ok(Tok::Plus)
            }
            b'-' => {
                self.pos += 1;
                Ok(Tok::Minus)
            }
            b'(' => {
                self.pos += 1;
                Ok(Tok::LParen)
            }
            b')' => {
                self.pos += 1;
                Ok(Tok::RParen)
            }
            b'%' => {
                self.pos += 1;
                Ok(Tok::Percent)
            }
            b'*' => {
                if self.bytes.get(self.pos + 1) == Some(&b'*') {
                    self.pos += 2;
                    Ok(Tok::StarStar)
                } else {
                    self.pos += 1;
                    Ok(Tok::Star)
                }
            }
            b'/' => {
                if self.bytes.get(self.pos + 1) == Some(&b'/') {
                    self.pos += 2;
                    Ok(Tok::SlashSlash)
                } else {
                    self.pos += 1;
                    Ok(Tok::Slash)
                }
            }
            _ => {
                let ch = c as char;
                return Err(CoreError::InvalidInput(format!("unexpected character: '{ch}'")));
            }
        }
    }

    fn number(&mut self) -> Result<Tok, CoreError> {
        let start = self.pos;
        let mut dots = 0;
        while self.pos < self.bytes.len()
            && (self.bytes[self.pos].is_ascii_digit() || self.bytes[self.pos] == b'.')
        {
            if self.bytes[self.pos] == b'.' {
                dots += 1;
                if dots > 1 {
                    break;
                }
            }
            self.pos += 1;
        }
        let s = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|e| CoreError::InvalidInput(e.to_string()))?;
        s.parse::<f64>()
            .map(Tok::Num)
            .map_err(|_| CoreError::InvalidInput(format!("bad number: {s}")))
    }
}

struct Parser<'a> {
    lex: Lexer<'a>,
    cur: Tok,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Result<Self, CoreError> {
        let mut lex = Lexer::new(s);
        let cur = lex.next()?;
        Ok(Self { lex, cur })
    }

    fn bump(&mut self) -> Result<(), CoreError> {
        self.cur = self.lex.next()?;
        Ok(())
    }

    fn expr(&mut self) -> Result<f64, CoreError> {
        let mut v = self.term()?;
        loop {
            match self.cur {
                Tok::Plus => {
                    self.bump()?;
                    v += self.term()?;
                }
                Tok::Minus => {
                    self.bump()?;
                    v -= self.term()?;
                }
                _ => break,
            }
        }
        Ok(v)
    }

    fn term(&mut self) -> Result<f64, CoreError> {
        let mut v = self.factor()?;
        loop {
            match self.cur {
                Tok::Star => {
                    self.bump()?;
                    v *= self.factor()?;
                }
                Tok::Slash => {
                    self.bump()?;
                    let r = self.factor()?;
                    if r == 0.0 {
                        return Err(CoreError::InvalidInput("division by zero".into()));
                    }
                    v /= r;
                }
                Tok::SlashSlash => {
                    self.bump()?;
                    let r = self.factor()?;
                    if r == 0.0 {
                        return Err(CoreError::InvalidInput("division by zero".into()));
                    }
                    v = ((v as i64) / (r as i64)) as f64;
                }
                Tok::Percent => {
                    self.bump()?;
                    let r = self.factor()?;
                    if r == 0.0 {
                        return Err(CoreError::InvalidInput("modulo by zero".into()));
                    }
                    v = ((v as i64) % (r as i64)) as f64;
                }
                _ => break,
            }
        }
        Ok(v)
    }

    fn factor(&mut self) -> Result<f64, CoreError> {
        let base = self.unary()?;
        if matches!(self.cur, Tok::StarStar) {
            self.bump()?;
            let exp = self.factor()?; // right-associative
            if exp < 0.0 || exp.fract() != 0.0 {
                return Err(CoreError::InvalidInput(
                    "only non-negative integer exponents supported".into(),
                ));
            }
            let mut n = exp as u64;
            let mut result = 1.0;
            let mut b = base;
            while n > 0 {
                if n & 1 == 1 {
                    result *= b;
                }
                b *= b;
                n >>= 1;
            }
            return Ok(result);
        }
        Ok(base)
    }

    fn unary(&mut self) -> Result<f64, CoreError> {
        match self.cur {
            Tok::Plus => {
                self.bump()?;
                self.unary()
            }
            Tok::Minus => {
                self.bump()?;
                Ok(-self.unary()?)
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Result<f64, CoreError> {
        match self.cur.clone() {
            Tok::Num(v) => {
                self.bump()?;
                Ok(v)
            }
            Tok::LParen => {
                self.bump()?;
                let v = self.expr()?;
                match self.cur {
                    Tok::RParen => {
                        self.bump()?;
                        Ok(v)
                    }
                    _ => Err(CoreError::InvalidInput("missing ')'".into())),
                }
            }
            _ => Err(CoreError::InvalidInput("expected number or '('".into())),
        }
    }
}

/// Evaluate a basic arithmetic expression. No function calls, names, or
/// attribute access are possible by construction — only number tokens and
/// operators reach the parser.
pub fn calc(expression: &str) -> Result<f64, CoreError> {
    if expression.trim().is_empty() {
        return Err(CoreError::InvalidInput("empty expression".into()));
    }
    let mut p = Parser::new(expression)?;
    let v = p.expr()?;
    match p.cur {
        Tok::End => Ok(v),
        _ => Err(CoreError::InvalidInput("unexpected trailing input".into())),
    }
}

// ---------------------------------------------------------------------------
// Base64 (RFC 4648, stdlib-only)
// ---------------------------------------------------------------------------

const B64_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn b64_val(c: u8) -> Option<u32> {
    match c {
        b'A'..=b'Z' => Some((c - b'A') as u32),
        b'a'..=b'z' => Some((c - b'a' + 26) as u32),
        b'0'..=b'9' => Some((c - b'0' + 52) as u32),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

pub fn b64_encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let a = chunk[0] as u32;
        let b = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let c = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (a << 16) | (b << 8) | c;
        out.push(B64_TABLE[((triple >> 18) & 63) as usize] as char);
        out.push(B64_TABLE[((triple >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { B64_TABLE[((triple >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { B64_TABLE[(triple & 63) as usize] as char } else { '=' });
    }
    out
}

pub fn b64_decode(text: &str) -> Result<Vec<u8>, CoreError> {
    let clean: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if clean.len() % 4 != 0 {
        return Err(CoreError::InvalidInput("invalid base64 length".into()));
    }
    let mut out = Vec::new();
    for quad in clean.chunks(4) {
        let mut v = [0u32; 4];
        let mut pad = 0;
        for (i, &c) in quad.iter().enumerate() {
            if c == b'=' {
                pad += 1;
            } else {
                v[i] = b64_val(c)
                    .ok_or_else(|| CoreError::InvalidInput("invalid base64 character".into()))?;
            }
        }
        let triple = (v[0] << 18) | (v[1] << 12) | (v[2] << 6) | v[3];
        out.push(((triple >> 16) & 0xFF) as u8);
        if pad < 2 {
            out.push(((triple >> 8) & 0xFF) as u8);
        }
        if pad < 1 {
            out.push((triple & 0xFF) as u8);
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// UUID v4, timestamps, file info
// ---------------------------------------------------------------------------

fn random_bytes(n: usize) -> Vec<u8> {
    // Prefer OS entropy with a BOUNDED read; fall back to a time-seeded
    // xorshift (still unique per call, just not cryptographic) so targets
    // without /dev/urandom keep working. NOTE: never use fs::read on
    // /dev/urandom — it is infinite and would hang/OOM.
    if let Ok(mut f) = fs::File::open("/dev/urandom") {
        let mut buf = vec![0u8; n];
        if f.read_exact(&mut buf).is_ok() {
            return buf;
        }
    }
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
        .unwrap_or(0x853c49e6748fea9b);
    let mut x = seed | 1;
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        out.extend_from_slice(&x.to_le_bytes());
    }
    out.truncate(n);
    out
}

/// Random UUID v4, e.g. `550e8400-e29b-41d4-a716-446655440000`.
pub fn uuid4() -> String {
    let mut b = random_bytes(16);
    b[6] = (b[6] & 0x0F) | 0x40;
    b[8] = (b[8] & 0x3F) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12],
        b[13], b[14], b[15]
    )
}

/// Seconds since the Unix epoch (UTC).
pub fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// UTC ISO-8601 for a Unix timestamp, e.g. `2026-09-24T18:52:00Z`.
/// Civil-from-days algorithm (Howard Hinnant), no libc calls needed.
pub fn iso_from_unix(ts: u64) -> String {
    let days = (ts / 86_400) as i64;
    let secs = (ts % 86_400) as i64;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    y += if m <= 2 { 1 } else { 0 };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

/// Current UTC time as ISO-8601.
pub fn iso_now() -> String {
    iso_from_unix(unix_now())
}

/// (exists, size_bytes or -1, line_count or -1).
pub fn file_info(path: &Path) -> (bool, i64, i64) {
    let text = match fs::read(path) {
        Ok(b) => b,
        Err(_) => return (false, -1, -1),
    };
    let lines = text.iter().filter(|&&b| b == b'\n').count() as i64;
    (true, text.len() as i64, lines)
}

// ---------------------------------------------------------------------------
// Tests (run with `cargo test`)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calc_basic() {
        assert_eq!(calc("2 + 3 * 4").unwrap(), 14.0);
        assert_eq!(calc("(2 + 3) * 4").unwrap(), 20.0);
        assert_eq!(calc("2 ** 10").unwrap(), 1024.0);
        assert_eq!(calc("7 // 2").unwrap(), 3.0);
        assert_eq!(calc("-3 + +4").unwrap(), 1.0);
    }

    #[test]
    fn calc_rejects_junk() {
        assert!(calc("").is_err());
        assert!(calc("__import__('os')").is_err());
        assert!(calc("1/0").is_err());
        assert!(calc("2 +").is_err());
    }

    #[test]
    fn b64_roundtrip() {
        assert_eq!(b64_encode(b"hello"), "aGVsbG8=");
        assert_eq!(b64_decode("aGVsbG8=").unwrap(), b"hello");
        assert!(b64_decode("!!!").is_err());
    }

    #[test]
    fn uuid_shape() {
        let id = uuid4();
        assert_eq!(id.len(), 36);
        // version nibble == '4', variant nibble in 8/9/a/b
        assert_eq!(id.chars().nth(14).unwrap(), '4');
        assert!(matches!(id.chars().nth(19).unwrap(), '8' | '9' | 'a' | 'b'));
    }

    #[test]
    fn manifest_scan_smoke() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        let tools = discover_tools(&root);
        assert!(tools.iter().any(|t| t.name == "calc"));
    }
}
