//! Tools view — search + daftar grup + navigasi (mock Tools view).

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
        "tools",
        &format!(
            "{} navigate  {} run  {} search  {} back",
            kbd("↑↓"),
            kbd("Enter"),
            kbd("/"),
            kbd("Esc"),
        ),
    );
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(body);
    let prompt = if app.searching {
        format!("Search: {}_", app.tools_query)
    } else {
        "Search: (press /)".into()
    };
    frame.render_widget(Paragraph::new(Line::from(Span::styled(prompt, theme::dim()))), rows[0]);

    let vis = app.visible_tools();
    let mut lines: Vec<Line> = Vec::new();
    let mut last_group = String::new();
    let mut sel_line = 0usize;
    for (pos, &i) in vis.iter().enumerate() {
        let t = &app.tools[i];
        if t.group != last_group {
            if !lines.is_empty() {
                lines.push(Line::from(""));
            }
            lines.push(Line::from(Span::styled(t.group.to_uppercase(), theme::dim())));
            lines.push(Line::from(""));
            last_group = t.group.clone();
        }
        let sel = pos == app.cursor;
        if sel {
            sel_line = lines.len();
        }
        // Runtime rata kanan kolom 72 agar sejajar di semua baris.
        let runtime = format!("{:>10}", t.runtime);
        lines.push(Line::from(vec![
            sel_marker(sel),
            Span::styled(format!("{:<20}", t.name), row_style(sel)),
            Span::styled(format!("{:<34} ", t.desc), if sel { row_style(true) } else { theme::dim() }),
            Span::styled(runtime, if sel { row_style(true) } else { theme::dim() }),
        ]));
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled("no tools match", theme::dim())));
    }
    // Jendela scroll mengikuti kursor.
    let h = rows[1].height as usize;
    let start = sel_line.saturating_sub(h / 2).min(lines.len().saturating_sub(1));
    let end = (start + h).min(lines.len());
    frame.render_widget(Paragraph::new(lines[start..end].to_vec()), rows[1]);
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
        let mut app = App::new(&root);
        app.switch_view(crate::app::View::Tools);
        app
    }

    #[test]
    fn tools_golden_lists_groups() {
        let mut app = test_app();
        // Kursor ke system-info: uji scroll window ikut.
        let vis = app.visible_tools();
        let pos = vis.iter().position(|&i| app.tools[i].name == "system-info").unwrap();
        app.cursor = pos;
        let backend = TestBackend::new(100, 44);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, &app)).unwrap();
        let text = crate::views::screen_text(&mut term, 100, 44);
        for expect in ["Search:", "SYSTEM-TOOLS", "system-info", "▌ system-info"] {
            assert!(text.contains(expect), "missing {expect}\n{text}");
        }
    }
}
