//! Help overlay — read-only render of the keymap, grouped by category.
//! No state, no input handling beyond "any key returns to Tasks" (that
//! transition is handled in the App's global handler).

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

const HELP: &[(&str, &[(&str, &str)])] = &[
    (
        "Navigation",
        &[
            ("j / Down", "move down"),
            ("k / Up", "move up"),
            ("h / Left", "previous (week / field)"),
            ("l / Right", "next (week / field)"),
            ("gg", "top"),
            ("G", "bottom"),
            ("Tab", "cycle view / modal field / report scope"),
        ],
    ),
    (
        "View",
        &[
            ("g", "Tasks (home)"),
            ("a", "Agenda"),
            ("r", "Report"),
            ("?", "Help"),
            ("q / Esc", "quit / close overlay"),
        ],
    ),
    (
        "Tasks",
        &[
            ("n", "new task"),
            ("s / Enter", "start selected"),
            ("S", "stop active timer"),
            ("d", "mark done"),
            ("A", "archive"),
            ("D", "delete (with confirm)"),
            ("e", "edit task"),
            ("/", "filter"),
        ],
    ),
    (
        "Report",
        &[
            ("Tab", "cycle scope"),
            ("T", "cycle grouping"),
            ("c", "copy summary to clipboard"),
            ("j", "toggle JSON dump"),
        ],
    ),
    (
        "Edit overlay",
        &[
            ("Tab", "cycle fields"),
            ("Enter", "save"),
            ("Ctrl+D", "delete entry"),
            ("Esc", "cancel"),
        ],
    ),
];

pub fn render(frame: &mut Frame<'_>, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Help");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    for (title, rows) in HELP {
        lines.push(Line::from(Span::styled(
            (*title).to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for (key, desc) in *rows {
            lines.push(Line::from(format!("  {key:<14}{desc}")));
        }
        lines.push(Line::from(""));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn buffer_text(width: u16, height: u16, area: Rect) -> String {
        let backend = TestBackend::new(width, height);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|frame| render(frame, area)).unwrap();
        let buf = term.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..buf.area().height {
            for x in 0..buf.area().width {
                out.push_str(buf.cell((x, y)).unwrap().symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn help_text_includes_all_categories() {
        let out = buffer_text(80, 40, Rect::new(0, 0, 80, 40));
        assert!(out.contains("Navigation"), "got:\n{out}");
        assert!(out.contains("Tasks"));
        assert!(out.contains("Report"));
        assert!(out.contains("Edit overlay"));
    }

    #[test]
    fn help_lists_core_shortcuts() {
        let out = buffer_text(80, 40, Rect::new(0, 0, 80, 40));
        assert!(out.contains("j / Down"));
        assert!(out.contains("Ctrl+D"));
        assert!(out.contains("toggle JSON"));
    }
}
