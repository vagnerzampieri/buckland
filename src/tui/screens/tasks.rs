//! Tasks screen — the home view. Lists open tasks with the running one
//! highlighted. Owns: selection cursor, optional filter input, optional
//! new-task prompt, optional delete-confirm prompt.

use crate::cli::format::duration_compact;
use crate::domain::Task;
use crate::storage::{Repo, SqliteRepo};
use crate::tui::app::App;
use crate::tui::keymap::KeyAction;
use chrono::{DateTime, Utc};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

#[derive(Debug, Default)]
pub struct TasksState {
    pub tasks: Vec<Task>,
    pub totals: Vec<i64>, // seconds, parallel array to `tasks`
    pub selected: usize,
    pub filter: Option<String>,
    pub prompt: Option<Prompt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prompt {
    NewTask { buffer: String },
    DeleteConfirm { task_id: i64, title: String },
    Filter { buffer: String },
}

impl TasksState {
    pub fn refresh<R: Repo>(&mut self, repo: &R, now: DateTime<Utc>) -> anyhow::Result<()> {
        let mut tasks = repo.list_open_tasks()?;
        // Filter is applied last so the source list is stable for selection math.
        if let Some(needle) = self.filter.as_deref() {
            let needle = needle.to_lowercase();
            tasks.retain(|t| t.title.to_lowercase().contains(&needle));
        }
        let mut totals = Vec::with_capacity(tasks.len());
        for t in &tasks {
            let d = repo.task_total_duration(t.id, now)?;
            totals.push(d.num_seconds());
        }
        if self.selected >= tasks.len() {
            self.selected = tasks.len().saturating_sub(1);
        }
        self.tasks = tasks;
        self.totals = totals;
        Ok(())
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.tasks.len() {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_top(&mut self) {
        self.selected = 0;
    }

    pub fn move_bottom(&mut self) {
        self.selected = self.tasks.len().saturating_sub(1);
    }

    pub fn selected_task(&self) -> Option<&Task> {
        self.tasks.get(self.selected)
    }
}

pub fn render<R: Repo>(frame: &mut Frame<'_>, area: Rect, app: &App<R>, state: &TasksState) {
    let block = Block::default().borders(Borders::ALL).title("Tasks");
    let active_id = app.active_timer.as_ref().map(|t| t.task_id);
    let items: Vec<ListItem> = state
        .tasks
        .iter()
        .zip(state.totals.iter())
        .enumerate()
        .map(|(i, (t, secs))| {
            let is_active = Some(t.id) == active_id;
            let marker = if is_active { "▶ " } else { "  " };
            let total = duration_compact(chrono::Duration::seconds(*secs));
            let line = Line::from(vec![
                Span::styled(
                    marker,
                    if is_active {
                        app.theme.running()
                    } else {
                        Style::default()
                    },
                ),
                Span::raw(format!("#{:<4} ", t.id)),
                Span::raw(format!(
                    "{:<40} ",
                    crate::cli::commands_truncate(&t.title, 40)
                )),
                Span::styled(total, app.theme.total()),
            ]);
            let style = if i == state.selected {
                app.theme.selected()
            } else if is_active {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(line).style(style)
        })
        .collect();
    let list = List::new(items).block(block);
    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));
    frame.render_stateful_widget(list, area, &mut list_state);

    if let Some(prompt) = &state.prompt {
        render_prompt(frame, area, prompt);
    }
}

fn render_prompt(frame: &mut Frame<'_>, area: Rect, prompt: &Prompt) {
    let h = area.height.saturating_sub(1);
    let prompt_area = Rect::new(area.x, area.y + h, area.width, 1);
    let line = match prompt {
        Prompt::NewTask { buffer } => Line::from(vec![
            Span::styled("New task: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(buffer.clone()),
            Span::raw("_"),
        ]),
        Prompt::Filter { buffer } => Line::from(vec![
            Span::styled("Filter: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(buffer.clone()),
            Span::raw("_"),
        ]),
        Prompt::DeleteConfirm { task_id, title } => Line::from(vec![Span::styled(
            format!("Delete task #{task_id} \"{title}\"? y/N"),
            Style::default().add_modifier(Modifier::BOLD),
        )]),
    };
    frame.render_widget(Paragraph::new(line), prompt_area);
}

pub fn handle_key(state: &mut TasksState, app: &mut App<SqliteRepo>, action: KeyAction) -> bool {
    if state.prompt.is_some() {
        return handle_prompt_key(state, app, action);
    }
    match action {
        KeyAction::Down => {
            state.move_down();
            true
        }
        KeyAction::Up => {
            state.move_up();
            true
        }
        KeyAction::Top => {
            state.move_top();
            true
        }
        KeyAction::Bottom => {
            state.move_bottom();
            true
        }
        KeyAction::StartSelected | KeyAction::Confirm => {
            start_selected(state, app);
            true
        }
        KeyAction::StopActive => {
            stop_active(state, app);
            true
        }
        KeyAction::Done => {
            mark_selected_done(state, app);
            true
        }
        KeyAction::Archive => {
            archive_selected(state, app);
            true
        }
        KeyAction::Delete => {
            open_delete_confirm(state, app);
            true
        }
        KeyAction::NewTask => {
            state.prompt = Some(Prompt::NewTask {
                buffer: String::new(),
            });
            true
        }
        KeyAction::Filter => {
            state.prompt = Some(Prompt::Filter {
                buffer: String::new(),
            });
            true
        }
        _ => false,
    }
}

fn handle_prompt_key(state: &mut TasksState, app: &mut App<SqliteRepo>, action: KeyAction) -> bool {
    let Some(prompt) = state.prompt.clone() else {
        return false;
    };
    match (prompt, action) {
        (Prompt::DeleteConfirm { task_id, title }, KeyAction::Char('y')) => {
            confirm_delete(state, app, task_id, &title);
            state.prompt = None;
            true
        }
        (Prompt::DeleteConfirm { .. }, _) => {
            state.prompt = None;
            app.info("Cancelled");
            true
        }
        (Prompt::NewTask { mut buffer }, KeyAction::Char(c)) => {
            buffer.push(c);
            state.prompt = Some(Prompt::NewTask { buffer });
            true
        }
        (Prompt::NewTask { mut buffer }, KeyAction::Backspace) => {
            buffer.pop();
            state.prompt = Some(Prompt::NewTask { buffer });
            true
        }
        (Prompt::NewTask { buffer }, KeyAction::Confirm) => {
            create_task_from_prompt(state, app, &buffer);
            state.prompt = None;
            true
        }
        (Prompt::NewTask { .. }, KeyAction::Quit) => {
            state.prompt = None;
            app.info("Cancelled");
            true
        }
        (Prompt::Filter { mut buffer }, KeyAction::Char(c)) => {
            buffer.push(c);
            state.prompt = Some(Prompt::Filter { buffer });
            true
        }
        (Prompt::Filter { mut buffer }, KeyAction::Backspace) => {
            buffer.pop();
            state.prompt = Some(Prompt::Filter { buffer });
            true
        }
        (Prompt::Filter { buffer }, KeyAction::Confirm) => {
            apply_filter(state, app, buffer);
            state.prompt = None;
            true
        }
        (Prompt::Filter { .. }, KeyAction::Quit) => {
            state.filter = None;
            state.prompt = None;
            let _ = state.refresh(&app.repo, app.now);
            true
        }
        _ => true, // swallow anything else while a prompt is open
    }
}

fn create_task_from_prompt(state: &mut TasksState, app: &mut App<SqliteRepo>, title: &str) {
    let title = title.trim();
    if title.is_empty() {
        app.info("Empty title — nothing created.");
        return;
    }
    match app.repo.create_task(title, None) {
        Ok(t) => {
            let _ = state.refresh(&app.repo, app.now);
            app.info(format!("Added #{} {}", t.id, t.title));
        }
        Err(e) => app.error(format!("Could not add: {e}")),
    }
}

fn apply_filter(state: &mut TasksState, app: &mut App<SqliteRepo>, buffer: String) {
    let trimmed = buffer.trim();
    state.filter = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    };
    let _ = state.refresh(&app.repo, app.now);
}

fn mark_selected_done(state: &mut TasksState, app: &mut App<SqliteRepo>) {
    let Some(task) = state.selected_task().cloned() else {
        app.info("Nothing selected.");
        return;
    };
    match app.repo.mark_task_done(task.id, app.now) {
        Ok(_) => {
            let _ = state.refresh(&app.repo, app.now);
            app.info(format!("Done #{} {}", task.id, task.title));
        }
        Err(e) => app.error(format!("Could not mark done: {e}")),
    }
}

fn archive_selected(state: &mut TasksState, app: &mut App<SqliteRepo>) {
    let Some(task) = state.selected_task().cloned() else {
        app.info("Nothing selected.");
        return;
    };
    match app.repo.archive_task(task.id, app.now) {
        Ok(_) => {
            let _ = state.refresh(&app.repo, app.now);
            app.info(format!("Archived #{} {}", task.id, task.title));
        }
        Err(e) => app.error(format!("Could not archive: {e}")),
    }
}

fn open_delete_confirm(state: &mut TasksState, app: &mut App<SqliteRepo>) {
    let Some(task) = state.selected_task() else {
        app.info("Nothing selected.");
        return;
    };
    state.prompt = Some(Prompt::DeleteConfirm {
        task_id: task.id,
        title: task.title.clone(),
    });
}

fn confirm_delete(state: &mut TasksState, app: &mut App<SqliteRepo>, task_id: i64, title: &str) {
    use crate::storage::RepoError;
    match app.repo.delete_task(task_id) {
        Ok(()) => {
            let _ = state.refresh(&app.repo, app.now);
            app.info(format!("Deleted #{task_id} {title}"));
        }
        Err(RepoError::TaskHasEntries(_)) => {
            app.error(format!(
                "Task #{task_id} has time entries — use Archive (A) instead."
            ));
        }
        Err(e) => app.error(format!("Could not delete: {e}")),
    }
}

fn start_selected(state: &mut TasksState, app: &mut App<SqliteRepo>) {
    use crate::domain::TimerOps;
    let Some(task) = state.selected_task().cloned() else {
        app.info("Nothing to start. Press n to add a task.");
        return;
    };
    let now = app.now;
    match TimerOps::new(&mut app.repo).start(task.id, now) {
        Ok(_) => {
            app.refresh_active_timer().ok();
            let _ = state.refresh(&app.repo, now);
            app.info(format!("Started #{} {}", task.id, task.title));
        }
        Err(e) => {
            app.error(format!("Could not start: {e}"));
        }
    }
}

fn stop_active(state: &mut TasksState, app: &mut App<SqliteRepo>) {
    use crate::domain::TimerOps;
    let now = app.now;
    match TimerOps::new(&mut app.repo).stop(now) {
        Ok(Some(_)) => {
            app.refresh_active_timer().ok();
            let _ = state.refresh(&app.repo, now);
            app.info("Stopped");
        }
        Ok(None) => app.info("Nothing to stop."),
        Err(e) => app.error(format!("Could not stop: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SqliteRepo;
    use crate::tui::app::{App, FooterMessage};
    use crate::tui::keymap::KeyAction;
    use crate::tui::theme::Theme;
    use chrono::TimeZone;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap()
    }

    fn fresh() -> (App<SqliteRepo>, TasksState) {
        let app = App::new(
            SqliteRepo::in_memory(),
            Theme::from_config_accent("cyan"),
            now(),
        );
        (app, TasksState::default())
    }

    #[test]
    fn refresh_loads_open_tasks_in_creation_order_desc() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        app.repo.create_task("first", None).unwrap();
        app.repo.create_task("second", None).unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        assert_eq!(state.tasks.len(), 2);
        // list_open_tasks sorts by created_at DESC — newest first.
        assert_eq!(state.tasks[0].title, "second");
        assert_eq!(state.tasks[1].title, "first");
    }

    #[test]
    fn refresh_filter_narrows_list_case_insensitive() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        app.repo.create_task("Login flow", None).unwrap();
        app.repo.create_task("Pricing tweak", None).unwrap();
        state.filter = Some("login".into());
        state.refresh(&app.repo, app.now).unwrap();
        assert_eq!(state.tasks.len(), 1);
        assert_eq!(state.tasks[0].title, "Login flow");
    }

    #[test]
    fn move_down_clamps_at_last_row() {
        let (_, mut state) = fresh();
        state.tasks = vec![dummy_task(1), dummy_task(2)];
        state.totals = vec![0, 0];
        state.move_down();
        assert_eq!(state.selected, 1);
        state.move_down();
        assert_eq!(state.selected, 1, "must clamp at len-1");
    }

    #[test]
    fn move_up_clamps_at_zero() {
        let (_, mut state) = fresh();
        state.tasks = vec![dummy_task(1), dummy_task(2)];
        state.totals = vec![0, 0];
        state.move_up();
        assert_eq!(state.selected, 0, "must clamp at 0");
    }

    #[test]
    fn move_top_and_bottom_jump() {
        let (_, mut state) = fresh();
        state.tasks = (1..=5).map(dummy_task).collect();
        state.totals = vec![0; 5];
        state.move_bottom();
        assert_eq!(state.selected, 4);
        state.move_top();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn start_action_starts_timer_for_selected_task() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        let t = app.repo.create_task("alpha", None).unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        let acted = handle_key(&mut state, &mut app, KeyAction::StartSelected);
        assert!(acted);
        let active = app.repo.active_time_entry().unwrap().expect("active set");
        assert_eq!(active.task_id, t.id);
    }

    #[test]
    fn start_action_with_empty_list_is_a_noop_and_warns() {
        let (mut app, mut state) = fresh();
        let acted = handle_key(&mut state, &mut app, KeyAction::StartSelected);
        assert!(acted);
        assert!(matches!(app.footer, FooterMessage::Info(_)));
    }

    #[test]
    fn stop_action_ends_active_timer_and_refreshes_header() {
        use crate::domain::TimerOps;
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        let t = app.repo.create_task("running", None).unwrap();
        TimerOps::new(&mut app.repo).start(t.id, app.now).unwrap();
        app.refresh_active_timer().unwrap();
        state.refresh(&app.repo, app.now).unwrap();

        let acted = handle_key(&mut state, &mut app, KeyAction::StopActive);
        assert!(acted);
        assert!(app.repo.active_time_entry().unwrap().is_none());
        assert!(app.active_timer.is_none());
    }

    #[test]
    fn confirm_action_starts_timer_when_no_prompt_open() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        let t = app.repo.create_task("alpha", None).unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        handle_key(&mut state, &mut app, KeyAction::Confirm);
        let active = app.repo.active_time_entry().unwrap().expect("active set");
        assert_eq!(active.task_id, t.id);
    }

    #[test]
    fn done_action_marks_selected_task_complete_and_drops_from_open_list() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        app.repo.create_task("finish me", None).unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        handle_key(&mut state, &mut app, KeyAction::Done);
        assert!(
            state.tasks.is_empty(),
            "open list should be empty after done"
        );
        assert!(matches!(app.footer, FooterMessage::Info(_)));
    }

    #[test]
    fn archive_action_drops_from_open_list() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        app.repo.create_task("shelf", None).unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        handle_key(&mut state, &mut app, KeyAction::Archive);
        assert!(state.tasks.is_empty());
    }

    #[test]
    fn delete_action_opens_confirm_prompt_then_y_deletes() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        app.repo.create_task("oops", None).unwrap();
        state.refresh(&app.repo, app.now).unwrap();

        // First D opens prompt.
        handle_key(&mut state, &mut app, KeyAction::Delete);
        assert!(matches!(state.prompt, Some(Prompt::DeleteConfirm { .. })));

        // Pressing y deletes the task and clears the prompt.
        handle_key(&mut state, &mut app, KeyAction::Char('y'));
        assert!(state.prompt.is_none());
        assert!(state.tasks.is_empty());
    }

    #[test]
    fn delete_confirm_n_cancels() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        app.repo.create_task("oops", None).unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        handle_key(&mut state, &mut app, KeyAction::Delete);
        handle_key(&mut state, &mut app, KeyAction::Char('n'));
        assert!(state.prompt.is_none());
        assert_eq!(state.tasks.len(), 1, "task still present after cancel");
    }

    #[test]
    fn delete_blocked_when_task_has_entries() {
        use crate::domain::TimerOps;
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        let t = app.repo.create_task("with entries", None).unwrap();
        TimerOps::new(&mut app.repo).start(t.id, app.now).unwrap();
        TimerOps::new(&mut app.repo)
            .stop(app.now + chrono::Duration::seconds(5))
            .unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        handle_key(&mut state, &mut app, KeyAction::Delete);
        handle_key(&mut state, &mut app, KeyAction::Char('y'));
        assert_eq!(state.tasks.len(), 1, "delete must be blocked");
        assert!(matches!(app.footer, FooterMessage::Error(_)));
    }

    fn dummy_task(id: i64) -> Task {
        Task {
            id,
            title: format!("t{id}"),
            description: None,
            shortcut_story_id: None,
            completed_at: None,
            archived_at: None,
            created_at: now(),
            updated_at: now(),
        }
    }

    #[test]
    fn new_task_prompt_consumes_chars_then_enter_creates_task() {
        let (mut app, mut state) = fresh();
        state.refresh(&app.repo, app.now).unwrap();

        handle_key(&mut state, &mut app, KeyAction::NewTask);
        assert!(matches!(state.prompt, Some(Prompt::NewTask { .. })));

        for c in "fix login".chars() {
            handle_key(&mut state, &mut app, KeyAction::Char(c));
        }
        if let Some(Prompt::NewTask { buffer }) = &state.prompt {
            assert_eq!(buffer, "fix login");
        } else {
            panic!("expected NewTask prompt");
        }

        handle_key(&mut state, &mut app, KeyAction::Confirm);
        assert!(state.prompt.is_none(), "prompt must close on enter");
        assert_eq!(state.tasks.len(), 1);
        assert_eq!(state.tasks[0].title, "fix login");
    }

    #[test]
    fn new_task_prompt_esc_cancels() {
        let (mut app, mut state) = fresh();
        handle_key(&mut state, &mut app, KeyAction::NewTask);
        handle_key(&mut state, &mut app, KeyAction::Char('x'));
        handle_key(&mut state, &mut app, KeyAction::Quit); // esc / q maps to Quit
        assert!(state.prompt.is_none());
        assert!(state.tasks.is_empty());
    }

    #[test]
    fn new_task_prompt_backspace_removes_last_char() {
        let (mut app, mut state) = fresh();
        handle_key(&mut state, &mut app, KeyAction::NewTask);
        handle_key(&mut state, &mut app, KeyAction::Char('a'));
        handle_key(&mut state, &mut app, KeyAction::Char('b'));
        handle_key(&mut state, &mut app, KeyAction::Backspace);
        if let Some(Prompt::NewTask { buffer }) = &state.prompt {
            assert_eq!(buffer, "a");
        } else {
            panic!("expected NewTask prompt");
        }
    }

    #[test]
    fn filter_prompt_narrows_list_live() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        app.repo.create_task("Login flow", None).unwrap();
        app.repo.create_task("Pricing", None).unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        assert_eq!(state.tasks.len(), 2);

        handle_key(&mut state, &mut app, KeyAction::Filter);
        for c in "log".chars() {
            handle_key(&mut state, &mut app, KeyAction::Char(c));
        }
        // After confirm, prompt closes, filter persists, list narrows.
        handle_key(&mut state, &mut app, KeyAction::Confirm);
        assert_eq!(state.tasks.len(), 1);
        assert_eq!(state.tasks[0].title, "Login flow");
        assert_eq!(state.filter.as_deref(), Some("log"));
    }

    #[test]
    fn filter_prompt_esc_clears_filter() {
        let (mut app, mut state) = fresh();
        state.filter = Some("old".into());
        handle_key(&mut state, &mut app, KeyAction::Filter);
        handle_key(&mut state, &mut app, KeyAction::Quit);
        assert!(state.filter.is_none(), "esc clears any prior filter");
    }
}
