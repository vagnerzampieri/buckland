use chrono::{DateTime, Duration, Utc};
use rusqlite::Row;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeEntry {
    pub id: i64,
    pub task_id: i64,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl TimeEntry {
    pub fn is_active(&self) -> bool {
        self.ended_at.is_none()
    }

    /// Duration of this entry. For active entries, uses `now` as the upper
    /// bound. Never negative — clamped to zero if clock goes backward.
    pub fn duration(&self, now: DateTime<Utc>) -> Duration {
        let end = self.ended_at.unwrap_or(now);
        let d = end - self.started_at;
        if d < Duration::zero() {
            Duration::zero()
        } else {
            d
        }
    }
}

impl<'a> TryFrom<&Row<'a>> for TimeEntry {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'a>) -> Result<Self, Self::Error> {
        Ok(TimeEntry {
            id: row.get("id")?,
            task_id: row.get("task_id")?,
            started_at: row.get("started_at")?,
            ended_at: row.get("ended_at")?,
            notes: row.get("notes")?,
            created_at: row.get("created_at")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn start_at(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 22, h, m, 0).unwrap()
    }

    #[test]
    fn active_entry_duration_uses_now() {
        let e = TimeEntry {
            id: 1,
            task_id: 1,
            started_at: start_at(10, 0),
            ended_at: None,
            notes: None,
            created_at: start_at(10, 0),
        };
        assert_eq!(e.duration(start_at(10, 30)), Duration::minutes(30));
    }

    #[test]
    fn closed_entry_uses_ended_at() {
        let e = TimeEntry {
            id: 1,
            task_id: 1,
            started_at: start_at(10, 0),
            ended_at: Some(start_at(11, 23)),
            notes: None,
            created_at: start_at(10, 0),
        };
        assert_eq!(e.duration(start_at(23, 0)), Duration::minutes(83));
    }

    #[test]
    fn negative_duration_clamps_to_zero() {
        let e = TimeEntry {
            id: 1,
            task_id: 1,
            started_at: start_at(12, 0),
            ended_at: None,
            notes: None,
            created_at: start_at(12, 0),
        };
        assert_eq!(e.duration(start_at(10, 0)), Duration::zero());
    }
}
