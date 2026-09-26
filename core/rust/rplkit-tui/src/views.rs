//! views — rendering per layar (Fase 4b).
//!
//! Aturan visual (mock + kontrak tema): border tipis + whitespace,
//! TANPA kotak-di-dalam-kotak berlebihan. Monokrom + aksen Blue Iris
//! hanya untuk selection/focus/status.

pub mod main;
pub mod tools;
pub mod exec;
pub mod modules;
pub mod overlay;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::{app::App, theme};

/// Chrome luar: header satu baris (nama kiri, status kanan) + footer
/// keybinding terkelompok. Subjudul "Developer Toolkit" dihapus —
/// nama + versi + status cukup (Fase 2 rafinasi).
pub fn chrome(frame: &mut Frame, _app: &App, area: Rect, title: &str, keys: &str) -> Rect {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0), Constraint::Length(2)])
        .split(area);
    let version = env!("CARGO_PKG_VERSION");
    let left = format!("RPLKIT  v{version}");
    let right = "● READY";
    let mid = area.width.saturating_sub(left.len() as u16 + right.len() as u16 + 2) as usize;
    let header = Paragraph::new(Line::from(vec![
        Span::styled(left, theme::accent()),
        Span::raw(" ".repeat(mid)),
        Span::styled(right, theme::accent()),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(theme::border(false)));
    frame.render_widget(header, rows[0]);
    let footer = Paragraph::new(Line::from(Span::styled(keys, theme::dim()))).block(
        Block::default().borders(Borders::TOP).border_style(theme::border(false)),
    );
    frame.render_widget(footer, rows[2]);
    let _ = title;
    rows[1]
}

/// Section header tipis: judul + garis proporsional (tak penuh lebar).
pub fn section(frame: &mut Frame, area: Rect, title: &str) {
    let dashes = (area.width as usize)
        .saturating_sub(title.len() + 4)
        .min(24)
        .max(4);
    let line = Paragraph::new(Line::from(vec![
        Span::styled(format!("{title}  "), theme::normal()),
        Span::styled("─".repeat(dashes), theme::dim()),
    ]));
    frame.render_widget(line, area);
}

/// Gaya baris daftar: terpilih = teks terang + bold, latar tetap gelap.
/// Lihat theme::selected — blok aksen penuh dihapus (Fase 1 rafinasi).
pub fn row_style(selected: bool) -> Style {
    if selected {
        theme::selected()
    } else {
        theme::normal()
    }
}

/// Marker 2-kolom sisi kiri baris: "▌" aksen bila terpilih, spasi bila tidak.
/// Lebar konstan menjaga kolom teks sejajar di semua baris.
pub fn sel_marker(selected: bool) -> Span<'static> {
    if selected {
        Span::styled("▌ ", theme::select_bar())
    } else {
        Span::raw("  ")
    }
}

/// Tombol keyboard terbalik: `▏Enter▕` — konsisten di semua footer/help.
pub fn kbd(label: &str) -> String {
    format!("▏{label}▕")
}

/// Dispatch render sesuai view aktif + overlay palette.
pub fn render(frame: &mut Frame, app: &App) {
    use super::app::View;
    match &app.view {
        View::Main => main::render(frame, app),
        View::Tools => tools::render(frame, app),
        View::Exec => exec::render(frame, app),
        View::Modules => modules::render(frame, app),
        View::ModuleDetail(name) => modules::render_detail(frame, app, name),
        View::Logs => overlay::render_logs(frame, app),
        View::Help => overlay::render_help(frame, app),
        View::Palette => main::render(frame, app),
    }
    if app.palette_open {
        overlay::render_palette(frame, app);
    }
}

/// Dump buffer TestBackend menjadi teks (untuk golden test).
#[cfg(test)]
pub fn screen_text(term: &mut ratatui::Terminal<ratatui::backend::TestBackend>, w: u16, h: u16) -> String {
    let mut text = String::new();
    let buf = term.backend().buffer().clone();
    for y in 0..h {
        for x in 0..w {
            text.push_str(buf[(x, y)].symbol());
        }
        text.push('\n');
    }
    text
}
