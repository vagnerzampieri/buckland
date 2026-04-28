//! Buckland TUI — ratatui surface composed on top of the `Repo` trait.

pub mod app;
pub mod chrome;
pub mod clipboard;
pub mod keymap;
pub mod screens;
pub mod terminal;
pub mod theme;

use crate::cli::context::Context;
use crate::storage::{Repo, SqliteRepo};
use crate::tui::app::{App, ScreenKind};
use crate::tui::keymap::{resolve, resolve_after_pending_g, KeyAction};
use crate::tui::screens::edit::{EditOutcome, EditState};
use crate::tui::screens::tasks::TasksState;
use crate::tui::screens::{agenda, edit, help, report, tasks};
use crate::tui::terminal::TerminalGuard;
use crate::tui::theme::Theme;
use chrono::Utc;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub struct TuiSession {
    pub tasks: tasks::TasksState,
    pub agenda: agenda::AgendaState,
    pub report: report::ReportState,
    pub edit: Option<EditState>,
}

impl TuiSession {
    /// Constructs a session with each per-screen state seeded for `now`.
    /// `now` is the same wall-clock used by the owning `App` so the
    /// Agenda's week_start matches the App's reference time even in
    /// tests that pin a fictional `now`.
    pub fn new(now: chrono::DateTime<Utc>) -> Self {
        Self {
            tasks: TasksState::default(),
            agenda: agenda::AgendaState::for_now(now),
            report: report::ReportState::default(),
            edit: None,
        }
    }

    pub fn refresh(&mut self, app: &mut App<SqliteRepo>) -> anyhow::Result<()> {
        app.refresh_active_timer()?;
        self.tasks.refresh(&app.repo, app.now)?;
        self.agenda.refresh(&app.repo, app.now)?;
        self.report.refresh(&app.repo, app.now)?;
        Ok(())
    }

    pub fn dispatch(&mut self, app: &mut App<SqliteRepo>, action: KeyAction) {
        // Edit overlay (when active) sees keys first.
        if let Some(state) = self.edit.as_mut() {
            let outcome = edit::handle_key(state, app, action);
            match outcome {
                EditOutcome::Saved | EditOutcome::Deleted | EditOutcome::Cancelled => {
                    self.edit = None;
                    let _ = self.refresh(app);
                }
                EditOutcome::Continue => {}
            }
            return;
        }

        // Global handlers — quit, help, view-switch (g/a/r), `gg`.
        if app.handle_global(action) {
            return;
        }

        // Per-screen routing.
        match app.screen {
            ScreenKind::Tasks => {
                tasks::handle_key(&mut self.tasks, app, action);
            }
            ScreenKind::Agenda => {
                if let Some(intent) = agenda::handle_key(&mut self.agenda, app, action) {
                    match intent {
                        agenda::AgendaIntent::OpenEdit(entry_id) => self.open_edit(app, entry_id),
                    }
                }
            }
            ScreenKind::Report => {
                report::handle_key(&mut self.report, app, action);
            }
            ScreenKind::Help => {
                // Any key returns to Tasks. The global handler captures
                // Quit; for everything else we route here.
                app.screen = ScreenKind::Tasks;
            }
        }
    }

    fn open_edit(&mut self, app: &mut App<SqliteRepo>, entry_id: i64) {
        let Ok(entry_opt) = (|| -> anyhow::Result<_> {
            let mut entries = Vec::new();
            for row in app.repo.list_entries_in_range(
                app.now - chrono::Duration::days(365),
                app.now + chrono::Duration::days(1),
                app.now,
            )? {
                if row.id == entry_id {
                    entries.push(row);
                }
            }
            Ok(entries.into_iter().next())
        })() else {
            app.error("Could not load entry for editing.");
            return;
        };
        let Some(entry) = entry_opt else {
            app.error(format!("Entry #{entry_id} not found."));
            return;
        };
        let Ok(Some(task)) = app.repo.find_task(entry.task_id) else {
            app.error(format!("Task for entry #{entry_id} not found."));
            return;
        };
        self.edit = Some(EditState::for_entry(&entry, task.title));
    }
}

pub fn run(ctx: Context) -> anyhow::Result<i32> {
    let theme = Theme::from_config_accent(
        &crate::config::load(&crate::config::config_path())
            .map(|c| c.ui.accent_color)
            .unwrap_or_else(|_| "cyan".into()),
    );
    let now = Utc::now();
    let mut app = App::new(ctx.repo, theme, now);
    let mut session = TuiSession::new(now);
    session.refresh(&mut app)?;

    let mut guard = TerminalGuard::enter()?;
    let event_rx = spawn_event_thread();

    while !app.should_quit {
        guard.terminal().draw(|frame| {
            let area = frame.area();
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(area);
            chrome::render_header(frame, layout[0], &app);
            render_main(frame, layout[1], &app, &session);
            chrome::render_footer(frame, layout[2], &app);
        })?;

        match event_rx.recv() {
            Ok(LoopEvent::Tick) => {
                app.on_tick(Utc::now());
            }
            Ok(LoopEvent::Key(key)) => {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                let action = if app.pending_g {
                    resolve_after_pending_g(key)
                } else {
                    resolve(key)
                };
                session.dispatch(&mut app, action);
            }
            Err(_) => break,
        }
    }
    Ok(0)
}

fn render_main(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    app: &App<SqliteRepo>,
    session: &TuiSession,
) {
    if let Some(edit_state) = &session.edit {
        edit::render(frame, area, edit_state);
        return;
    }
    match app.screen {
        ScreenKind::Tasks => tasks::render(frame, area, app, &session.tasks),
        ScreenKind::Agenda => agenda::render(frame, area, app, &session.agenda),
        ScreenKind::Report => report::render(frame, area, app, &session.report),
        ScreenKind::Help => help::render(frame, area),
    }
}

enum LoopEvent {
    Tick,
    Key(KeyEvent),
}

fn spawn_event_thread() -> mpsc::Receiver<LoopEvent> {
    let (tx, rx) = mpsc::channel();
    let key_tx = tx.clone();
    thread::spawn(move || loop {
        if event::poll(Duration::from_millis(250)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key_tx.send(LoopEvent::Key(key)).is_err() {
                    return;
                }
            }
        }
    });
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(1));
        if tx.send(LoopEvent::Tick).is_err() {
            return;
        }
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Repo, SqliteRepo};
    use crate::tui::app::{App, ScreenKind};
    use crate::tui::keymap::KeyAction;
    use crate::tui::theme::Theme;
    use chrono::{TimeZone, Utc};

    fn at(h: u32) -> chrono::DateTime<chrono::Utc> {
        Utc.with_ymd_and_hms(2026, 4, 22, h, 0, 0).unwrap()
    }

    #[test]
    fn dispatch_handles_screen_switching_through_global() {
        let mut app = App::new(
            SqliteRepo::in_memory(),
            Theme::from_config_accent("cyan"),
            at(10),
        );
        let mut session = TuiSession::new(at(10));
        session.dispatch(&mut app, KeyAction::OpenAgenda);
        assert_eq!(app.screen, ScreenKind::Agenda);
        session.dispatch(&mut app, KeyAction::OpenReport);
        assert_eq!(app.screen, ScreenKind::Report);
        session.dispatch(&mut app, KeyAction::Help);
        assert_eq!(app.screen, ScreenKind::Help);
    }

    #[test]
    fn dispatch_routes_tasks_keys_to_tasks_screen() {
        let mut app = App::new(
            SqliteRepo::in_memory(),
            Theme::from_config_accent("cyan"),
            at(10),
        );
        app.repo.create_task("alpha", None).unwrap();
        let mut session = TuiSession::new(at(10));
        session.refresh(&mut app).unwrap();
        // Down on Tasks moves the cursor.
        session.dispatch(&mut app, KeyAction::Down);
        // Selection clamps at len-1 = 0 with one task; just assert no panic.
        assert_eq!(session.tasks.selected, 0);
    }

    #[test]
    fn dispatch_quit_sets_should_quit() {
        let mut app = App::new(
            SqliteRepo::in_memory(),
            Theme::from_config_accent("cyan"),
            at(10),
        );
        let mut session = TuiSession::new(at(10));
        session.dispatch(&mut app, KeyAction::Quit);
        assert!(app.should_quit);
    }

    #[test]
    fn dispatch_edit_intent_from_agenda_opens_edit_overlay() {
        use crate::storage::Repo;
        let mut app = App::new(
            SqliteRepo::in_memory(),
            Theme::from_config_accent("cyan"),
            at(10),
        );
        let task = app.repo.create_task("alpha", None).unwrap();
        let e = app.repo.create_time_entry(task.id, at(9)).unwrap();
        app.repo.end_time_entry(e.id, at(10)).unwrap();
        let mut session = TuiSession::new(at(10));
        app.screen = ScreenKind::Agenda;
        session.refresh(&mut app).unwrap();
        session.dispatch(&mut app, KeyAction::Confirm);
        assert!(session.edit.is_some(), "edit overlay should be open");
    }
}
