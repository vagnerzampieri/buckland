//! The top-level `App<R: Repo>` state machine. Owns the active screen,
//! a header snapshot of the active timer, and the footer message line.
//! `step()` is the pure transition function used by tests; the real
//! event loop wraps it with crossterm + a 1Hz tick thread.

use crate::storage::Repo;
use crate::tui::keymap::KeyAction;
use crate::tui::theme::Theme;
use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenKind {
    Tasks,
    Agenda,
    Report,
    Help,
}

#[derive(Debug, Clone)]
pub struct ActiveTimer {
    pub task_id: i64,
    pub task_title: String,
    pub shortcut_external_id: Option<i64>,
    pub started_at: DateTime<Utc>,
}

impl ActiveTimer {
    pub fn elapsed(&self, now: DateTime<Utc>) -> Duration {
        (now - self.started_at).max(Duration::zero())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FooterMessage {
    Empty,
    Info(String),
    Error(String),
}

pub struct App<R: Repo> {
    pub repo: R,
    pub theme: Theme,
    pub screen: ScreenKind,
    pub active_timer: Option<ActiveTimer>,
    pub footer: FooterMessage,
    pub now: DateTime<Utc>,
    pub pending_g: bool,
    pub should_quit: bool,
}

impl<R: Repo> App<R> {
    pub fn new(repo: R, theme: Theme, now: DateTime<Utc>) -> Self {
        Self {
            repo,
            theme,
            screen: ScreenKind::Tasks,
            active_timer: None,
            footer: FooterMessage::Empty,
            now,
            pending_g: false,
            should_quit: false,
        }
    }

    /// Refresh the active-timer snapshot from the repo. Called once at
    /// startup and after every mutation that may have changed the active
    /// row.
    pub fn refresh_active_timer(&mut self) -> anyhow::Result<()> {
        let entry = self.repo.active_time_entry()?;
        self.active_timer = match entry {
            Some(e) => {
                let task = self.repo.find_task(e.task_id)?.ok_or_else(|| {
                    anyhow::anyhow!(
                        "internal: active entry #{} references missing task #{}",
                        e.id,
                        e.task_id
                    )
                })?;
                let sc_external = match task.shortcut_story_id {
                    Some(row_id) => self
                        .repo
                        .find_shortcut_story_by_row_id(row_id)?
                        .map(|s| s.external_id),
                    None => None,
                };
                Some(ActiveTimer {
                    task_id: task.id,
                    task_title: task.title,
                    shortcut_external_id: sc_external,
                    started_at: e.started_at,
                })
            }
            None => None,
        };
        Ok(())
    }

    /// Pure transition for a tick. Records `now` so the header clock
    /// advances; no DB calls.
    pub fn on_tick(&mut self, now: DateTime<Utc>) {
        self.now = now;
    }

    /// Pure transition for a `Quit` request from any screen. Sets
    /// `should_quit`; the loop checks this each iteration.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Posts a transient info message to the footer, replacing whatever
    /// was there.
    pub fn info(&mut self, msg: impl Into<String>) {
        self.footer = FooterMessage::Info(msg.into());
    }

    pub fn error(&mut self, msg: impl Into<String>) {
        self.footer = FooterMessage::Error(msg.into());
    }

    pub fn clear_footer(&mut self) {
        self.footer = FooterMessage::Empty;
    }

    /// Resolves the global handlers shared by every screen. Returns
    /// `true` when the action was consumed at the global level so screens
    /// can skip it.
    pub fn handle_global(&mut self, action: KeyAction) -> bool {
        match action {
            KeyAction::Quit => {
                if self.screen == ScreenKind::Help {
                    self.screen = ScreenKind::Tasks;
                } else {
                    self.quit();
                }
                true
            }
            KeyAction::Help => {
                self.screen = ScreenKind::Help;
                true
            }
            KeyAction::OpenTasks if !self.pending_g => {
                self.pending_g = true;
                self.clear_footer();
                false
            }
            KeyAction::OpenAgenda => {
                self.screen = ScreenKind::Agenda;
                self.pending_g = false;
                self.clear_footer();
                true
            }
            KeyAction::OpenReport => {
                self.screen = ScreenKind::Report;
                self.pending_g = false;
                self.clear_footer();
                true
            }
            _ => {
                if self.pending_g {
                    self.pending_g = false;
                    self.clear_footer();
                    if matches!(action, KeyAction::OpenTasks) {
                        self.screen = ScreenKind::Tasks;
                        return true;
                    }
                }
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SqliteRepo;
    use chrono::TimeZone;

    fn at(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 22, h, 0, 0).unwrap()
    }

    fn app() -> App<SqliteRepo> {
        let repo = SqliteRepo::in_memory();
        let theme = Theme::from_config_accent("cyan");
        App::new(repo, theme, at(10))
    }

    #[test]
    fn starts_on_tasks_screen() {
        let a = app();
        assert_eq!(a.screen, ScreenKind::Tasks);
        assert!(!a.should_quit);
        assert!(a.active_timer.is_none());
    }

    #[test]
    fn quit_action_globally_sets_quit() {
        let mut a = app();
        a.handle_global(KeyAction::Quit);
        assert!(a.should_quit);
    }

    #[test]
    fn quit_inside_help_returns_to_tasks_instead_of_quitting() {
        let mut a = app();
        a.screen = ScreenKind::Help;
        a.handle_global(KeyAction::Quit);
        assert!(!a.should_quit);
        assert_eq!(a.screen, ScreenKind::Tasks);
    }

    #[test]
    fn help_action_opens_help_screen() {
        let mut a = app();
        a.handle_global(KeyAction::Help);
        assert_eq!(a.screen, ScreenKind::Help);
    }

    #[test]
    fn agenda_action_switches_screen() {
        let mut a = app();
        a.handle_global(KeyAction::OpenAgenda);
        assert_eq!(a.screen, ScreenKind::Agenda);
    }

    #[test]
    fn pending_g_then_g_returns_to_tasks() {
        let mut a = app();
        a.screen = ScreenKind::Agenda;
        // First g — sets pending and is NOT consumed (the screen will see it
        // and ignore it; pending_g will be reset on the next key).
        a.handle_global(KeyAction::OpenTasks);
        assert!(a.pending_g);
        // Second g — recognized as `gg` and routes to Tasks.
        let consumed = a.handle_global(KeyAction::OpenTasks);
        assert!(consumed);
        assert_eq!(a.screen, ScreenKind::Tasks);
        assert!(!a.pending_g);
    }

    #[test]
    fn tick_updates_now() {
        let mut a = app();
        let later = at(11);
        a.on_tick(later);
        assert_eq!(a.now, later);
    }

    #[test]
    fn refresh_active_timer_picks_up_running_entry() {
        use crate::domain::TimerOps;
        use crate::storage::Repo;
        let mut a = app();
        let task = a.repo.create_task("active", None).unwrap();
        TimerOps::new(&mut a.repo).start(task.id, at(10)).unwrap();
        a.refresh_active_timer().unwrap();
        let snapshot = a.active_timer.as_ref().unwrap();
        assert_eq!(snapshot.task_id, task.id);
        assert_eq!(snapshot.task_title, "active");
    }

    #[test]
    fn footer_messaging_helpers_set_state() {
        let mut a = app();
        a.info("ok");
        assert_eq!(a.footer, FooterMessage::Info("ok".into()));
        a.error("nope");
        assert_eq!(a.footer, FooterMessage::Error("nope".into()));
        a.clear_footer();
        assert_eq!(a.footer, FooterMessage::Empty);
    }
}
