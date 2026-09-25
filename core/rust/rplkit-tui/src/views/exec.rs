//! Exec view — jalankan tool + tampilkan output + timing (mock Exec view).

use ratatui::{
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::chrome;
use crate::{app::App, theme};

pub fn render(frame: &mut Frame, app: &App) {
    let body = chrome(
        frame,
        app,
        frame.area(),
        "exec",
        "Type args + Enter Run   Ctrl+R Again   Ctrl+Y Copy   Esc Back",
    );
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
        .split(body);
    let e = &app.exec;
    let mut head = vec![Line::from(vec![
        Span::styled(format!("$ rplkit {} ", e.tool), theme::accent()),
        Span::styled(format!("{}_", e.args_line), theme::normal()),
    ])];
    if let Some(code) = e.code {
        let status = if code == 0 { "✓ Completed" } else { "✗ Failed" };
        head.push(Line::from(""));
        head.push(Line::from(Span::styled(status, theme::normal())));
        head.push(Line::from(Span::styled(
            format!("Execution time: {}ms   executed_by: {}", e.ms, e.by),
            theme::dim(),
        )));
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
