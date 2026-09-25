//! rplkit-tui — terminal workspace RPLKit.
//!
//! Monokrom + SATU aksen (Blue Iris). Aturan keras: tidak ada warna kedua;
//! sukses/gagal dibedakan simbol + teks, bukan warna. Lihat [`theme`].

pub mod theme;
pub mod app;
pub mod views;

pub use app::App;
use std::{io, path::Path, time::Duration};

/// Jalankan TUI fullscreen. Return exit code proses.
/// Ctrl+C diperlakukan seperti `q` (keluar bersih, terminal direstore).
pub fn run(repo_root: &Path) -> io::Result<i32> {
    use crossterm::{
        event::{self, Event},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::{backend::CrosstermBackend, Terminal};

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let mut app = App::new(repo_root);
    app.tick_sys();
    let mut ticks = 0u32;
    let code = loop {
        term.draw(|f| views::render(f, &app))?;
        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) => {
                    if app.on_key(key) {
                        break 0;
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
        ticks += 1;
        if ticks % 8 == 0 {
            // Refresh SYSTEM ~tiap 2 detik.
            app.tick_sys();
        }
        if app.should_quit {
            break 0;
        }
    };

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    term.show_cursor()?;
    Ok(code)
}

/// Salin teks ke clipboard via OSC52 (fallback: kembalikan false agar
/// pemanggil menampilkan pesan jujur, bukan diam).
pub fn copy_osc52(text: &str) -> bool {
    use std::io::Write as _;
    if text.is_empty() {
        return false;
    }
    let mut out = io::stdout();
    // OSC52 membutuhkan base64 — encode manual (stdlib-only di crate ini).
    const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = text.as_bytes();
    let mut enc = String::new();
    for chunk in bytes.chunks(3) {
        let a = chunk[0] as u32;
        let b = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let c = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let t = (a << 16) | (b << 8) | c;
        enc.push(B64[((t >> 18) & 63) as usize] as char);
        enc.push(B64[((t >> 12) & 63) as usize] as char);
        enc.push(if chunk.len() > 1 { B64[((t >> 6) & 63) as usize] as char } else { '=' });
        enc.push(if chunk.len() > 2 { B64[(t & 63) as usize] as char } else { '=' });
    }
    write!(out, "\x1b]52;c;{enc}\x1b\\").is_ok() && out.flush().is_ok()
}
