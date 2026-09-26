//! Modules view + detail (mock Module manager).

use ratatui::{
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::{chrome, kbd, row_style, sel_marker};
use crate::{app::App, theme};

pub fn render(frame: &mut Frame, app: &App) {
    let body = chrome(
        frame,
        app,
        frame.area(),
        "modules",
        &format!(
            "{} details  {} enable/disable  {} back",
            kbd("Enter"),
            kbd("Space"),
            kbd("Esc"),
        ),
    );
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(body);
    let enabled = app.modules.iter().filter(|m| m.enabled).count();
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{} installed", app.modules.len()), theme::dim()),
            Span::styled("   ", theme::dim()),
            Span::styled(format!("{} enabled", enabled), theme::normal()),
        ])),
        rows[0],
    );
    let mut lines = Vec::new();
    for (pos, m) in app.modules.iter().enumerate() {
        let sel = pos == app.cursor;
        let dot = if m.enabled { "●" } else { "○" };
        let state = if m.enabled { "enabled" } else { "disabled" };
        lines.push(Line::from(vec![
            sel_marker(sel),
            Span::styled(format!("{dot} "), theme::accent()),
            Span::styled(format!("{:<16}", m.name), row_style(sel)),
            Span::styled(format!("{:<10} ", m.runtime), theme::dim()),
            Span::styled(format!("{:<8} ", state), if sel { row_style(true) } else { theme::dim() }),
            Span::styled(format!("v{}", m.version), theme::dim()),
        ]));
    }
    frame.render_widget(Paragraph::new(lines), rows[1]);
}

pub fn render_detail(frame: &mut Frame, app: &App, name: &str) {
    let body = chrome(frame, app, frame.area(), "module", "e Edit   r Reload   d Disable   Esc Back");
    let Some(m) = app.modules.iter().find(|x| x.name == name) else {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled("unknown module", theme::dim()))),
            body,
        );
        return;
    };
    let tools: Vec<&str> = app.tools.iter().filter(|t| t.module == m.name).map(|t| t.name.as_str()).collect();
    let mut lines = vec![
        Line::from(Span::styled(m.name.to_uppercase(), theme::accent())),
        Line::from(Span::styled(m.description.clone(), theme::dim())),
        Line::from(""),
        Line::from(vec![
            Span::styled("Status        ", theme::dim()),
            Span::styled(
                if m.enabled { "● Enabled" } else { "○ Disabled" },
                theme::normal(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Version       ", theme::dim()),
            Span::styled(m.version.clone(), theme::normal()),
        ]),
        Line::from(vec![
            Span::styled("Runtime       ", theme::dim()),
            Span::styled(m.runtime.clone(), theme::normal()),
        ]),
        Line::from(""),
        Line::from(Span::styled("COMMANDS", theme::dim())),
        Line::from(""),
    ];
    for t in tools {
        lines.push(Line::from(Span::styled(format!("  {t}"), theme::normal())));
    }
    if lines.len() <= 10 {
        lines.push(Line::from(Span::styled("  (no tools yet)", theme::dim())));
    }
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
    fn modules_golden_counts() {
        let mut app = test_app();
        app.switch_view(crate::app::View::Modules);
        let backend = TestBackend::new(80, 24);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, &app)).unwrap();
        let text = crate::views::screen_text(&mut term, 80, 24);
        for expect in ["5 installed", "4 enabled", "system-tools", "school-tools", "disabled", "▌ "] {
            assert!(text.contains(expect), "missing {expect}\n{text}");
        }
    }

    #[test]
    fn detail_golden_school_empty() {
        let app = test_app();
        let backend = TestBackend::new(80, 24);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render_detail(f, &app, "school-tools")).unwrap();
        let text = crate::views::screen_text(&mut term, 80, 24);
        assert!(text.contains("no tools yet"), "{text}");
    }
}
