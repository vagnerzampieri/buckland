//! The storage contract consumed by the domain layer.
//!
//! `Repo` abstracts the concrete database. `SqliteRepo` is the production
//! implementation. Tests can build on an in-memory SqliteRepo via
//! `SqliteRepo::in_memory()`.

use crate::domain::{ShortcutStory, Task, TimeEntry};
use crate::shortcut::Story;
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("task {0} not found")]
    TaskNotFound(i64),
    #[error("time entry {0} not found")]
    TimeEntryNotFound(i64),
    #[error("cannot hard-delete task {0}: it has time entries. Use archive instead.")]
    TaskHasEntries(i64),
    /// The database file does not exist on disk. Distinct from a generic
    /// open error so the tray can render "no database yet" instead of an
    /// alarming "cannot read database" message.
    #[error("database file not found: {0}")]
    DatabaseMissing(std::path::PathBuf),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

pub type RepoResult<T> = Result<T, RepoError>;

pub trait Repo {
    fn create_task(&mut self, title: &str, description: Option<&str>) -> RepoResult<Task>;
    fn find_task(&self, id: i64) -> RepoResult<Option<Task>>;
    fn list_open_tasks(&self) -> RepoResult<Vec<Task>>;
    fn list_completed_tasks(&self) -> RepoResult<Vec<Task>>;
    fn list_archived_tasks(&self) -> RepoResult<Vec<Task>>;
    fn list_all_tasks(&self) -> RepoResult<Vec<Task>>;
    fn mark_task_done(&mut self, id: i64, at: DateTime<Utc>) -> RepoResult<Task>;
    fn archive_task(&mut self, id: i64, at: DateTime<Utc>) -> RepoResult<Task>;
    fn delete_task(&mut self, id: i64) -> RepoResult<()>;

    fn create_time_entry(
        &mut self,
        task_id: i64,
        started_at: DateTime<Utc>,
    ) -> RepoResult<TimeEntry>;
    /// Ends the entry with the given `id`. Returns
    /// [`RepoError::TimeEntryNotFound`] if no row matches the id OR if the
    /// row exists but has already been ended.
    fn end_time_entry(&mut self, id: i64, ended_at: DateTime<Utc>) -> RepoResult<TimeEntry>;
    fn active_time_entry(&self) -> RepoResult<Option<TimeEntry>>;
    /// One-shot read of the currently active timer joined with its task
    /// and (if linked) the Shortcut story's `external_id`. Returns
    /// `Ok(None)` when no row in `time_entries` has `ended_at IS NULL`.
    fn active_snapshot(&self) -> RepoResult<Option<crate::domain::ActiveSnapshot>>;
    fn list_entries_for_task(&self, task_id: i64) -> RepoResult<Vec<TimeEntry>>;
    fn task_total_duration(&self, task_id: i64, now: DateTime<Utc>) -> RepoResult<Duration>;
    fn delete_time_entry(&mut self, id: i64) -> RepoResult<()>;

    /// Update started_at, ended_at, and/or notes on a time entry. Pass
    /// `None` to clear the optional fields.
    fn update_time_entry(
        &mut self,
        id: i64,
        started_at: DateTime<Utc>,
        ended_at: Option<DateTime<Utc>>,
        notes: Option<&str>,
    ) -> RepoResult<TimeEntry>;

    /// All time entries whose `[started_at, ended_at_or_now)` interval
    /// overlaps the half-open UTC range `[from, to)`.
    /// Active entries (where `ended_at IS NULL`) are evaluated against `now`.
    /// Ordered by `started_at ASC`.
    fn list_entries_in_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> RepoResult<Vec<TimeEntry>>;

    fn upsert_shortcut_story(
        &mut self,
        story: &Story,
        fetched_at: DateTime<Utc>,
    ) -> RepoResult<ShortcutStory>;
    fn find_shortcut_story_by_external_id(
        &self,
        external_id: i64,
    ) -> RepoResult<Option<ShortcutStory>>;
    fn find_shortcut_story_by_row_id(&self, id: i64) -> RepoResult<Option<ShortcutStory>>;
    fn link_task_to_story(
        &mut self,
        task_id: i64,
        story_row_id: i64,
        at: DateTime<Utc>,
    ) -> RepoResult<Task>;
    fn find_task_by_story_external_id(&self, external_id: i64) -> RepoResult<Option<Task>>;
}

pub struct SqliteRepo {
    // Connection doesn't implement Debug; we derive Debug manually with a
    // placeholder so test panics and error messages can format the type.
    conn: Connection,
}

impl std::fmt::Debug for SqliteRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteRepo").finish_non_exhaustive()
    }
}

impl SqliteRepo {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    /// Open the database at `path` for read-only access. Returns
    /// [`RepoError::DatabaseMissing`] if the file does not exist (so the
    /// tray can render "no database yet" without alarming the user) and
    /// [`RepoError::Sqlite`] for any other failure (locked, corrupt,
    /// permission denied).
    pub fn open_read_only(path: &std::path::Path) -> RepoResult<Self> {
        if !path.exists() {
            return Err(RepoError::DatabaseMissing(path.to_path_buf()));
        }
        let conn = rusqlite::Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        // Foreign keys aren't enforced by default on read-only connections;
        // we don't need them for SELECTs but turning them on costs nothing.
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "busy_timeout", 3000)?;
        Ok(Self { conn })
    }

    #[cfg(test)]
    pub fn in_memory() -> Self {
        let conn = super::open_memory().expect("open in-memory DB");
        Self { conn }
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn transaction(&mut self) -> RepoResult<Transaction<'_>> {
        Ok(self.conn.transaction()?)
    }
}

const TASK_COLS: &str =
    "id, title, description, shortcut_story_id, completed_at, archived_at, created_at, updated_at";

const SHORTCUT_STORY_COLS: &str = "id, external_id, title, epic_name, state, fetched_at";

pub(crate) const TIME_ENTRY_COLS: &str = "id, task_id, started_at, ended_at, notes, created_at";

fn load_task(conn: &Connection, id: i64) -> RepoResult<Task> {
    conn.query_row(
        &format!("SELECT {TASK_COLS} FROM tasks WHERE id = ?1"),
        [id],
        |row| Task::try_from(row),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => RepoError::TaskNotFound(id),
        other => RepoError::Sqlite(other),
    })
}

fn list_tasks_where(conn: &Connection, where_clause: &str) -> RepoResult<Vec<Task>> {
    let sql =
        format!("SELECT {TASK_COLS} FROM tasks WHERE {where_clause} ORDER BY created_at DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| Task::try_from(row))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

impl Repo for SqliteRepo {
    fn create_task(&mut self, title: &str, description: Option<&str>) -> RepoResult<Task> {
        let now = Utc::now();
        self.conn.execute(
            "INSERT INTO tasks (title, description, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?3)",
            params![title, description, now],
        )?;
        let id = self.conn.last_insert_rowid();
        load_task(&self.conn, id)
    }

    fn find_task(&self, id: i64) -> RepoResult<Option<Task>> {
        self.conn
            .query_row(
                &format!("SELECT {TASK_COLS} FROM tasks WHERE id = ?1"),
                [id],
                |row| Task::try_from(row),
            )
            .optional()
            .map_err(RepoError::from)
    }

    fn list_open_tasks(&self) -> RepoResult<Vec<Task>> {
        list_tasks_where(&self.conn, "completed_at IS NULL AND archived_at IS NULL")
    }

    fn list_completed_tasks(&self) -> RepoResult<Vec<Task>> {
        list_tasks_where(&self.conn, "completed_at IS NOT NULL")
    }

    fn list_archived_tasks(&self) -> RepoResult<Vec<Task>> {
        list_tasks_where(&self.conn, "archived_at IS NOT NULL")
    }

    fn list_all_tasks(&self) -> RepoResult<Vec<Task>> {
        list_tasks_where(&self.conn, "1 = 1")
    }

    fn mark_task_done(&mut self, id: i64, at: DateTime<Utc>) -> RepoResult<Task> {
        let updated = self.conn.execute(
            "UPDATE tasks SET completed_at = ?1, updated_at = ?1 \
             WHERE id = ?2 AND completed_at IS NULL",
            params![at, id],
        )?;
        if updated == 0 {
            // Distinguish "not found" from "already done" so the CLI can give a
            // better message.
            let exists: bool = self
                .conn
                .query_row("SELECT 1 FROM tasks WHERE id = ?1", [id], |_| Ok(true))
                .optional()?
                .unwrap_or(false);
            if !exists {
                return Err(RepoError::TaskNotFound(id));
            }
            // Already done — no-op, return the existing row (with its original completed_at).
        }
        load_task(&self.conn, id)
    }

    fn archive_task(&mut self, id: i64, at: DateTime<Utc>) -> RepoResult<Task> {
        let updated = self.conn.execute(
            "UPDATE tasks SET archived_at = ?1, updated_at = ?1 \
             WHERE id = ?2 AND archived_at IS NULL",
            params![at, id],
        )?;
        if updated == 0 {
            // Distinguish "not found" from "already archived" so the CLI can give a
            // better message.
            let exists: bool = self
                .conn
                .query_row("SELECT 1 FROM tasks WHERE id = ?1", [id], |_| Ok(true))
                .optional()?
                .unwrap_or(false);
            if !exists {
                return Err(RepoError::TaskNotFound(id));
            }
            // Already archived — no-op, return the existing row (with its original archived_at).
        }
        load_task(&self.conn, id)
    }

    fn delete_task(&mut self, id: i64) -> RepoResult<()> {
        let entry_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM time_entries WHERE task_id = ?1",
            [id],
            |row| row.get(0),
        )?;
        if entry_count > 0 {
            return Err(RepoError::TaskHasEntries(id));
        }
        let removed = self.conn.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
        if removed == 0 {
            return Err(RepoError::TaskNotFound(id));
        }
        Ok(())
    }

    fn create_time_entry(
        &mut self,
        task_id: i64,
        started_at: DateTime<Utc>,
    ) -> RepoResult<TimeEntry> {
        self.conn.execute(
            "INSERT INTO time_entries (task_id, started_at) VALUES (?1, ?2)",
            params![task_id, started_at],
        )?;
        let id = self.conn.last_insert_rowid();
        self.conn
            .query_row(
                &format!("SELECT {TIME_ENTRY_COLS} FROM time_entries WHERE id = ?1"),
                [id],
                |row| TimeEntry::try_from(row),
            )
            .map_err(RepoError::from)
    }

    fn end_time_entry(&mut self, id: i64, ended_at: DateTime<Utc>) -> RepoResult<TimeEntry> {
        let updated = self.conn.execute(
            "UPDATE time_entries SET ended_at = ?1 WHERE id = ?2 AND ended_at IS NULL",
            params![ended_at, id],
        )?;
        if updated == 0 {
            return Err(RepoError::TimeEntryNotFound(id));
        }
        self.conn
            .query_row(
                &format!("SELECT {TIME_ENTRY_COLS} FROM time_entries WHERE id = ?1"),
                [id],
                |row| TimeEntry::try_from(row),
            )
            .map_err(RepoError::from)
    }

    fn active_time_entry(&self) -> RepoResult<Option<TimeEntry>> {
        self.conn
            .query_row(
                &format!(
                    "SELECT {TIME_ENTRY_COLS} FROM time_entries WHERE ended_at IS NULL LIMIT 1"
                ),
                [],
                |row| TimeEntry::try_from(row),
            )
            .optional()
            .map_err(RepoError::from)
    }

    fn active_snapshot(&self) -> RepoResult<Option<crate::domain::ActiveSnapshot>> {
        use crate::domain::ActiveSnapshot;
        let sql = "SELECT te.task_id, t.title, ss.external_id, te.started_at \
                   FROM time_entries te \
                   JOIN tasks t ON t.id = te.task_id \
                   LEFT JOIN shortcut_stories ss ON ss.id = t.shortcut_story_id \
                   WHERE te.ended_at IS NULL \
                   LIMIT 1";
        self.conn
            .query_row(sql, [], |row| {
                Ok(ActiveSnapshot {
                    task_id: row.get(0)?,
                    task_title: row.get(1)?,
                    sc_external_id: row.get(2)?,
                    started_at: row.get(3)?,
                })
            })
            .optional()
            .map_err(RepoError::from)
    }

    fn list_entries_for_task(&self, task_id: i64) -> RepoResult<Vec<TimeEntry>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {TIME_ENTRY_COLS} FROM time_entries WHERE task_id = ?1 ORDER BY started_at DESC"
        ))?;
        let rows = stmt.query_map([task_id], |row| TimeEntry::try_from(row))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    fn task_total_duration(&self, task_id: i64, now: DateTime<Utc>) -> RepoResult<Duration> {
        let entries = self.list_entries_for_task(task_id)?;
        Ok(entries
            .iter()
            .fold(Duration::zero(), |acc, e| acc + e.duration(now)))
    }

    fn delete_time_entry(&mut self, id: i64) -> RepoResult<()> {
        let removed = self
            .conn
            .execute("DELETE FROM time_entries WHERE id = ?1", [id])?;
        if removed == 0 {
            return Err(RepoError::TimeEntryNotFound(id));
        }
        Ok(())
    }

    fn update_time_entry(
        &mut self,
        id: i64,
        started_at: DateTime<Utc>,
        ended_at: Option<DateTime<Utc>>,
        notes: Option<&str>,
    ) -> RepoResult<TimeEntry> {
        let updated = self.conn.execute(
            "UPDATE time_entries SET started_at = ?1, ended_at = ?2, notes = ?3 WHERE id = ?4",
            params![started_at, ended_at, notes, id],
        )?;
        if updated == 0 {
            return Err(RepoError::TimeEntryNotFound(id));
        }
        self.conn
            .query_row(
                &format!("SELECT {TIME_ENTRY_COLS} FROM time_entries WHERE id = ?1"),
                [id],
                |row| TimeEntry::try_from(row),
            )
            .map_err(RepoError::from)
    }

    fn list_entries_in_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> RepoResult<Vec<TimeEntry>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {TIME_ENTRY_COLS} FROM time_entries \
             WHERE started_at < ?1 \
               AND COALESCE(ended_at, ?2) > ?3 \
             ORDER BY started_at ASC"
        ))?;
        let rows = stmt.query_map(params![to, now, from], |row| TimeEntry::try_from(row))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    fn upsert_shortcut_story(
        &mut self,
        story: &Story,
        fetched_at: DateTime<Utc>,
    ) -> RepoResult<ShortcutStory> {
        self.conn.execute(
            "INSERT INTO shortcut_stories \
                 (external_id, title, epic_name, state, fetched_at) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(external_id) DO UPDATE SET \
                 title = excluded.title, \
                 epic_name = excluded.epic_name, \
                 state = excluded.state, \
                 fetched_at = excluded.fetched_at",
            params![
                story.external_id,
                story.title,
                story.epic_name,
                story.state,
                fetched_at,
            ],
        )?;
        self.conn
            .query_row(
                &format!(
                    "SELECT {SHORTCUT_STORY_COLS} FROM shortcut_stories \
                     WHERE external_id = ?1"
                ),
                [story.external_id],
                |row| ShortcutStory::try_from(row),
            )
            .map_err(RepoError::from)
    }

    fn find_shortcut_story_by_external_id(
        &self,
        external_id: i64,
    ) -> RepoResult<Option<ShortcutStory>> {
        self.conn
            .query_row(
                &format!(
                    "SELECT {SHORTCUT_STORY_COLS} FROM shortcut_stories \
                     WHERE external_id = ?1"
                ),
                [external_id],
                |row| ShortcutStory::try_from(row),
            )
            .optional()
            .map_err(RepoError::from)
    }

    fn find_shortcut_story_by_row_id(&self, id: i64) -> RepoResult<Option<ShortcutStory>> {
        self.conn
            .query_row(
                &format!("SELECT {SHORTCUT_STORY_COLS} FROM shortcut_stories WHERE id = ?1"),
                [id],
                |row| ShortcutStory::try_from(row),
            )
            .optional()
            .map_err(RepoError::from)
    }

    fn link_task_to_story(
        &mut self,
        task_id: i64,
        story_row_id: i64,
        at: DateTime<Utc>,
    ) -> RepoResult<Task> {
        let updated = self.conn.execute(
            "UPDATE tasks SET shortcut_story_id = ?1, updated_at = ?2 \
             WHERE id = ?3",
            params![story_row_id, at, task_id],
        )?;
        if updated == 0 {
            return Err(RepoError::TaskNotFound(task_id));
        }
        load_task(&self.conn, task_id)
    }

    fn find_task_by_story_external_id(&self, external_id: i64) -> RepoResult<Option<Task>> {
        self.conn
            .query_row(
                &format!(
                    "SELECT {TASK_COLS} FROM tasks \
                     WHERE shortcut_story_id = \
                         (SELECT id FROM shortcut_stories WHERE external_id = ?1) \
                     ORDER BY created_at DESC LIMIT 1"
                ),
                [external_id],
                |row| Task::try_from(row),
            )
            .optional()
            .map_err(RepoError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> SqliteRepo {
        SqliteRepo::in_memory()
    }

    #[test]
    fn create_and_find_task() {
        let mut r = repo();
        let t = r.create_task("fix login", Some("oauth glitch")).unwrap();
        assert_eq!(t.title, "fix login");
        assert_eq!(t.description.as_deref(), Some("oauth glitch"));
        assert!(t.is_open());
        assert_eq!(r.find_task(t.id).unwrap().unwrap().id, t.id);
    }

    #[test]
    fn list_open_excludes_completed_and_archived() {
        let mut r = repo();
        let a = r.create_task("open", None).unwrap();
        let b = r.create_task("done", None).unwrap();
        let c = r.create_task("arch", None).unwrap();
        r.mark_task_done(b.id, Utc::now()).unwrap();
        r.archive_task(c.id, Utc::now()).unwrap();
        let open: Vec<i64> = r
            .list_open_tasks()
            .unwrap()
            .into_iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(open, vec![a.id]);
    }

    #[test]
    fn delete_task_blocked_when_entries_exist() {
        let mut r = repo();
        let t = r.create_task("t", None).unwrap();
        r.create_time_entry(t.id, Utc::now()).unwrap();
        match r.delete_task(t.id) {
            Err(RepoError::TaskHasEntries(id)) => assert_eq!(id, t.id),
            other => panic!("expected TaskHasEntries, got {other:?}"),
        }
    }

    #[test]
    fn delete_task_succeeds_when_no_entries() {
        let mut r = repo();
        let t = r.create_task("t", None).unwrap();
        r.delete_task(t.id).unwrap();
        assert!(r.find_task(t.id).unwrap().is_none());
    }

    #[test]
    fn create_entry_sets_active_and_end_clears_it() {
        let mut r = repo();
        let t = r.create_task("t", None).unwrap();
        let e = r.create_time_entry(t.id, Utc::now()).unwrap();
        assert!(r.active_time_entry().unwrap().is_some());
        r.end_time_entry(e.id, Utc::now()).unwrap();
        assert!(r.active_time_entry().unwrap().is_none());
    }

    #[test]
    fn task_total_duration_sums_entries() {
        use chrono::TimeZone;
        let mut r = repo();
        let t = r.create_task("t", None).unwrap();
        let a = r
            .create_time_entry(t.id, Utc.with_ymd_and_hms(2026, 4, 22, 9, 0, 0).unwrap())
            .unwrap();
        r.end_time_entry(a.id, Utc.with_ymd_and_hms(2026, 4, 22, 9, 30, 0).unwrap())
            .unwrap();
        let b = r
            .create_time_entry(t.id, Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap())
            .unwrap();
        r.end_time_entry(b.id, Utc.with_ymd_and_hms(2026, 4, 22, 10, 45, 0).unwrap())
            .unwrap();
        let total = r.task_total_duration(t.id, Utc::now()).unwrap();
        assert_eq!(total, Duration::minutes(30 + 45));
    }

    #[test]
    fn upsert_shortcut_story_inserts_then_updates() {
        use crate::shortcut::Story;
        let mut r = repo();
        let s1 = Story {
            external_id: 42,
            title: Some("first".into()),
            epic_id: None,
            epic_name: None,
            state: None,
        };
        let now1 = Utc::now();
        let row1 = r.upsert_shortcut_story(&s1, now1).unwrap();
        assert_eq!(row1.external_id, 42);
        assert_eq!(row1.title.as_deref(), Some("first"));

        let s2 = Story {
            external_id: 42,
            title: Some("second".into()),
            epic_id: None,
            epic_name: Some("Epic X".into()),
            state: Some("backlog".into()),
        };
        let now2 = Utc::now();
        let row2 = r.upsert_shortcut_story(&s2, now2).unwrap();
        assert_eq!(row2.id, row1.id, "upsert must reuse the same PK");
        assert_eq!(row2.title.as_deref(), Some("second"));
        assert_eq!(row2.epic_name.as_deref(), Some("Epic X"));
    }

    #[test]
    fn find_shortcut_story_by_external_id_returns_none_when_absent() {
        let r = repo();
        assert!(r.find_shortcut_story_by_external_id(999).unwrap().is_none());
    }

    #[test]
    fn link_task_to_story_sets_shortcut_story_id() {
        use crate::shortcut::Story;
        let mut r = repo();
        let t = r.create_task("t", None).unwrap();
        let row = r
            .upsert_shortcut_story(
                &Story {
                    external_id: 7,
                    title: Some("story".into()),
                    epic_id: None,
                    epic_name: None,
                    state: None,
                },
                Utc::now(),
            )
            .unwrap();
        let linked = r.link_task_to_story(t.id, row.id, Utc::now()).unwrap();
        assert_eq!(linked.shortcut_story_id, Some(row.id));
    }

    #[test]
    fn link_task_to_story_errors_on_missing_task() {
        use crate::shortcut::Story;
        let mut r = repo();
        let row = r
            .upsert_shortcut_story(
                &Story {
                    external_id: 1,
                    title: None,
                    epic_id: None,
                    epic_name: None,
                    state: None,
                },
                Utc::now(),
            )
            .unwrap();
        match r.link_task_to_story(999, row.id, Utc::now()) {
            Err(RepoError::TaskNotFound(id)) => assert_eq!(id, 999),
            other => panic!("expected TaskNotFound, got {other:?}"),
        }
    }

    #[test]
    fn find_task_by_story_external_id_finds_linked_task() {
        use crate::shortcut::Story;
        let mut r = repo();
        let row = r
            .upsert_shortcut_story(
                &Story {
                    external_id: 88,
                    title: Some("s".into()),
                    epic_id: None,
                    epic_name: None,
                    state: None,
                },
                Utc::now(),
            )
            .unwrap();
        let t = r.create_task("t", None).unwrap();
        r.link_task_to_story(t.id, row.id, Utc::now()).unwrap();
        let found = r.find_task_by_story_external_id(88).unwrap().unwrap();
        assert_eq!(found.id, t.id);
        assert_eq!(found.shortcut_story_id, Some(row.id));
    }

    #[test]
    fn find_task_by_story_external_id_none_when_story_absent() {
        let r = repo();
        assert!(r.find_task_by_story_external_id(404).unwrap().is_none());
    }

    #[test]
    fn find_shortcut_story_by_row_id_roundtrips() {
        use crate::shortcut::Story;
        let mut r = repo();
        let row = r
            .upsert_shortcut_story(
                &Story {
                    external_id: 77,
                    title: Some("x".into()),
                    epic_id: None,
                    epic_name: None,
                    state: None,
                },
                Utc::now(),
            )
            .unwrap();
        let found = r.find_shortcut_story_by_row_id(row.id).unwrap().unwrap();
        assert_eq!(found.external_id, 77);
        assert!(r.find_shortcut_story_by_row_id(9999).unwrap().is_none());
    }

    #[test]
    fn list_entries_in_range_includes_overlapping_entries() {
        use chrono::TimeZone;
        let mut r = repo();
        let t = r.create_task("t", None).unwrap();
        // Closed entry 09:00–09:30
        let a = r
            .create_time_entry(t.id, Utc.with_ymd_and_hms(2026, 4, 22, 9, 0, 0).unwrap())
            .unwrap();
        r.end_time_entry(a.id, Utc.with_ymd_and_hms(2026, 4, 22, 9, 30, 0).unwrap())
            .unwrap();
        // Closed entry 10:00–11:00, fully inside the day
        let b = r
            .create_time_entry(t.id, Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap())
            .unwrap();
        r.end_time_entry(b.id, Utc.with_ymd_and_hms(2026, 4, 22, 11, 0, 0).unwrap())
            .unwrap();
        // Closed entry on the next day, must not appear in the 22nd window
        let c = r
            .create_time_entry(t.id, Utc.with_ymd_and_hms(2026, 4, 23, 9, 0, 0).unwrap())
            .unwrap();
        r.end_time_entry(c.id, Utc.with_ymd_and_hms(2026, 4, 23, 9, 15, 0).unwrap())
            .unwrap();

        let from = Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2026, 4, 23, 0, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 4, 23, 12, 0, 0).unwrap();
        let entries = r.list_entries_in_range(from, to, now).unwrap();
        let ids: Vec<i64> = entries.iter().map(|e| e.id).collect();
        assert_eq!(ids, vec![a.id, b.id]);
    }

    #[test]
    fn list_entries_in_range_includes_active_entry_started_before_range_end() {
        use chrono::TimeZone;
        let mut r = repo();
        let t = r.create_task("t", None).unwrap();
        // Active entry started inside the window.
        let active = r
            .create_time_entry(t.id, Utc.with_ymd_and_hms(2026, 4, 22, 14, 0, 0).unwrap())
            .unwrap();

        let from = Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2026, 4, 23, 0, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 4, 22, 16, 0, 0).unwrap();
        let entries = r.list_entries_in_range(from, to, now).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, active.id);
        assert!(entries[0].is_active());
    }

    #[test]
    fn update_time_entry_changes_started_ended_notes() {
        use chrono::TimeZone;
        let mut r = repo();
        let t = r.create_task("t", None).unwrap();
        let e = r
            .create_time_entry(t.id, Utc.with_ymd_and_hms(2026, 4, 22, 9, 0, 0).unwrap())
            .unwrap();
        r.end_time_entry(e.id, Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap())
            .unwrap();

        let new_start = Utc.with_ymd_and_hms(2026, 4, 22, 9, 15, 0).unwrap();
        let new_end = Utc.with_ymd_and_hms(2026, 4, 22, 11, 0, 0).unwrap();
        let updated = r
            .update_time_entry(e.id, new_start, Some(new_end), Some("forgot to start"))
            .unwrap();
        assert_eq!(updated.started_at, new_start);
        assert_eq!(updated.ended_at, Some(new_end));
        assert_eq!(updated.notes.as_deref(), Some("forgot to start"));
    }

    #[test]
    fn update_time_entry_returns_not_found_for_missing_id() {
        match repo().update_time_entry(9999, Utc::now(), None, None) {
            Err(RepoError::TimeEntryNotFound(id)) => assert_eq!(id, 9999),
            other => panic!("expected TimeEntryNotFound, got {other:?}"),
        }
    }

    #[test]
    fn list_entries_in_range_excludes_entries_ending_before_range() {
        use chrono::TimeZone;
        let mut r = repo();
        let t = r.create_task("t", None).unwrap();
        let earlier = r
            .create_time_entry(t.id, Utc.with_ymd_and_hms(2026, 4, 21, 9, 0, 0).unwrap())
            .unwrap();
        r.end_time_entry(
            earlier.id,
            Utc.with_ymd_and_hms(2026, 4, 21, 10, 0, 0).unwrap(),
        )
        .unwrap();
        let from = Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2026, 4, 23, 0, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 4, 22, 12, 0, 0).unwrap();
        let entries = r.list_entries_in_range(from, to, now).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn active_snapshot_returns_none_when_idle() {
        let r = repo();
        assert!(r.active_snapshot().unwrap().is_none());
    }

    #[test]
    fn active_snapshot_returns_task_and_started_at_for_active_entry() {
        use chrono::TimeZone;
        let mut r = repo();
        let t = r.create_task("fix login", None).unwrap();
        let started = Utc.with_ymd_and_hms(2026, 4, 22, 9, 15, 0).unwrap();
        r.create_time_entry(t.id, started).unwrap();

        let snap = r.active_snapshot().unwrap().expect("active snapshot");
        assert_eq!(snap.task_id, t.id);
        assert_eq!(snap.task_title, "fix login");
        assert_eq!(snap.sc_external_id, None);
        assert_eq!(snap.started_at, started);
    }

    #[test]
    fn active_snapshot_includes_shortcut_external_id_when_linked() {
        use crate::shortcut::Story;
        use chrono::TimeZone;
        let mut r = repo();
        let row = r
            .upsert_shortcut_story(
                &Story {
                    external_id: 4242,
                    title: Some("SC story".into()),
                    epic_id: None,
                    epic_name: None,
                    state: None,
                },
                Utc::now(),
            )
            .unwrap();
        let t = r.create_task("linked", None).unwrap();
        r.link_task_to_story(t.id, row.id, Utc::now()).unwrap();
        let started = Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap();
        r.create_time_entry(t.id, started).unwrap();

        let snap = r.active_snapshot().unwrap().expect("active");
        assert_eq!(snap.sc_external_id, Some(4242));
        assert_eq!(snap.task_title, "linked");
    }

    #[test]
    fn open_read_only_rejects_writes() {
        use crate::storage::open;
        use rusqlite::ErrorCode;
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("buckland.db");
        // Create the schema with a writable open first so the DB exists.
        let _ = open(&path).unwrap();

        let ro = SqliteRepo::open_read_only(&path).expect("open ro");
        let res = ro.connection().execute(
            "INSERT INTO tasks (title, created_at, updated_at) VALUES ('x', ?1, ?1)",
            rusqlite::params![Utc::now()],
        );
        let err = res.expect_err("read-only must reject writes");
        let rusqlite::Error::SqliteFailure(e, _) = err else {
            panic!("expected SqliteFailure, got {err:?}");
        };
        assert_eq!(e.code, ErrorCode::ReadOnly);
    }

    #[test]
    fn open_read_only_returns_database_missing_when_file_absent() {
        use crate::storage::repo::RepoError;
        use std::path::Path;
        let res = SqliteRepo::open_read_only(Path::new("/tmp/buckland-does-not-exist-zzz.db"));
        match res {
            Err(RepoError::DatabaseMissing(_)) => {}
            other => panic!("expected DatabaseMissing, got {other:?}"),
        }
    }
}
