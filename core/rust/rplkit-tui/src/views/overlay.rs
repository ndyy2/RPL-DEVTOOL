//! Palette (Ctrl+K) + Logs + Help views.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::{chrome, kbd, row_style, sel_marker};
use crate::{app::App, theme};

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect::new(x, y, w.min(area.width), h.min(area.height))
}

pub fn render_palette(frame: &mut Frame, app: &App) {
    let area = centered(frame.area(), 72, 20);
    frame.render_widget(Clear, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("> ", theme::accent()),
            Span::styled(format!("{}_", app.palette_query), theme::normal()),
        ])),
        rows[0],
    );
    let items = app.palette_items();
    let mut lines = vec![Line::from(Span::styled("RUN", theme::dim())), Line::from("")];
    let mut shown = 0;
    for (pos, (label, kind)) in items.iter().enumerate() {
        if kind.starts_with("action:") {
            continue;
        }
        if shown >= 6 {
            break;
        }
        shown += 1;
        let sel = pos == app.palette_cursor;
        lines.push(Line::from(vec![
            sel_marker(sel),
            Span::styled("◇ ", theme::accent()),
            Span::styled(label.clone(), row_style(sel)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("ACTIONS", theme::dim())));
    lines.push(Line::from(""));
    for (label, kind) in items.iter().filter(|(_, k)| k.starts_with("action:")) {
        lines.push(Line::from(vec![
            Span::styled("  ", theme::dim()),
            Span::styled(label.clone(), theme::normal()),
        ]));
        let _ = kind;
    }
    frame.render_widget(Paragraph::new(lines), rows[1]);
}

pub fn render_logs(frame: &mut Frame, app: &App) {
    let body = chrome(
        frame,
        app,
        frame.area(),
        "logs",
        &format!("{} back  {} quit", kbd("Esc"), kbd("q")),
    );
    let mut lines = vec![Line::from(Span::styled("SESSION LOG", theme::dim())), Line::from("")];
    if app.session_log.is_empty() {
        lines.push(Line::from(Span::styled("no runs this session", theme::dim())));
    }
    for (pos, r) in app.session_log.iter().enumerate() {
        let sel = pos == app.cursor;
        let mark = if r.code == 0 { "✓" } else { "✗" };
        lines.push(Line::from(vec![
            sel_marker(sel),
            Span::styled(format!("{mark} "), theme::normal()),
            Span::styled(format!("{:<16}", r.tool), row_style(sel)),
            Span::styled(
                format!("{:>6}ms  {:<8}  {}", r.ms, r.by, r.args.join(" ")),
                theme::dim(),
            ),
        ]));
    }
    frame.render_widget(Paragraph::new(lines), body);
}

pub fn render_help(frame: &mut Frame, app: &App) {
    let body = chrome(
        frame,
        app,
        frame.area(),
        "help",
        &format!("{} back  {} quit", kbd("Esc"), kbd("q")),
    );
    let key = |k: &str, desc: &str| {
        Line::from(vec![
            Span::styled(format!("  {:<22}", kbd(k)), theme::normal()),
            Span::styled(desc.to_string(), theme::dim()),
        ])
    };
    let lines = vec![
        Line::from(Span::styled("KEYS", theme::dim())),
        Line::from(""),
        key("1 / 2 / 3", "Tools · Modules · Logs"),
        key("↑↓", "Navigate"),
        key("Enter", "Run / open details"),
        key("/", "Search tools"),
        key("Space", "Enable / disable module"),
        key("Ctrl+K", "Command palette"),
        key("Ctrl+R / Ctrl+Y", "(Exec) run again / copy output"),
        key("Esc", "Back"),
        key("q", "Quit (outside text input; Esc first)"),
        Line::from(""),
        Line::from(Span::styled("Two modes: CLI for scripting, TUI for interactive work.", theme::dim())),
    ];
    frame.render_widget(Paragraph::new(lines), body);
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
    fn palette_golden_sections() {
        let mut app = test_app();
        app.palette_open = true;
        app.palette_query = "calc".into();
        let backend = TestBackend::new(76, 22);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render_palette(f, &app)).unwrap();
        let text = crate::views::screen_text(&mut term, 76, 22);
        for expect in ["> calc", "RUN", "calculator", "ACTIONS"] {
            assert!(text.contains(expect), "missing {expect}\n{text}");
        }
    }

    #[test]
    fn help_golden_keys() {
        let app = test_app();
        let backend = TestBackend::new(76, 22);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render_help(f, &app)).unwrap();
        let text = crate::views::screen_text(&mut term, 76, 22);
        assert!(text.contains("Ctrl+K"), "{text}");
    }
}
