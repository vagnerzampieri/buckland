//! Header + footer chrome. Pure rendering — no state mutations, no I/O.
//! The header shows the active timer or "idle"; the footer shows the
//! per-screen shortcut hints plus any transient info/error message.

use crate::cli::format::duration_hms;
use crate::storage::Repo;
use crate::tui::app::{ActiveTimer, App, FooterMessage, ScreenKind};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render_header<R: Repo>(frame: &mut Frame<'_>, area: Rect, app: &App<R>) {
    let line = match &app.active_timer {
        Some(t) => header_active_line(t, app),
        None => Line::from(vec![Span::styled(
            "Buckland — idle",
            Style::default().add_modifier(Modifier::DIM),
        )]),
    };
    frame.render_widget(Paragraph::new(line), area);
}

fn header_active_line<R: Repo>(timer: &ActiveTimer, app: &App<R>) -> Line<'static> {
    let elapsed = duration_hms(timer.elapsed(app.now));
    let label = match timer.shortcut_external_id {
        Some(ext) => format!("SC-{ext} {}", timer.task_title),
        None => timer.task_title.clone(),
    };
    Line::from(vec![
        Span::styled("▶ ", app.theme.running()),
        Span::raw(label),
        Span::raw(" — "),
        Span::styled(elapsed, app.theme.total()),
    ])
}

pub fn render_footer<R: Repo>(frame: &mut Frame<'_>, area: Rect, app: &App<R>) {
    let hint = match app.screen {
        ScreenKind::Tasks => "j/k move • s start • S stop • n new • d done • A archive • D delete • e edit • / filter • ? help • q quit",
        ScreenKind::Agenda => "j/k move • h/l week • enter edit • D delete • g tasks • r report • ? help • q quit",
        ScreenKind::Report => "Tab scope • T group • c copy • j json • g tasks • a agenda • ? help • q quit",
        ScreenKind::Help => "any key to return",
    };
    let line = match &app.footer {
        FooterMessage::Empty => Line::from(Span::styled(hint, app.theme.dim())),
        FooterMessage::Info(msg) => Line::from(vec![
            Span::styled("ℹ ", app.theme.dim()),
            Span::raw(msg.clone()),
        ]),
        FooterMessage::Error(msg) => Line::from(vec![
            Span::styled("✗ ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(msg.clone()),
        ]),
    };
    frame.render_widget(Paragraph::new(line), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SqliteRepo;
    use crate::tui::app::{ActiveTimer, App};
    use crate::tui::theme::Theme;
    use chrono::{TimeZone, Utc};
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use ratatui::Terminal;

    fn render<F: FnOnce(&mut Frame<'_>)>(width: u16, height: u16, f: F) -> String {
        let backend = TestBackend::new(width, height);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|frame| f(frame)).unwrap();
        let buffer = term.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..buffer.area().height {
            for x in 0..buffer.area().width {
                out.push_str(buffer.cell((x, y)).unwrap().symbol());
            }
            out.push('\n');
        }
        out
    }

    fn app_idle() -> App<SqliteRepo> {
        let now = Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap();
        App::new(
            SqliteRepo::in_memory(),
            Theme::from_config_accent("cyan"),
            now,
        )
    }

    #[test]
    fn header_says_idle_when_no_active_timer() {
        let app = app_idle();
        let out = render(40, 1, |f| render_header(f, Rect::new(0, 0, 40, 1), &app));
        assert!(out.contains("idle"), "got buffer:\n{out}");
    }

    #[test]
    fn header_shows_active_task_label_and_elapsed() {
        let mut app = app_idle();
        app.active_timer = Some(ActiveTimer {
            task_id: 1,
            task_title: "fix login".into(),
            shortcut_external_id: None,
            started_at: app.now,
        });
        // Advance the clock by 1h 5m 12s.
        app.now += chrono::Duration::seconds(3912);
        let out = render(60, 1, |f| render_header(f, Rect::new(0, 0, 60, 1), &app));
        assert!(out.contains("fix login"));
        assert!(out.contains("01:05:12"), "elapsed missing in:\n{out}");
        assert!(out.contains('▶'));
    }

    #[test]
    fn header_prefixes_sc_id_when_linked() {
        let mut app = app_idle();
        app.active_timer = Some(ActiveTimer {
            task_id: 1,
            task_title: "fix login".into(),
            shortcut_external_id: Some(123),
            started_at: app.now,
        });
        let out = render(60, 1, |f| render_header(f, Rect::new(0, 0, 60, 1), &app));
        assert!(out.contains("SC-123 fix login"), "got:\n{out}");
    }

    #[test]
    fn footer_shows_screen_hint_when_idle() {
        let app = app_idle();
        let out = render(120, 1, |f| render_footer(f, Rect::new(0, 0, 120, 1), &app));
        assert!(out.contains("j/k"));
        assert!(out.contains("? help"));
    }

    #[test]
    fn footer_shows_error_message_with_marker() {
        let mut app = app_idle();
        app.error("can't delete");
        let out = render(60, 1, |f| render_footer(f, Rect::new(0, 0, 60, 1), &app));
        assert!(out.contains("can't delete"));
        assert!(out.contains('✗'));
    }
}
