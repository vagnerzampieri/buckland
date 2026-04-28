//! Agenda screen — time entries grouped by local day, week-paginated
//! with `h` / `l`. `enter` opens the Edit overlay; `D` deletes the
//! highlighted entry with an inline y/N confirm.

use crate::cli::format::duration_compact;
use crate::domain::TimeEntry;
use crate::storage::{Repo, SqliteRepo};
use crate::tui::app::App;
use crate::tui::keymap::KeyAction;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Utc};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

#[derive(Debug)]
pub struct AgendaState {
    /// The Monday (local) of the visible week.
    pub week_start: NaiveDate,
    pub entries: Vec<EntryRow>,
    pub selected: usize,
    pub prompt: Option<AgendaPrompt>,
}

#[derive(Debug, Clone)]
pub struct EntryRow {
    pub entry: TimeEntry,
    pub task_title: String,
    pub day_label: String, // "Mon 22 Apr"
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgendaPrompt {
    DeleteConfirm { entry_id: i64, label: String },
}

impl AgendaState {
    pub fn for_now(now: DateTime<Utc>) -> Self {
        Self {
            week_start: monday_of_local_week(now),
            entries: Vec::new(),
            selected: 0,
            prompt: None,
        }
    }

    pub fn refresh<R: Repo>(&mut self, repo: &R, now: DateTime<Utc>) -> anyhow::Result<()> {
        let from = local_midnight_utc(self.week_start);
        let to = local_midnight_utc(self.week_start + Duration::days(7));
        let entries = repo.list_entries_in_range(from, to, now)?;
        let mut rows = Vec::with_capacity(entries.len());
        for e in entries {
            let task = repo.find_task(e.task_id)?.ok_or_else(|| {
                anyhow::anyhow!("agenda: missing task #{} for entry #{}", e.task_id, e.id)
            })?;
            let day = e.started_at.with_timezone(&Local).date_naive();
            let day_label = day.format("%a %d %b").to_string();
            rows.push(EntryRow {
                entry: e,
                task_title: task.title,
                day_label,
            });
        }
        if self.selected >= rows.len() {
            self.selected = rows.len().saturating_sub(1);
        }
        self.entries = rows;
        Ok(())
    }

    pub fn prev_week(&mut self) {
        self.week_start -= Duration::days(7);
    }

    pub fn next_week(&mut self) {
        self.week_start += Duration::days(7);
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
}

pub fn render<R: Repo>(frame: &mut Frame<'_>, area: Rect, app: &App<R>, state: &AgendaState) {
    let title = format!("Agenda — week of {}", state.week_start.format("%Y-%m-%d"));
    let block = Block::default().borders(Borders::ALL).title(title);

    let mut last_day = String::new();
    let mut items: Vec<ListItem> = Vec::new();
    for (i, row) in state.entries.iter().enumerate() {
        if row.day_label != last_day {
            items.push(ListItem::new(Line::from(vec![Span::styled(
                row.day_label.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            )])));
            last_day = row.day_label.clone();
        }
        let entry_dur = row.entry.duration(app.now);
        let times = format!(
            "{}–{}",
            row.entry.started_at.with_timezone(&Local).format("%H:%M"),
            match row.entry.ended_at {
                Some(end) => end.with_timezone(&Local).format("%H:%M").to_string(),
                None => "now".into(),
            }
        );
        let line = Line::from(vec![
            Span::raw("  "),
            Span::raw(times),
            Span::raw("  "),
            Span::raw(crate::cli::commands_truncate(&row.task_title, 40)),
            Span::raw("  "),
            Span::styled(duration_compact(entry_dur), app.theme.total()),
        ]);
        let style = if i == state.selected {
            app.theme.selected()
        } else {
            Style::default()
        };
        items.push(ListItem::new(line).style(style));
    }
    if state.entries.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "  (no entries this week)",
            app.theme.dim(),
        ))));
    }
    let list = List::new(items).block(block);
    let mut list_state = ListState::default();
    // Translate entry index into list-item index (each day header adds one).
    list_state.select(Some(state.selected));
    frame.render_stateful_widget(list, area, &mut list_state);

    if let Some(AgendaPrompt::DeleteConfirm { entry_id, label }) = &state.prompt {
        let h = area.height.saturating_sub(1);
        let prompt_area = Rect::new(area.x, area.y + h, area.width, 1);
        let line = Line::from(Span::styled(
            format!("Delete entry #{entry_id} ({label})? y/N"),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(line), prompt_area);
    }
}

pub fn handle_key(
    state: &mut AgendaState,
    app: &mut App<SqliteRepo>,
    action: KeyAction,
) -> Option<AgendaIntent> {
    if state.prompt.is_some() {
        return handle_prompt_key(state, app, action);
    }
    match action {
        KeyAction::Down => {
            state.move_down();
            None
        }
        KeyAction::Up => {
            state.move_up();
            None
        }
        KeyAction::Left => {
            state.prev_week();
            let _ = state.refresh(&app.repo, app.now);
            None
        }
        KeyAction::Right => {
            state.next_week();
            let _ = state.refresh(&app.repo, app.now);
            None
        }
        KeyAction::Delete => {
            if let Some(row) = state.entries.get(state.selected) {
                let entry_dur = row.entry.duration(app.now);
                state.prompt = Some(AgendaPrompt::DeleteConfirm {
                    entry_id: row.entry.id,
                    label: duration_compact(entry_dur),
                });
            }
            None
        }
        KeyAction::Confirm | KeyAction::Edit => state
            .entries
            .get(state.selected)
            .map(|row| AgendaIntent::OpenEdit(row.entry.id)),
        _ => None,
    }
}

fn handle_prompt_key(
    state: &mut AgendaState,
    app: &mut App<SqliteRepo>,
    action: KeyAction,
) -> Option<AgendaIntent> {
    let Some(AgendaPrompt::DeleteConfirm { entry_id, .. }) = state.prompt.clone() else {
        return None;
    };
    match action {
        KeyAction::Char('y') => {
            match app.repo.delete_time_entry(entry_id) {
                Ok(()) => {
                    let _ = state.refresh(&app.repo, app.now);
                    app.refresh_active_timer().ok();
                    app.info(format!("Deleted entry #{entry_id}"));
                }
                Err(e) => app.error(format!("Could not delete: {e}")),
            }
            state.prompt = None;
            None
        }
        _ => {
            state.prompt = None;
            app.info("Cancelled");
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgendaIntent {
    OpenEdit(i64),
}

fn monday_of_local_week(now: DateTime<Utc>) -> NaiveDate {
    let local_today = now.with_timezone(&Local).date_naive();
    let days_since_monday = local_today.weekday().num_days_from_monday() as i64;
    local_today - Duration::days(days_since_monday)
}

fn local_midnight_utc(date: NaiveDate) -> DateTime<Utc> {
    let naive = date.and_hms_opt(0, 0, 0).expect("midnight always exists");
    Local
        .from_local_datetime(&naive)
        .single()
        .or_else(|| Local.from_local_datetime(&naive).earliest())
        .expect("local midnight resolvable (DST handled)")
        .with_timezone(&Utc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SqliteRepo;
    use crate::tui::app::App;
    use crate::tui::theme::Theme;

    fn at(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, 0, 0).unwrap()
    }

    fn fresh() -> (App<SqliteRepo>, AgendaState) {
        let now = at(2026, 4, 22, 10);
        let app = App::new(
            SqliteRepo::in_memory(),
            Theme::from_config_accent("cyan"),
            now,
        );
        let state = AgendaState::for_now(now);
        (app, state)
    }

    #[test]
    fn week_start_is_monday_of_now_local() {
        let (_, state) = fresh();
        // 2026-04-22 is a Wednesday → Monday is 2026-04-20.
        assert_eq!(
            state.week_start,
            NaiveDate::from_ymd_opt(2026, 4, 20).unwrap()
        );
    }

    #[test]
    fn refresh_loads_entries_for_visible_week_only() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        let task = app.repo.create_task("alpha", None).unwrap();
        // Entry inside the visible week (Wed):
        let inside = app
            .repo
            .create_time_entry(task.id, at(2026, 4, 22, 9))
            .unwrap();
        app.repo
            .end_time_entry(inside.id, at(2026, 4, 22, 10))
            .unwrap();
        // Entry outside (next week Mon):
        let outside = app
            .repo
            .create_time_entry(task.id, at(2026, 4, 27, 9))
            .unwrap();
        app.repo
            .end_time_entry(outside.id, at(2026, 4, 27, 10))
            .unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.entries[0].entry.id, inside.id);
    }

    #[test]
    fn left_paginates_to_previous_week_and_refreshes() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        let task = app.repo.create_task("t", None).unwrap();
        // Entry in the previous week (Wed 2026-04-15).
        let last_week = app
            .repo
            .create_time_entry(task.id, at(2026, 4, 15, 9))
            .unwrap();
        app.repo
            .end_time_entry(last_week.id, at(2026, 4, 15, 10))
            .unwrap();

        // Initial week has no entries.
        state.refresh(&app.repo, app.now).unwrap();
        assert!(state.entries.is_empty());

        // h navigates back; refresh now picks up the prior-week entry.
        handle_key(&mut state, &mut app, KeyAction::Left);
        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.entries[0].entry.id, last_week.id);
    }

    #[test]
    fn enter_returns_open_edit_intent_for_selected_entry() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        let task = app.repo.create_task("t", None).unwrap();
        let e = app
            .repo
            .create_time_entry(task.id, at(2026, 4, 22, 9))
            .unwrap();
        app.repo.end_time_entry(e.id, at(2026, 4, 22, 10)).unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        let intent = handle_key(&mut state, &mut app, KeyAction::Confirm);
        assert_eq!(intent, Some(AgendaIntent::OpenEdit(e.id)));
    }

    #[test]
    fn delete_confirm_y_deletes_entry() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        let task = app.repo.create_task("t", None).unwrap();
        let e = app
            .repo
            .create_time_entry(task.id, at(2026, 4, 22, 9))
            .unwrap();
        app.repo.end_time_entry(e.id, at(2026, 4, 22, 10)).unwrap();
        state.refresh(&app.repo, app.now).unwrap();

        handle_key(&mut state, &mut app, KeyAction::Delete);
        assert!(matches!(
            state.prompt,
            Some(AgendaPrompt::DeleteConfirm { .. })
        ));

        handle_key(&mut state, &mut app, KeyAction::Char('y'));
        assert!(state.entries.is_empty());
        assert!(state.prompt.is_none());
    }
}
