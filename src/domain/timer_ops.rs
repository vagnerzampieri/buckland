//! Timer operations: start, stop. Enforces the single-active-timer invariant
//! by opening a transaction, ending any active entry, then creating the new
//! one — all in one shot.

use crate::domain::TimeEntry;
use crate::storage::repo::TIME_ENTRY_COLS;
use crate::storage::{RepoResult, SqliteRepo};
use chrono::{DateTime, Utc};
use rusqlite::params;
use rusqlite::OptionalExtension;

pub struct TimerOps<'a> {
    repo: &'a mut SqliteRepo,
}

impl<'a> TimerOps<'a> {
    pub fn new(repo: &'a mut SqliteRepo) -> Self {
        Self { repo }
    }

    /// Start a timer for `task_id` at `now`. If any timer is active, end it
    /// at the exact same `now` timestamp so there is no overlap and no gap
    /// between entries. Both actions share a transaction so there's never
    /// a moment with two active entries.
    pub fn start(&mut self, task_id: i64, now: DateTime<Utc>) -> RepoResult<TimeEntry> {
        let tx = self.repo.transaction()?;
        tx.execute(
            "UPDATE time_entries SET ended_at = ?1 WHERE ended_at IS NULL",
            params![now],
        )?;
        tx.execute(
            "INSERT INTO time_entries (task_id, started_at) VALUES (?1, ?2)",
            params![task_id, now],
        )?;
        let id = tx.last_insert_rowid();
        let entry = tx.query_row(
            &format!("SELECT {TIME_ENTRY_COLS} FROM time_entries WHERE id = ?1"),
            [id],
            |row| TimeEntry::try_from(row),
        )?;
        tx.commit()?;
        Ok(entry)
    }

    /// Stop the active timer, if any. Returns the ended entry, or `None`
    /// if there was nothing to stop.
    pub fn stop(&mut self, now: DateTime<Utc>) -> RepoResult<Option<TimeEntry>> {
        let tx = self.repo.transaction()?;
        let active_id: Option<i64> = tx
            .query_row(
                "SELECT id FROM time_entries WHERE ended_at IS NULL LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let Some(id) = active_id else {
            tx.commit()?;
            return Ok(None);
        };
        tx.execute(
            "UPDATE time_entries SET ended_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        let entry = tx.query_row(
            &format!("SELECT {TIME_ENTRY_COLS} FROM time_entries WHERE id = ?1"),
            [id],
            |row| TimeEntry::try_from(row),
        )?;
        tx.commit()?;
        Ok(Some(entry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Repo;
    use chrono::TimeZone;

    fn at(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 22, h, 0, 0).unwrap()
    }

    #[test]
    fn start_creates_entry_and_makes_it_active() {
        let mut repo = SqliteRepo::in_memory();
        let t = repo.create_task("t", None).unwrap();
        let e = TimerOps::new(&mut repo).start(t.id, at(10)).unwrap();
        assert!(e.is_active());
        assert_eq!(repo.active_time_entry().unwrap().unwrap().id, e.id);
    }

    #[test]
    fn start_stops_previously_active_entry() {
        let mut repo = SqliteRepo::in_memory();
        let a = repo.create_task("a", None).unwrap();
        let b = repo.create_task("b", None).unwrap();
        let first = TimerOps::new(&mut repo).start(a.id, at(9)).unwrap();
        let second = TimerOps::new(&mut repo).start(b.id, at(10)).unwrap();

        // Only `second` is active now.
        let active = repo.active_time_entry().unwrap().unwrap();
        assert_eq!(active.id, second.id);

        // `first` got ended at the switch time.
        let entries = repo.list_entries_for_task(a.id).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].ended_at, Some(at(10)));
        let _ = first;
    }

    #[test]
    fn stop_noop_when_idle() {
        let mut repo = SqliteRepo::in_memory();
        assert!(TimerOps::new(&mut repo).stop(at(10)).unwrap().is_none());
    }

    #[test]
    fn stop_ends_active_entry() {
        let mut repo = SqliteRepo::in_memory();
        let t = repo.create_task("t", None).unwrap();
        TimerOps::new(&mut repo).start(t.id, at(9)).unwrap();
        let ended = TimerOps::new(&mut repo).stop(at(10)).unwrap().unwrap();
        assert_eq!(ended.ended_at, Some(at(10)));
        assert!(repo.active_time_entry().unwrap().is_none());
    }

    #[test]
    fn never_two_active_across_start_calls() {
        let mut repo = SqliteRepo::in_memory();
        let t = repo.create_task("t", None).unwrap();
        for h in 9..15 {
            TimerOps::new(&mut repo).start(t.id, at(h)).unwrap();
            let active_count: i64 = repo
                .connection()
                .query_row(
                    "SELECT COUNT(*) FROM time_entries WHERE ended_at IS NULL",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(active_count, 1, "two active at hour {h}");
        }
    }

    #[test]
    fn start_with_unknown_task_returns_error() {
        let mut repo = SqliteRepo::in_memory();
        assert!(TimerOps::new(&mut repo).start(999, at(10)).is_err());
    }
}
