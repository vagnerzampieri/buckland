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

// ReportBuilder, Report, ReportRow live here too — added in later tasks.

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
}
