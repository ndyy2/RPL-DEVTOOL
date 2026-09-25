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

/// Chrome luar: header (nama + versi + status) + footer keybindings.
pub fn chrome(frame: &mut Frame, app: &App, area: Rect, title: &str, keys: &str) -> Rect {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
        .split(area);
    let version = env!("CARGO_PKG_VERSION");
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("RPLKIT", theme::accent()),
            Span::styled(format!("  v{version}  "), theme::dim()),
            Span::styled("● READY", theme::accent()),
        ]),
        Line::from(Span::styled("Developer Toolkit", theme::dim())),
    ])
    .block(Block::default().borders(Borders::BOTTOM).border_style(theme::border(false)));
    frame.render_widget(header, rows[0]);
    let footer = Paragraph::new(Line::from(Span::styled(keys, theme::dim()))).block(
        Block::default().borders(Borders::TOP).border_style(theme::border(false)),
    );
    frame.render_widget(footer, rows[2]);
    let _ = title;
    rows[1]
}

/// Section header tipis gaya mock ("SYSTEM" + garis).
pub fn section(frame: &mut Frame, area: Rect, title: &str) {
    let line = Paragraph::new(Line::from(vec![
        Span::styled(format!("{title}  "), theme::normal()),
        Span::styled("─".repeat(area.width.saturating_sub(title.len() as u16 + 4) as usize), theme::dim()),
    ]));
    frame.render_widget(line, area);
}

/// Gaya baris daftar: terpilih = aksen, biasa = normal.
pub fn row_style(selected: bool) -> Style {
    if selected {
        theme::selected()
    } else {
        theme::normal()
    }
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
