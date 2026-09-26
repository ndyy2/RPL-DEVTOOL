//! Exec view — jalankan tool + tampilkan output + timing (mock Exec view).

use ratatui::{
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::{chrome, kbd};
use crate::{app::App, theme};

pub fn render(frame: &mut Frame, app: &App) {
    let body = chrome(
        frame,
        app,
        frame.area(),
        "exec",
        &format!(
            "type args + {} run  {} again  {} copy",
            kbd("Enter"),
            kbd("Ctrl+R"),
            kbd("Ctrl+Y"),
        ),
    );
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)])
        .split(body);
    let e = &app.exec;
    let mut head = vec![Line::from(vec![
        Span::styled("$ ", theme::accent()),
        Span::styled(format!("rplkit {} ", e.tool), theme::normal()),
        Span::styled(format!("{}_", e.args_line), theme::normal()),
    ])];
    if let Some(code) = e.code {
        let mark = if code == 0 { "✓" } else { "✗" };
        let word = if code == 0 { "Completed" } else { "Failed" };
        head.push(Line::from(vec![
            Span::styled(format!("{mark} {word}"), theme::normal()),
            Span::styled(format!("   {}ms · {}", e.ms, e.by), theme::dim()),
        ]));
    }
    frame.render_widget(Paragraph::new(head), rows[0]);
    let out = if e.output.is_empty() {
        "output appears here after Enter".to_string()
    } else {
        e.output.clone()
    };
    let lines: Vec<Line> = out.lines().map(|l| Line::from(Span::styled(l.to_string(), theme::normal()))).collect();
    frame.render_widget(Paragraph::new(lines), rows[1]);
    let tail = if e.copied { "copied ✓" } else { "" };
    frame.render_widget(Paragraph::new(Line::from(Span::styled(tail, theme::dim()))), rows[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn exec_golden_shows_prompt() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..");
        let mut app = App::new(&root);
        app.exec.tool = "calculator".into();
        app.exec.args_line = "2+2".into();
        let backend = TestBackend::new(80, 24);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, &app)).unwrap();
        let text = crate::views::screen_text(&mut term, 80, 24);
        assert!(text.contains("rplkit calculator"), "{text}");
        assert!(text.contains("Ctrl+Y"), "{text}");
    }
}
