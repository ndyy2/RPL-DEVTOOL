//! Main view — OVERVIEW (system + recent) + MODULES ringkas.
//!
//! Border tipis + whitespace, tanpa kotak bersarang (sesuai mock).

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::{chrome, section};
use crate::{app::App, theme};

fn bar(pct: f32, width: usize) -> String {
    let fill = ((pct / 100.0 * width as f32).round() as usize).min(width);
    format!("{}{}", "█".repeat(fill), "░".repeat(width - fill))
}

pub fn render(frame: &mut Frame, app: &App) {
    let body = chrome(
        frame,
        app,
        frame.area(),
        "main",
        "[1] Tools   [2] Modules   [3] Logs   [?] Help   [q] Quit   [Ctrl+K] Palette",
    );
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(8), Constraint::Length(1)])
        .split(body);
    section(frame, rows[0], "OVERVIEW");
    render_overview(frame, app, rows[1]);
}

fn render_overview(frame: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(area);
    // Kiri: SYSTEM (snapshot atau placeholder).
    let mut sys_lines = vec![Line::from(Span::styled("SYSTEM", theme::dim()))];
    if let Some(s) = &app.sys {
        let cpu = s.global_cpu_pct;
        let mem_pct = if s.mem.total > 0 { s.mem.used as f32 * 100.0 / s.mem.total as f32 } else { 0.0 };
        let (d_used, d_total) = s
            .disks
            .first()
            .map(|d| (d.total.saturating_sub(d.available), d.total))
            .unwrap_or((0, 1));
        let disk_pct = d_used as f32 * 100.0 / d_total.max(1) as f32;
        sys_lines.push(Line::from(vec![
            Span::styled(format!("CPU     {cpu:5.1}%  "), theme::normal()),
            Span::styled(bar(cpu, 12), theme::accent()),
        ]));
        sys_lines.push(Line::from(vec![
            Span::styled(format!("Memory  {mem_pct:5.1}%  "), theme::normal()),
            Span::styled(bar(mem_pct, 12), theme::accent()),
        ]));
        sys_lines.push(Line::from(vec![
            Span::styled(format!("Disk    {disk_pct:5.1}%  "), theme::normal()),
            Span::styled(bar(disk_pct, 12), theme::accent()),
        ]));
    } else {
        sys_lines.push(Line::from(Span::styled("collecting…", theme::dim())));
    }
    sys_lines.push(Line::from(""));
    sys_lines.push(Line::from(Span::styled(
        format!("{} modules · {} tools", app.modules.len(), app.tools.len()),
        theme::dim(),
    )));
    frame.render_widget(Paragraph::new(sys_lines), cols[0]);

    // Kanan: RECENT (history, max 5) + MODULES ringkas.
    let mut right = vec![Line::from(Span::styled("RECENT", theme::dim()))];
    if app.recent.is_empty() {
        right.push(Line::from(Span::styled("no runs yet — press 1, pick a tool", theme::dim())));
    } else {
        let now = rplkit_runtime::history::now_unix();
        for r in app.recent.iter().take(4) {
            right.push(Line::from(vec![
                Span::styled("◇ ", theme::accent()),
                Span::styled(format!("{:<16}", r.tool), theme::normal()),
                Span::styled(rplkit_runtime::history::ago_text(now, r.at), theme::dim()),
            ]));
        }
    }
    right.push(Line::from(""));
    right.push(Line::from(Span::styled("MODULES", theme::dim())));
    for m in app.modules.iter().take(4) {
        right.push(Line::from(vec![
            Span::styled(if m.enabled { "● " } else { "○ " }, theme::accent()),
            Span::styled(format!("{:<16} {}", m.name, m.runtime), theme::normal()),
        ]));
    }
    frame.render_widget(Paragraph::new(right), cols[1]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn test_app() -> App {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..");
        App::new(&root)
    }

    #[test]
    fn main_golden_contains_sections() {
        let app = test_app();
        let backend = TestBackend::new(80, 24);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, &app)).unwrap();
        let text = crate::views::screen_text(&mut term, 80, 24);
        for expect in ["RPLKIT", "OVERVIEW", "RECENT", "MODULES", "SYSTEM", "[1] Tools"] {
            assert!(text.contains(expect), "missing {expect}\n{text}");
        }
    }
}
