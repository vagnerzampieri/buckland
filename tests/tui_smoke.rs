//! End-to-end smoke tests for the Phase D TUI.
//!
//! These tests drive `TuiSession::dispatch` through realistic key sequences
//! against an in-memory `SqliteRepo`. They never enter raw mode or touch a
//! real terminal — `crossterm` would refuse without a TTY in CI — so they
//! exercise the *logic* of the TUI (state transitions, screen routing,
//! prompt I/O, edit overlay lifecycle) rather than its rendering. The
//! per-screen `tests/` modules already cover rendering with `TestBackend`.
//!
//! The manual TTY smoke test (open `bl tui`, navigate, quit) lives in the
//! README under "Keyboard shortcuts" — that one needs human eyes.

use buckland::storage::{self, Repo, SqliteRepo};
use buckland::tui::app::{App, ScreenKind};
use buckland::tui::keymap::KeyAction;
use buckland::tui::theme::Theme;
use buckland::tui::TuiSession;
use chrono::{DateTime, TimeZone, Utc};

fn at(h: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 4, 28, h, 0, 0).unwrap()
}

fn boot() -> (App<SqliteRepo>, TuiSession) {
    let now = at(10);
    let conn = storage::open_memory().expect("open in-memory DB");
    let app = App::new(
        SqliteRepo::new(conn),
        Theme::from_config_accent("cyan"),
        now,
    );
    let session = TuiSession::new(now);
    (app, session)
}

#[test]
fn full_user_flow_tasks_agenda_report_help_quit() {
    let (mut app, mut session) = boot();
    let t1 = app.repo.create_task("alpha", None).unwrap();
    app.repo.create_task("beta", None).unwrap();
    let entry = app.repo.create_time_entry(t1.id, at(9)).unwrap();
    app.repo
        .end_time_entry(entry.id, at(9) + chrono::Duration::minutes(30))
        .unwrap();
    session.refresh(&mut app).unwrap();

    assert_eq!(app.screen, ScreenKind::Tasks);
    assert_eq!(session.tasks.tasks.len(), 2);

    session.dispatch(&mut app, KeyAction::OpenAgenda);
    assert_eq!(app.screen, ScreenKind::Agenda);
    assert!(
        !session.agenda.entries.is_empty(),
        "agenda should surface the seeded entry"
    );

    session.dispatch(&mut app, KeyAction::Confirm);
    assert!(
        session.edit.is_some(),
        "Enter on an agenda row should open the Edit overlay"
    );

    session.dispatch(&mut app, KeyAction::Quit);
    assert!(
        session.edit.is_none(),
        "Esc inside the Edit overlay should close it"
    );
    assert!(
        !app.should_quit,
        "closing the overlay must not quit the app"
    );

    session.dispatch(&mut app, KeyAction::OpenReport);
    assert_eq!(app.screen, ScreenKind::Report);
    session.dispatch(&mut app, KeyAction::CycleNext);
    session.dispatch(&mut app, KeyAction::CycleGroup);
    session.dispatch(&mut app, KeyAction::Down);
    assert!(session.report.json, "Down on Report should toggle JSON");

    session.dispatch(&mut app, KeyAction::Help);
    assert_eq!(app.screen, ScreenKind::Help);

    session.dispatch(&mut app, KeyAction::OpenAgenda);
    assert_eq!(
        app.screen,
        ScreenKind::Agenda,
        "any view-switch from Help should leave the overlay"
    );

    session.dispatch(&mut app, KeyAction::Quit);
    assert!(app.should_quit);
}

#[test]
fn new_task_prompt_creates_task_via_dispatch() {
    let (mut app, mut session) = boot();
    session.refresh(&mut app).unwrap();

    session.dispatch(&mut app, KeyAction::NewTask);
    for c in "fix login".chars() {
        session.dispatch(&mut app, KeyAction::Char(c));
    }
    session.dispatch(&mut app, KeyAction::Confirm);

    assert_eq!(session.tasks.tasks.len(), 1);
    assert_eq!(session.tasks.tasks[0].title, "fix login");
}

#[test]
fn start_then_stop_via_dispatch_drives_active_timer_snapshot() {
    let (mut app, mut session) = boot();
    app.repo.create_task("alpha", None).unwrap();
    session.refresh(&mut app).unwrap();

    session.dispatch(&mut app, KeyAction::StartSelected);
    let snap = app
        .active_timer
        .as_ref()
        .expect("StartSelected should set an active timer in the App snapshot");
    assert_eq!(snap.task_title, "alpha");

    session.dispatch(&mut app, KeyAction::StopActive);
    assert!(
        app.active_timer.is_none(),
        "StopActive should clear the active timer snapshot"
    );
}

#[test]
fn delete_confirm_blocks_when_task_has_entries() {
    let (mut app, mut session) = boot();
    let t = app.repo.create_task("with entries", None).unwrap();
    let e = app.repo.create_time_entry(t.id, at(9)).unwrap();
    app.repo
        .end_time_entry(e.id, at(9) + chrono::Duration::minutes(5))
        .unwrap();
    session.refresh(&mut app).unwrap();

    session.dispatch(&mut app, KeyAction::Delete);
    session.dispatch(&mut app, KeyAction::Char('y'));

    assert_eq!(
        session.tasks.tasks.len(),
        1,
        "delete must be blocked for tasks with entries"
    );
    assert!(matches!(
        app.footer,
        buckland::tui::app::FooterMessage::Error(_)
    ));
}
