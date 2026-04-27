//! Report aggregation: scope resolution, grouping, and totals.
//!
//! `Scope` resolves CLI flags into a UTC `[from, to)` window plus a kind
//! tag for display. `ReportBuilder` walks the entries the repo returns and
//! distributes their durations into groups.

use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeKind {
    Today,
    Week,
    Month,
    All,
    Range,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Scope {
    pub kind: ScopeKind,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Grouping {
    Task,
    Epic,
    Day,
}

#[derive(Debug, thiserror::Error)]
pub enum ScopeError {
    #[error("range must be FROM..TO with two YYYY-MM-DD dates")]
    RangeShape,
    #[error("range FROM ({from}) must be on or before TO ({to})")]
    RangeOrder { from: NaiveDate, to: NaiveDate },
    #[error("range date {0} is not YYYY-MM-DD")]
    RangeDate(String),
}

impl Scope {
    /// Today in local time, expressed as a UTC half-open interval.
    pub fn today(now: DateTime<Utc>) -> Self {
        let local_today = now.with_timezone(&chrono::Local).date_naive();
        let from = local_midnight_utc(local_today);
        let to = local_midnight_utc(local_today.succ_opt().expect("date does not overflow"));
        Self {
            kind: ScopeKind::Today,
            from,
            to,
        }
    }

    /// Current ISO week (Monday–Sunday) in local time.
    pub fn week(now: DateTime<Utc>) -> Self {
        let local_today = now.with_timezone(&chrono::Local).date_naive();
        let weekday = local_today.weekday();
        let days_since_monday = weekday.num_days_from_monday() as i64;
        let monday = local_today - Duration::days(days_since_monday);
        let next_monday = monday + Duration::days(7);
        Self {
            kind: ScopeKind::Week,
            from: local_midnight_utc(monday),
            to: local_midnight_utc(next_monday),
        }
    }

    /// Current calendar month in local time.
    pub fn month(now: DateTime<Utc>) -> Self {
        let local_today = now.with_timezone(&chrono::Local).date_naive();
        let first = NaiveDate::from_ymd_opt(local_today.year(), local_today.month(), 1)
            .expect("first day of month exists");
        let next_first = next_month_first(first);
        Self {
            kind: ScopeKind::Month,
            from: local_midnight_utc(first),
            to: local_midnight_utc(next_first),
        }
    }

    /// Everything ever recorded, plus a generous tail so currently-active
    /// timers count.
    pub fn all(now: DateTime<Utc>) -> Self {
        let from = Utc
            .with_ymd_and_hms(1970, 1, 1, 0, 0, 0)
            .single()
            .expect("Unix epoch is a valid UTC datetime");
        let to = now + Duration::days(1);
        Self {
            kind: ScopeKind::All,
            from,
            to,
        }
    }

    /// Parse `FROM..TO` with both endpoints as `YYYY-MM-DD` (inclusive whole days).
    pub fn range(input: &str) -> Result<Self, ScopeError> {
        let (from_str, to_str) = input.split_once("..").ok_or(ScopeError::RangeShape)?;
        if from_str.is_empty() || to_str.is_empty() {
            return Err(ScopeError::RangeShape);
        }
        let from_date = NaiveDate::parse_from_str(from_str, "%Y-%m-%d")
            .map_err(|_| ScopeError::RangeDate(from_str.to_string()))?;
        let to_date = NaiveDate::parse_from_str(to_str, "%Y-%m-%d")
            .map_err(|_| ScopeError::RangeDate(to_str.to_string()))?;
        if from_date > to_date {
            return Err(ScopeError::RangeOrder {
                from: from_date,
                to: to_date,
            });
        }
        let next_after_to = to_date.succ_opt().expect("date does not overflow");
        Ok(Self {
            kind: ScopeKind::Range,
            from: local_midnight_utc(from_date),
            to: local_midnight_utc(next_after_to),
        })
    }
}

fn local_midnight_utc(date: NaiveDate) -> DateTime<Utc> {
    let naive = date.and_hms_opt(0, 0, 0).expect("midnight always exists");
    chrono::Local
        .from_local_datetime(&naive)
        .single()
        .or_else(|| chrono::Local.from_local_datetime(&naive).earliest())
        .expect("local midnight resolvable (ambiguous DST handled by .earliest())")
        .with_timezone(&Utc)
}

fn next_month_first(first_of_month: NaiveDate) -> NaiveDate {
    let (y, m) = (first_of_month.year(), first_of_month.month());
    if m == 12 {
        NaiveDate::from_ymd_opt(y + 1, 1, 1).expect("Jan 1 of next year is always valid")
    } else {
        NaiveDate::from_ymd_opt(y, m + 1, 1)
            .expect("month incremented within 1..=11 is always valid")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub scope: Scope,
    pub grouping: Grouping,
    pub rows: Vec<ReportRow>,
    pub total_seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReportRow {
    pub label: String,
    pub duration_seconds: i64,
    /// `Some` only when [`Grouping::Task`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<i64>,
    /// `Some` only when [`Grouping::Task`] and the task links to a Shortcut story.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shortcut_external_id: Option<i64>,
    /// `Some` only when [`Grouping::Day`]. Format: YYYY-MM-DD (Local).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

pub struct ReportBuilder<'a, R: crate::storage::Repo> {
    repo: &'a R,
}

impl<'a, R: crate::storage::Repo> ReportBuilder<'a, R> {
    pub fn new(repo: &'a R) -> Self {
        Self { repo }
    }

    pub fn build(
        &self,
        scope: Scope,
        grouping: Grouping,
        now: DateTime<Utc>,
    ) -> Result<Report, crate::storage::RepoError> {
        match grouping {
            Grouping::Task => self.build_by_task(scope, now),
            Grouping::Epic => Err(crate::storage::RepoError::Sqlite(
                rusqlite::Error::InvalidQuery,
            )), // filled in by Task 10
            Grouping::Day => Err(crate::storage::RepoError::Sqlite(
                rusqlite::Error::InvalidQuery,
            )), // filled in by Task 8
        }
    }

    fn build_by_task(
        &self,
        scope: Scope,
        now: DateTime<Utc>,
    ) -> Result<Report, crate::storage::RepoError> {
        let entries = self.repo.list_entries_in_range(scope.from, scope.to, now)?;

        // task_id -> accumulated seconds
        let mut totals: std::collections::BTreeMap<i64, i64> = std::collections::BTreeMap::new();
        for e in &entries {
            let contribution = contribution_seconds(e, scope.from, scope.to, now);
            *totals.entry(e.task_id).or_default() += contribution;
        }

        let mut rows = Vec::with_capacity(totals.len());
        for (task_id, secs) in totals.iter() {
            let task = self
                .repo
                .find_task(*task_id)?
                .ok_or(crate::storage::RepoError::TaskNotFound(*task_id))?;
            let shortcut_external_id = match task.shortcut_story_id {
                Some(row_id) => self
                    .repo
                    .find_shortcut_story_by_row_id(row_id)?
                    .map(|s| s.external_id),
                None => None,
            };
            let label = match shortcut_external_id {
                Some(ext) => format!("SC-{ext} {}", task.title),
                None => task.title.clone(),
            };
            rows.push(ReportRow {
                label,
                duration_seconds: *secs,
                task_id: Some(task.id),
                shortcut_external_id,
                date: None,
            });
        }
        rows.sort_by(|a, b| b.duration_seconds.cmp(&a.duration_seconds));
        let total_seconds = rows.iter().map(|r| r.duration_seconds).sum();
        Ok(Report {
            scope,
            grouping: Grouping::Task,
            rows,
            total_seconds,
        })
    }
}

fn contribution_seconds(
    entry: &crate::domain::TimeEntry,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    now: DateTime<Utc>,
) -> i64 {
    let entry_end = entry.ended_at.unwrap_or(now);
    let start = entry.started_at.max(from);
    let end = entry_end.min(to);
    (end - start).max(Duration::zero()).num_seconds()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, 0, 0).unwrap()
    }

    #[test]
    fn today_spans_local_midnight_to_next_midnight() {
        let now = at(2026, 4, 27, 13);
        let s = Scope::today(now);
        assert_eq!(s.kind, ScopeKind::Today);
        // The window must contain `now`.
        assert!(s.from <= now && now < s.to);
        // Its width must be at most 25 hours (DST tolerance) and at least 23.
        let width = (s.to - s.from).num_hours();
        assert!((23..=25).contains(&width), "width was {width}h");
    }

    #[test]
    fn week_starts_on_monday_and_is_seven_days() {
        let monday = at(2026, 4, 27, 13); // Mon 2026-04-27 13:00 UTC
        let s = Scope::week(monday);
        assert_eq!(s.kind, ScopeKind::Week);
        let width = (s.to - s.from).num_days();
        assert_eq!(width, 7, "week width was {width}d, expected exactly 7");
    }

    #[test]
    fn month_uses_calendar_first_to_first() {
        let mid_april = at(2026, 4, 15, 12);
        let s = Scope::month(mid_april);
        let width = (s.to - s.from).num_days();
        // April has 30 days; allow 29..=31 for any DST or month-length edges.
        assert!((28..=31).contains(&width), "width was {width}d");
    }

    #[test]
    fn all_covers_a_huge_range_ending_in_the_future() {
        let now = at(2026, 4, 27, 13);
        let s = Scope::all(now);
        assert!(s.to > now);
        assert!(s.from < at(2000, 1, 1, 0));
    }

    #[test]
    fn range_parses_inclusive_dates() {
        let s = Scope::range("2026-04-01..2026-04-30").unwrap();
        assert_eq!(s.kind, ScopeKind::Range);
        let width = (s.to - s.from).num_days();
        assert!((29..=31).contains(&width));
    }

    #[test]
    fn range_rejects_bad_shape() {
        assert!(matches!(Scope::range(""), Err(ScopeError::RangeShape)));
        assert!(matches!(
            Scope::range("..2026-04-30"),
            Err(ScopeError::RangeShape)
        ));
        assert!(matches!(
            Scope::range("2026-04-01.."),
            Err(ScopeError::RangeShape)
        ));
        assert!(matches!(
            Scope::range("2026-04-01"),
            Err(ScopeError::RangeShape)
        ));
    }

    #[test]
    fn range_rejects_bad_dates() {
        assert!(matches!(
            Scope::range("2026-13-01..2026-04-30"),
            Err(ScopeError::RangeDate(_))
        ));
    }

    #[test]
    fn range_rejects_inverted_endpoints() {
        assert!(matches!(
            Scope::range("2026-04-30..2026-04-01"),
            Err(ScopeError::RangeOrder { .. })
        ));
    }

    use crate::storage::SqliteRepo;

    fn seed_closed_entry(
        r: &mut SqliteRepo,
        task_id: i64,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) {
        use crate::storage::Repo;
        let e = r.create_time_entry(task_id, start).unwrap();
        r.end_time_entry(e.id, end).unwrap();
    }

    #[test]
    fn build_by_task_aggregates_per_task_within_scope() {
        use crate::storage::Repo;
        let mut r = SqliteRepo::in_memory();
        let t1 = r.create_task("alpha", None).unwrap();
        let t2 = r.create_task("beta", None).unwrap();

        let from = at(2026, 4, 22, 0);
        let to = at(2026, 4, 23, 0);
        let now = at(2026, 4, 22, 18);

        // Two closed entries on t1, totalling 90 minutes.
        seed_closed_entry(&mut r, t1.id, at(2026, 4, 22, 9), at(2026, 4, 22, 10));
        seed_closed_entry(
            &mut r,
            t1.id,
            at(2026, 4, 22, 11),
            at(2026, 4, 22, 11) + Duration::minutes(30),
        );
        // One closed entry on t2, 30 minutes.
        seed_closed_entry(
            &mut r,
            t2.id,
            at(2026, 4, 22, 13),
            at(2026, 4, 22, 13) + Duration::minutes(30),
        );

        let scope = Scope {
            kind: ScopeKind::Today,
            from,
            to,
        };
        let report = ReportBuilder::new(&r)
            .build(scope, Grouping::Task, now)
            .unwrap();
        assert_eq!(report.rows.len(), 2);
        // Sorted descending by duration: t1 first.
        assert_eq!(report.rows[0].task_id, Some(t1.id));
        assert_eq!(report.rows[0].duration_seconds, 90 * 60);
        assert_eq!(report.rows[1].task_id, Some(t2.id));
        assert_eq!(report.rows[1].duration_seconds, 30 * 60);
        assert_eq!(report.total_seconds, 120 * 60);
        assert_eq!(report.grouping, Grouping::Task);
    }

    #[test]
    fn build_by_task_clamps_entries_to_scope_window() {
        use crate::storage::Repo;
        let mut r = SqliteRepo::in_memory();
        let t = r.create_task("over the edge", None).unwrap();

        let from = at(2026, 4, 22, 0);
        let to = at(2026, 4, 23, 0);
        let now = at(2026, 4, 22, 18);

        // Entry started yesterday at 23:00, ended today at 02:00 — only the 2h on the
        // today side should count.
        seed_closed_entry(&mut r, t.id, at(2026, 4, 21, 23), at(2026, 4, 22, 2));

        let scope = Scope {
            kind: ScopeKind::Today,
            from,
            to,
        };
        let report = ReportBuilder::new(&r)
            .build(scope, Grouping::Task, now)
            .unwrap();
        assert_eq!(report.rows.len(), 1);
        assert_eq!(report.rows[0].duration_seconds, 2 * 3600);
    }

    #[test]
    fn build_by_task_counts_active_entry_up_to_now() {
        use crate::storage::Repo;
        let mut r = SqliteRepo::in_memory();
        let t = r.create_task("active", None).unwrap();
        let from = at(2026, 4, 22, 0);
        let to = at(2026, 4, 23, 0);
        let now = at(2026, 4, 22, 14);

        // Active entry started at 13:00, no end. Now is 14:00 → 1h should count.
        r.create_time_entry(t.id, at(2026, 4, 22, 13)).unwrap();

        let scope = Scope {
            kind: ScopeKind::Today,
            from,
            to,
        };
        let report = ReportBuilder::new(&r)
            .build(scope, Grouping::Task, now)
            .unwrap();
        assert_eq!(report.rows.len(), 1);
        assert_eq!(report.rows[0].duration_seconds, 3600);
    }

    #[test]
    fn build_by_task_includes_shortcut_external_id_when_linked() {
        use crate::shortcut::Story;
        use crate::storage::Repo;
        let mut r = SqliteRepo::in_memory();
        // NOTE: `Story` does not yet have `epic_id` in this task — Task 9 adds
        // it and updates this fixture to set `epic_id: None`. Do not include
        // the field here; the test would not compile until Task 9 runs.
        let row = r
            .upsert_shortcut_story(
                &Story {
                    external_id: 555,
                    title: Some("from sc".into()),
                    epic_name: None,
                    state: None,
                },
                Utc::now(),
            )
            .unwrap();
        let t = r.create_task("linked", None).unwrap();
        r.link_task_to_story(t.id, row.id, Utc::now()).unwrap();
        seed_closed_entry(&mut r, t.id, at(2026, 4, 22, 10), at(2026, 4, 22, 11));

        let scope = Scope {
            kind: ScopeKind::Today,
            from: at(2026, 4, 22, 0),
            to: at(2026, 4, 23, 0),
        };
        let now = at(2026, 4, 22, 12);
        let report = ReportBuilder::new(&r)
            .build(scope, Grouping::Task, now)
            .unwrap();
        assert_eq!(report.rows[0].shortcut_external_id, Some(555));
        assert!(report.rows[0].label.starts_with("SC-555 "));
    }
}
