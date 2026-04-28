//! Edit overlay — modal for amending a single `time_entry`. Fields:
//! task title (read-only), started_at, ended_at, notes. `Tab` cycles
//! fields. `Enter` saves. `Esc` cancels. `^D` deletes (with `y/N`).

use crate::domain::TimeEntry;
use crate::storage::Repo;
use crate::tui::app::App;
use crate::tui::keymap::KeyAction;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditField {
    StartedAt,
    EndedAt,
    Notes,
}

#[derive(Debug, Clone)]
pub struct EditState {
    pub entry_id: i64,
    pub task_title: String,
    pub started_at: String, // "YYYY-MM-DD HH:MM"
    pub ended_at: String,   // empty when active
    pub notes: String,
    pub field: EditField,
    pub confirm_delete: bool,
}

impl EditState {
    pub fn for_entry(entry: &TimeEntry, task_title: String) -> Self {
        Self {
            entry_id: entry.id,
            task_title,
            started_at: format_local(entry.started_at),
            ended_at: entry.ended_at.map(format_local).unwrap_or_default(),
            notes: entry.notes.clone().unwrap_or_default(),
            field: EditField::StartedAt,
            confirm_delete: false,
        }
    }

    pub fn cycle_field(&mut self) {
        self.field = match self.field {
            EditField::StartedAt => EditField::EndedAt,
            EditField::EndedAt => EditField::Notes,
            EditField::Notes => EditField::StartedAt,
        };
    }

    pub fn current_buffer_mut(&mut self) -> &mut String {
        match self.field {
            EditField::StartedAt => &mut self.started_at,
            EditField::EndedAt => &mut self.ended_at,
            EditField::Notes => &mut self.notes,
        }
    }
}

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &EditState) {
    let backdrop_area = area;
    let modal_area = centered(60, 12, backdrop_area);
    let dim = Block::default().style(Style::default().add_modifier(Modifier::DIM));
    frame.render_widget(dim, backdrop_area);

    let block = Block::default().borders(Borders::ALL).title("Edit entry");
    frame.render_widget(block.clone(), modal_area);
    let inner = block.inner(modal_area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(format!("Task: {}", state.task_title)),
        rows[0],
    );
    frame.render_widget(
        field_line(
            "Started",
            &state.started_at,
            state.field == EditField::StartedAt,
        ),
        rows[2],
    );
    frame.render_widget(
        field_line(
            "Ended  ",
            &state.ended_at,
            state.field == EditField::EndedAt,
        ),
        rows[3],
    );
    frame.render_widget(
        field_line("Notes  ", &state.notes, state.field == EditField::Notes),
        rows[4],
    );
    let footer = if state.confirm_delete {
        "Delete this entry? y/N"
    } else {
        "Tab cycle • Enter save • ^D delete • Esc cancel"
    };
    frame.render_widget(
        Paragraph::new(footer).style(Style::default().add_modifier(Modifier::DIM)),
        rows[5],
    );
}

fn field_line(label: &'static str, value: &str, focused: bool) -> Paragraph<'static> {
    let style = if focused {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };
    Paragraph::new(Line::from(vec![
        Span::raw(format!("{label}: ")),
        Span::styled(value.to_string(), style),
    ]))
}

fn centered(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditOutcome {
    Continue,
    Saved,
    Deleted,
    Cancelled,
}

pub fn handle_key<R: Repo>(
    state: &mut EditState,
    app: &mut App<R>,
    action: KeyAction,
) -> EditOutcome {
    if state.confirm_delete {
        return match action {
            KeyAction::Char('y') => match app.repo.delete_time_entry(state.entry_id) {
                Ok(()) => {
                    app.info(format!("Deleted entry #{}", state.entry_id));
                    EditOutcome::Deleted
                }
                Err(e) => {
                    app.error(format!("Could not delete: {e}"));
                    state.confirm_delete = false;
                    EditOutcome::Continue
                }
            },
            _ => {
                state.confirm_delete = false;
                EditOutcome::Continue
            }
        };
    }
    match action {
        KeyAction::CycleNext => {
            state.cycle_field();
            EditOutcome::Continue
        }
        KeyAction::Char(c) => {
            state.current_buffer_mut().push(c);
            EditOutcome::Continue
        }
        KeyAction::Backspace => {
            state.current_buffer_mut().pop();
            EditOutcome::Continue
        }
        KeyAction::Confirm => save(state, app),
        KeyAction::DeleteCtrlD => {
            state.confirm_delete = true;
            EditOutcome::Continue
        }
        KeyAction::Quit => EditOutcome::Cancelled,
        _ => EditOutcome::Continue,
    }
}

fn save<R: Repo>(state: &mut EditState, app: &mut App<R>) -> EditOutcome {
    let started_at = match parse_local(&state.started_at) {
        Some(dt) => dt,
        None => {
            app.error(format!("Invalid started_at: {}", state.started_at));
            return EditOutcome::Continue;
        }
    };
    let ended_at = if state.ended_at.trim().is_empty() {
        None
    } else {
        match parse_local(&state.ended_at) {
            Some(dt) => Some(dt),
            None => {
                app.error(format!("Invalid ended_at: {}", state.ended_at));
                return EditOutcome::Continue;
            }
        }
    };
    let notes = if state.notes.trim().is_empty() {
        None
    } else {
        Some(state.notes.as_str())
    };
    match app
        .repo
        .update_time_entry(state.entry_id, started_at, ended_at, notes)
    {
        Ok(_) => {
            app.info(format!("Saved entry #{}", state.entry_id));
            EditOutcome::Saved
        }
        Err(e) => {
            app.error(format!("Could not save: {e}"));
            EditOutcome::Continue
        }
    }
}

fn format_local(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Local)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

fn parse_local(s: &str) -> Option<DateTime<Utc>> {
    let naive = NaiveDateTime::parse_from_str(s.trim(), "%Y-%m-%d %H:%M").ok()?;
    Local
        .from_local_datetime(&naive)
        .single()
        .or_else(|| Local.from_local_datetime(&naive).earliest())
        .map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SqliteRepo;
    use crate::tui::app::App;
    use crate::tui::theme::Theme;
    use chrono::TimeZone;

    fn at(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 22, h, m, 0).unwrap()
    }

    fn fresh() -> (App<SqliteRepo>, EditState) {
        use crate::storage::Repo;
        let mut app = App::new(
            SqliteRepo::in_memory(),
            Theme::from_config_accent("cyan"),
            at(12, 0),
        );
        let task = app.repo.create_task("fix login", None).unwrap();
        let e = app.repo.create_time_entry(task.id, at(9, 0)).unwrap();
        app.repo.end_time_entry(e.id, at(10, 0)).unwrap();
        let entry = app.repo.list_entries_for_task(task.id).unwrap().remove(0);
        let state = EditState::for_entry(&entry, task.title);
        (app, state)
    }

    #[test]
    fn for_entry_seeds_local_strings() {
        let (_, state) = fresh();
        // The runner's local zone determines the exact text; check shape.
        assert!(state.started_at.contains(':'));
        assert!(state.ended_at.contains(':'));
        assert_eq!(state.field, EditField::StartedAt);
    }

    #[test]
    fn cycle_field_walks_started_ended_notes() {
        let (_, mut state) = fresh();
        assert_eq!(state.field, EditField::StartedAt);
        state.cycle_field();
        assert_eq!(state.field, EditField::EndedAt);
        state.cycle_field();
        assert_eq!(state.field, EditField::Notes);
        state.cycle_field();
        assert_eq!(state.field, EditField::StartedAt);
    }

    #[test]
    fn typing_into_notes_field_appends_to_buffer() {
        let (mut app, mut state) = fresh();
        state.field = EditField::Notes;
        handle_key(&mut state, &mut app, KeyAction::Char('o'));
        handle_key(&mut state, &mut app, KeyAction::Char('k'));
        assert_eq!(state.notes, "ok");
    }

    #[test]
    fn invalid_started_at_blocks_save_and_surfaces_error() {
        let (mut app, mut state) = fresh();
        state.started_at = "not a date".into();
        let outcome = handle_key(&mut state, &mut app, KeyAction::Confirm);
        assert_eq!(outcome, EditOutcome::Continue);
        assert!(matches!(
            app.footer,
            crate::tui::app::FooterMessage::Error(_)
        ));
    }

    #[test]
    fn ctrl_d_then_y_deletes() {
        let (mut app, mut state) = fresh();
        handle_key(&mut state, &mut app, KeyAction::DeleteCtrlD);
        assert!(state.confirm_delete);
        let outcome = handle_key(&mut state, &mut app, KeyAction::Char('y'));
        assert_eq!(outcome, EditOutcome::Deleted);
    }

    #[test]
    fn ctrl_d_then_n_cancels_confirm() {
        let (mut app, mut state) = fresh();
        handle_key(&mut state, &mut app, KeyAction::DeleteCtrlD);
        let outcome = handle_key(&mut state, &mut app, KeyAction::Char('n'));
        assert_eq!(outcome, EditOutcome::Continue);
        assert!(!state.confirm_delete);
    }

    #[test]
    fn esc_cancels_overlay() {
        let (mut app, mut state) = fresh();
        let outcome = handle_key(&mut state, &mut app, KeyAction::Quit);
        assert_eq!(outcome, EditOutcome::Cancelled);
    }
}
