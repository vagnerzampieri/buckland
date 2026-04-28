//! Tasks screen — the home view. Lists open tasks with the running one
//! highlighted. Owns: selection cursor, optional filter input, optional
//! new-task prompt, optional delete-confirm prompt.

use crate::cli::format::duration_compact;
use crate::domain::Task;
use crate::storage::Repo;
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
        Prompt::DeleteConfirm { task_id, title } => Line::from(vec![Span::styled(
            format!("Delete task #{task_id} \"{title}\"? y/N"),
            Style::default().add_modifier(Modifier::BOLD),
        )]),
    };
    frame.render_widget(Paragraph::new(line), prompt_area);
}

pub fn handle_key<R: Repo>(state: &mut TasksState, _app: &mut App<R>, action: KeyAction) -> bool {
    if state.prompt.is_some() {
        // Prompt-mode handling is added in Tasks 9 & 10 below.
        return false;
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
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SqliteRepo;
    use crate::tui::app::App;
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
}
