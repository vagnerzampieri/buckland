//! Report screen — wraps the Phase C `ReportBuilder` output. Tab cycles
//! scope, T cycles grouping, c copies a one-liner, j toggles a JSON dump.

use crate::cli::format::{bar, duration_compact};
use crate::domain::{Grouping, Report, ReportBuilder, Scope, ScopeKind};
use crate::storage::Repo;
use crate::tui::app::App;
use crate::tui::clipboard;
use crate::tui::keymap::KeyAction;
use chrono::{DateTime, Utc};
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

#[derive(Debug, Clone)]
pub struct ReportState {
    pub scope_kind: ScopeKind,
    pub grouping: Grouping,
    pub json: bool,
    pub report: Option<Report>,
}

impl Default for ReportState {
    fn default() -> Self {
        Self {
            scope_kind: ScopeKind::Today,
            grouping: Grouping::Task,
            json: false,
            report: None,
        }
    }
}

impl ReportState {
    pub fn refresh<R: Repo>(&mut self, repo: &R, now: DateTime<Utc>) -> anyhow::Result<()> {
        let scope = match self.scope_kind {
            ScopeKind::Today => Scope::today(now),
            ScopeKind::Week => Scope::week(now),
            ScopeKind::Month => Scope::month(now),
            ScopeKind::All => Scope::all(now),
            ScopeKind::Range => Scope::today(now), // Custom ranges are CLI-only.
        };
        let report = ReportBuilder::new(repo).build(scope, self.grouping, now)?;
        self.report = Some(report);
        Ok(())
    }

    pub fn cycle_scope(&mut self) {
        self.scope_kind = match self.scope_kind {
            ScopeKind::Today => ScopeKind::Week,
            ScopeKind::Week => ScopeKind::Month,
            ScopeKind::Month => ScopeKind::All,
            ScopeKind::All | ScopeKind::Range => ScopeKind::Today,
        };
    }

    pub fn cycle_grouping(&mut self) {
        self.grouping = match self.grouping {
            Grouping::Task => Grouping::Epic,
            Grouping::Epic => Grouping::Day,
            Grouping::Day => Grouping::Task,
        };
    }
}

pub fn render<R: Repo>(frame: &mut Frame<'_>, area: Rect, app: &App<R>, state: &ReportState) {
    let title = format!(
        "Report — {} / {}",
        scope_label(state.scope_kind),
        grouping_label(state.grouping)
    );
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let body = match (&state.report, state.json) {
        (None, _) => Paragraph::new("Loading…").style(app.theme.dim()),
        (Some(report), true) => {
            let json = serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".into());
            Paragraph::new(json).wrap(Wrap { trim: false })
        }
        (Some(report), false) => Paragraph::new(table_lines(report)).wrap(Wrap { trim: false }),
    };
    frame.render_widget(body, inner);
}

fn scope_label(kind: ScopeKind) -> &'static str {
    match kind {
        ScopeKind::Today => "Today",
        ScopeKind::Week => "This week",
        ScopeKind::Month => "This month",
        ScopeKind::All => "All time",
        ScopeKind::Range => "Range",
    }
}

fn grouping_label(g: Grouping) -> &'static str {
    match g {
        Grouping::Task => "by task",
        Grouping::Epic => "by epic",
        Grouping::Day => "by day",
    }
}

fn table_lines(report: &Report) -> Vec<Line<'static>> {
    if report.rows.is_empty() {
        return vec![Line::from("No time tracked in this scope.")];
    }
    let max = report
        .rows
        .iter()
        .map(|r| r.duration_seconds)
        .max()
        .unwrap_or(0);
    let mut lines: Vec<Line> = report
        .rows
        .iter()
        .map(|row| {
            let total = chrono::Duration::seconds(row.duration_seconds);
            Line::from(format!(
                "{:<48}  {:>8}  {}",
                crate::cli::commands_truncate(&row.label, 48),
                duration_compact(total),
                bar(row.duration_seconds, max, 30),
            ))
        })
        .collect();
    let total = chrono::Duration::seconds(report.total_seconds);
    lines.push(Line::from(vec![Span::styled(
        format!("{:<48}  {:>8}", "Total", duration_compact(total)),
        ratatui::style::Style::default().add_modifier(Modifier::BOLD),
    )]));
    lines
}

pub fn handle_key<R: Repo>(state: &mut ReportState, app: &mut App<R>, action: KeyAction) -> bool {
    match action {
        KeyAction::CycleNext => {
            state.cycle_scope();
            let _ = state.refresh(&app.repo, app.now);
            true
        }
        KeyAction::CycleGroup => {
            state.cycle_grouping();
            let _ = state.refresh(&app.repo, app.now);
            true
        }
        KeyAction::Down | KeyAction::ToggleJson => {
            // `j` resolves to `Down` in the global keymap. The Report screen
            // has no row navigation (spec: "Report has no row selection"),
            // so on this screen `Down` is repurposed as the JSON toggle.
            state.json = !state.json;
            true
        }
        KeyAction::Copy => {
            copy_one_liner(state, app);
            true
        }
        _ => false,
    }
}

fn copy_one_liner<R: Repo>(state: &ReportState, app: &mut App<R>) {
    let Some(report) = &state.report else {
        app.info("Nothing to copy yet.");
        return;
    };
    let total = chrono::Duration::seconds(report.total_seconds);
    let one_liner = format!(
        "{}: {} ({} rows)",
        scope_label(state.scope_kind),
        duration_compact(total),
        report.rows.len()
    );
    match clipboard::copy(&one_liner) {
        Ok(tool) => app.info(format!("Copied via {tool}")),
        Err(e) => app.error(format!("Copy failed: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SqliteRepo;
    use crate::tui::app::App;
    use crate::tui::theme::Theme;
    use chrono::TimeZone;

    fn at(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, 0, 0).unwrap()
    }

    fn fresh() -> (App<SqliteRepo>, ReportState) {
        let now = at(2026, 4, 22, 10);
        (
            App::new(
                SqliteRepo::in_memory(),
                Theme::from_config_accent("cyan"),
                now,
            ),
            ReportState::default(),
        )
    }

    #[test]
    fn defaults_today_by_task() {
        let s = ReportState::default();
        assert_eq!(s.scope_kind, ScopeKind::Today);
        assert_eq!(s.grouping, Grouping::Task);
        assert!(!s.json);
    }

    #[test]
    fn cycle_scope_walks_today_week_month_all() {
        let mut s = ReportState::default();
        assert_eq!(s.scope_kind, ScopeKind::Today);
        s.cycle_scope();
        assert_eq!(s.scope_kind, ScopeKind::Week);
        s.cycle_scope();
        assert_eq!(s.scope_kind, ScopeKind::Month);
        s.cycle_scope();
        assert_eq!(s.scope_kind, ScopeKind::All);
        s.cycle_scope();
        assert_eq!(s.scope_kind, ScopeKind::Today);
    }

    #[test]
    fn cycle_grouping_walks_task_epic_day() {
        let mut s = ReportState::default();
        assert_eq!(s.grouping, Grouping::Task);
        s.cycle_grouping();
        assert_eq!(s.grouping, Grouping::Epic);
        s.cycle_grouping();
        assert_eq!(s.grouping, Grouping::Day);
        s.cycle_grouping();
        assert_eq!(s.grouping, Grouping::Task);
    }

    #[test]
    fn refresh_populates_report() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        let t = app.repo.create_task("alpha", None).unwrap();
        let e = app
            .repo
            .create_time_entry(t.id, at(2026, 4, 22, 9))
            .unwrap();
        app.repo.end_time_entry(e.id, at(2026, 4, 22, 10)).unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        let r = state.report.as_ref().unwrap();
        assert_eq!(r.rows.len(), 1);
        assert_eq!(r.total_seconds, 3600);
    }

    #[test]
    fn handle_tab_cycles_scope_and_refreshes() {
        let (mut app, mut state) = fresh();
        let acted = handle_key(&mut state, &mut app, KeyAction::CycleNext);
        assert!(acted);
        assert_eq!(state.scope_kind, ScopeKind::Week);
    }

    #[test]
    fn handle_t_cycles_grouping() {
        let (mut app, mut state) = fresh();
        handle_key(&mut state, &mut app, KeyAction::CycleGroup);
        assert_eq!(state.grouping, Grouping::Epic);
    }

    #[test]
    fn handle_down_toggles_json_on_report_screen() {
        // `j` resolves to KeyAction::Down globally; on Report it's the
        // JSON toggle.
        let (mut app, mut state) = fresh();
        assert!(!state.json);
        handle_key(&mut state, &mut app, KeyAction::Down);
        assert!(state.json);
        handle_key(&mut state, &mut app, KeyAction::Down);
        assert!(!state.json);
    }
}
