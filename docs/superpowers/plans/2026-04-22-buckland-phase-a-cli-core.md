# Buckland — Phase A: CLI core

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` (inline) or `superpowers:subagent-driven-development` (fresh subagent per task). Steps use checkbox (`- [ ]`) syntax for tracking. Before starting, read the spec and `CLAUDE.md` referenced in **Required reading** below.

## What this phase delivers

A working personal time-tracker as a pure CLI. After Phase A is green, these commands work end-to-end against a local SQLite file:

```
bl add <title> [--description <text>]
bl list [--all | --archived | --completed]
bl start <id | "text">
bl stop          (alias: bl pause)
bl status
bl done <id>
bl archive <id>
bl delete <id>
```

Phase A intentionally does **not** include:
- Shortcut integration (`--sc`, `bl shortcut`) — Phase B.
- `bl report` — Phase C.
- TUI (`bl tui`) — Phase D.
- Tray (`bl-tray`) and clipboard integration — Phase E.
- CI, `.deb`, release docs — Phase F.

The `bl start <SC-ID>` resolution path and anything else Shortcut-related is **out of scope** here. In this phase, `bl start <numeric>` is task-id only; non-numeric text creates a new task and starts it.

## Required reading (load before executing the first task)

1. `CLAUDE.md` — project-wide development guidelines (TDD, Rust idioms, size budgets, testing approach).
2. `docs/superpowers/specs/2026-04-22-buckland-design.md` — the design spec. Sections especially relevant here: "Architecture", "Data Model", "Command Grammar (CLI)", "Testing".
3. `docs/superpowers/plans/README.md` — phase order and what has shipped.

## Preconditions

- Running in the `buckland` repository root (`/home/nuuvem/Projects/study/buckland`).
- `git status` is clean.
- `cargo` and `rustc` (>= 1.74) installed on PATH.
- The repo currently has **no Rust source** — this phase bootstraps it. The pre-existing files are docs only (`CLAUDE.md`, `docs/`, `.claude/`, `.gitignore`).

## Postconditions (how to verify Phase A is done)

- `cargo test` passes cleanly (unit + integration).
- `cargo clippy --all-targets -- -D warnings` is clean.
- `cargo fmt --all --check` is clean.
- Running the full CLI smoke flow works:
  ```bash
  cargo run -- add "try buckland"
  cargo run -- list
  cargo run -- start 1
  cargo run -- status        # exit code 0
  cargo run -- stop
  cargo run -- status        # exit code 1
  cargo run -- done 1
  cargo run -- list          # empty
  cargo run -- list --completed
  ```
- `git log --oneline` shows one commit per task below.

## Architecture (in scope for this phase)

One crate (`buckland`). Binary `bl` is the only surface produced here (the `bl-tray` binary file is created as a placeholder but unused). Layers:

- **Domain** (`src/domain/`) — plain structs + pure helpers (Task, TimeEntry, ShortcutStory).
- **Storage** (`src/storage/`) — `rusqlite` migrations + `Repo` trait + `SqliteRepo` impl.
- **TimerOps** (`src/domain/timer_ops.rs`) — the single-active-timer invariant enforced in a transaction.
- **Config** (`src/config.rs`) — XDG paths + TOML load/save.
- **CLI** (`src/cli/`) — `clap` derive structs + a dispatch module that wires commands to domain + storage.

SQLite file path: `$XDG_DATA_HOME/buckland/buckland.db` (fall back to `~/.local/share/buckland/buckland.db`). Tests use `Connection::open_in_memory()`.

## Tech stack (this phase)

- `clap` v4 (derive)
- `rusqlite` + `rusqlite_migration`
- `chrono` (clock + serde, no default tz features)
- `serde` + `toml`
- `dirs` (XDG paths)
- `thiserror` (domain errors) + `anyhow` (binary errors)
- Test deps: `tempfile`, `assert_cmd`, `predicates`

## Next phase

When Phase A is merged/green, proceed to `2026-04-22-buckland-phase-b-shortcut.md` (still to draft — see `docs/superpowers/plans/README.md`).

---

## Task 1: Bootstrap the Rust crate

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/main.rs`
- Create: `LICENSE`
- Modify: `.gitignore`

- [ ] **Step 1: Initialize Cargo crate**

Run: `cd /home/nuuvem/Projects/study/buckland && cargo init --name buckland --edition 2021`
Expected: `Created binary (application) package`. Creates `Cargo.toml`, `src/main.rs`.

- [ ] **Step 2: Replace Cargo.toml with the minimal manifest**

```toml
[package]
name = "buckland"
version = "0.1.0"
edition = "2021"
rust-version = "1.74"
license = "GPL-3.0-or-later"
description = "TUI-first time tracker for developers who use Shortcut."
repository = "https://github.com/vagnerzampieri/buckland"
categories = ["command-line-utilities"]
keywords = ["tui", "time-tracking", "shortcut", "productivity"]

[lib]
name = "buckland"
path = "src/lib.rs"

[[bin]]
name = "bl"
path = "src/main.rs"

[[bin]]
name = "bl-tray"
path = "src/bin/bl-tray.rs"
required-features = ["tray"]

[features]
default = ["tray"]
tray = []

[dependencies]
anyhow = "1"
thiserror = "2"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Create empty `src/lib.rs`**

```rust
//! Buckland — personal time tracker core library.
```

- [ ] **Step 4: Replace `src/main.rs` with a placeholder**

```rust
fn main() {
    println!("buckland v{}", env!("CARGO_PKG_VERSION"));
}
```

- [ ] **Step 5: Create empty `src/bin/bl-tray.rs`**

```rust
fn main() {
    println!("bl-tray v{} (placeholder)", env!("CARGO_PKG_VERSION"));
}
```

- [ ] **Step 6: Add GPL-3.0 license text**

Run: `curl -sfLo LICENSE https://www.gnu.org/licenses/gpl-3.0.txt`
Expected: `LICENSE` file written (~35KB).

- [ ] **Step 7: Update `.gitignore` for Rust**

Append to `.gitignore`:
```
/target
**/*.rs.bk
Cargo.lock.bak
```

(Keep existing entries; do not remove.)

- [ ] **Step 8: Verify build**

Run: `cargo build`
Expected: `Compiling buckland v0.1.0 ... Finished` without errors.

- [ ] **Step 9: Verify test harness**

Run: `cargo test`
Expected: `running 0 tests ... test result: ok. 0 passed; 0 failed`.

- [ ] **Step 10: Commit**

```bash
git add Cargo.toml Cargo.lock LICENSE .gitignore src/
git commit -m "feat: bootstrap Rust crate with bl and bl-tray binaries"
```

---

## Task 2: Database migrations

**Files:**
- Create: `migrations/0001_init.sql`
- Create: `src/storage/mod.rs`
- Create: `src/storage/migrate.rs`
- Modify: `src/lib.rs`
- Modify: `Cargo.toml` (add rusqlite + rusqlite_migration)
- Test: `tests/migrations.rs`

- [ ] **Step 1: Add SQLite dependencies**

Replace the `[dependencies]` block in `Cargo.toml` with:

```toml
[dependencies]
anyhow = "1"
thiserror = "2"
rusqlite = { version = "0.32", features = ["bundled", "chrono"] }
rusqlite_migration = "1"
chrono = { version = "0.4", default-features = false, features = ["clock", "serde"] }
```

- [ ] **Step 2: Write the initial schema migration**

Create `migrations/0001_init.sql`:

```sql
CREATE TABLE tasks (
  id                INTEGER PRIMARY KEY,
  title             TEXT NOT NULL,
  description       TEXT,
  shortcut_story_id INTEGER REFERENCES shortcut_stories(id),
  completed_at      DATETIME,
  archived_at       DATETIME,
  created_at        DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at        DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE time_entries (
  id          INTEGER PRIMARY KEY,
  task_id     INTEGER NOT NULL REFERENCES tasks(id) ON DELETE RESTRICT,
  started_at  DATETIME NOT NULL,
  ended_at    DATETIME,
  notes       TEXT,
  created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX idx_single_active
  ON time_entries(started_at)
  WHERE ended_at IS NULL;

CREATE TABLE shortcut_stories (
  id          INTEGER PRIMARY KEY,
  external_id INTEGER NOT NULL UNIQUE,
  title       TEXT,
  epic_name   TEXT,
  state       TEXT,
  fetched_at  DATETIME NOT NULL
);

CREATE INDEX idx_tasks_open
  ON tasks(created_at)
  WHERE completed_at IS NULL AND archived_at IS NULL;

CREATE INDEX idx_time_entries_task_started
  ON time_entries(task_id, started_at);
```

- [ ] **Step 3: Create storage module skeleton**

Create `src/storage/mod.rs`:

```rust
//! SQLite storage layer.

pub mod migrate;

use rusqlite::Connection;
use std::path::Path;

/// Open a SQLite connection at `path`, apply migrations, and return it.
pub fn open(path: &Path) -> anyhow::Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate::run(&mut conn)?;
    Ok(conn)
}

/// Open an in-memory SQLite connection with migrations applied. Used in tests.
pub fn open_memory() -> anyhow::Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate::run(&mut conn)?;
    Ok(conn)
}
```

- [ ] **Step 4: Create migration loader**

Create `src/storage/migrate.rs`:

```rust
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(include_str!("../../migrations/0001_init.sql"))])
}

pub fn run(conn: &mut Connection) -> anyhow::Result<()> {
    migrations().to_latest(conn)?;
    Ok(())
}
```

- [ ] **Step 5: Expose storage from the library**

Replace `src/lib.rs`:

```rust
//! Buckland — personal time tracker core library.

pub mod storage;
```

- [ ] **Step 6: Write the failing integration test**

Create `tests/migrations.rs`:

```rust
use buckland::storage;

#[test]
fn migrations_create_expected_tables_and_indexes() {
    let conn = storage::open_memory().expect("open in-memory DB with migrations");

    // All three tables exist.
    let table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type='table' AND name IN ('tasks', 'time_entries', 'shortcut_stories')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(table_count, 3);

    // Partial unique index on active timer exists.
    let has_active_idx: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type='index' AND name='idx_single_active'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(has_active_idx, 1);
}
```

- [ ] **Step 7: Run test to verify it fails**

Run: `cargo test --test migrations`
Expected: FAIL — `migrations` module or `open_memory` not yet wired up. (If the earlier steps compiled, it should PASS; run it to confirm.)

- [ ] **Step 8: Add a second test: partial index enforces single-active invariant**

Append to `tests/migrations.rs`:

```rust
#[test]
fn partial_unique_index_rejects_two_active_entries() {
    let conn = storage::open_memory().unwrap();

    conn.execute(
        "INSERT INTO tasks (title, created_at, updated_at) \
         VALUES ('t', '2026-04-22 10:00:00', '2026-04-22 10:00:00')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO time_entries (task_id, started_at) VALUES (1, '2026-04-22 10:00:00')",
        [],
    )
    .unwrap();

    let second = conn.execute(
        "INSERT INTO time_entries (task_id, started_at) VALUES (1, '2026-04-22 10:01:00')",
        [],
    );

    assert!(second.is_err(), "expected UNIQUE constraint failure, got {:?}", second);
}
```

- [ ] **Step 9: Run both tests**

Run: `cargo test --test migrations`
Expected: Both tests PASS.

- [ ] **Step 10: Commit**

```bash
git add Cargo.toml Cargo.lock migrations/ src/ tests/
git commit -m "feat(storage): initial schema with single-active-timer invariant"
```

---

## Task 3: Domain types (Task, TimeEntry, ShortcutStory)

**Files:**
- Create: `src/domain/mod.rs`
- Create: `src/domain/task.rs`
- Create: `src/domain/time_entry.rs`
- Create: `src/domain/shortcut_story.rs`
- Modify: `src/lib.rs`
- Test: inline in each domain file.

- [ ] **Step 1: Create the domain module**

Create `src/domain/mod.rs`:

```rust
//! Domain types: Task, TimeEntry, ShortcutStory.
//!
//! These are plain data structures with minimal behavior. Persistence is
//! the storage layer's concern; domain operations live in sibling modules
//! like `timer_ops`.

pub mod task;
pub mod time_entry;
pub mod shortcut_story;

pub use task::Task;
pub use time_entry::TimeEntry;
pub use shortcut_story::ShortcutStory;
```

- [ ] **Step 2: Add Task with TryFrom<&Row>**

Create `src/domain/task.rs`:

```rust
use chrono::{DateTime, Utc};
use rusqlite::Row;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub shortcut_story_id: Option<i64>,
    pub completed_at: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn is_open(&self) -> bool {
        self.completed_at.is_none() && self.archived_at.is_none()
    }
}

impl<'a> TryFrom<&Row<'a>> for Task {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'a>) -> Result<Self, Self::Error> {
        Ok(Task {
            id: row.get("id")?,
            title: row.get("title")?,
            description: row.get("description")?,
            shortcut_story_id: row.get("shortcut_story_id")?,
            completed_at: row.get("completed_at")?,
            archived_at: row.get("archived_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sample() -> Task {
        Task {
            id: 1,
            title: "t".into(),
            description: None,
            shortcut_story_id: None,
            completed_at: None,
            archived_at: None,
            created_at: Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap(),
            updated_at: Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap(),
        }
    }

    #[test]
    fn is_open_when_neither_completed_nor_archived() {
        assert!(sample().is_open());
    }

    #[test]
    fn is_not_open_when_completed() {
        let mut t = sample();
        t.completed_at = Some(Utc::now());
        assert!(!t.is_open());
    }

    #[test]
    fn is_not_open_when_archived() {
        let mut t = sample();
        t.archived_at = Some(Utc::now());
        assert!(!t.is_open());
    }
}
```

- [ ] **Step 3: Add TimeEntry**

Create `src/domain/time_entry.rs`:

```rust
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
        if d < Duration::zero() { Duration::zero() } else { d }
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
            id: 1, task_id: 1,
            started_at: start_at(10, 0), ended_at: None,
            notes: None, created_at: start_at(10, 0),
        };
        assert_eq!(e.duration(start_at(10, 30)), Duration::minutes(30));
    }

    #[test]
    fn closed_entry_uses_ended_at() {
        let e = TimeEntry {
            id: 1, task_id: 1,
            started_at: start_at(10, 0), ended_at: Some(start_at(11, 23)),
            notes: None, created_at: start_at(10, 0),
        };
        assert_eq!(e.duration(start_at(23, 0)), Duration::minutes(83));
    }

    #[test]
    fn negative_duration_clamps_to_zero() {
        let e = TimeEntry {
            id: 1, task_id: 1,
            started_at: start_at(12, 0), ended_at: None,
            notes: None, created_at: start_at(12, 0),
        };
        assert_eq!(e.duration(start_at(10, 0)), Duration::zero());
    }
}
```

- [ ] **Step 4: Add ShortcutStory**

Create `src/domain/shortcut_story.rs`:

```rust
use chrono::{DateTime, Duration, Utc};
use rusqlite::Row;

const CACHE_TTL: Duration = Duration::hours(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutStory {
    pub id: i64,
    pub external_id: i64,
    pub title: Option<String>,
    pub epic_name: Option<String>,
    pub state: Option<String>,
    pub fetched_at: DateTime<Utc>,
}

impl ShortcutStory {
    pub fn is_fresh(&self, now: DateTime<Utc>) -> bool {
        now - self.fetched_at < CACHE_TTL
    }
}

impl<'a> TryFrom<&Row<'a>> for ShortcutStory {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'a>) -> Result<Self, Self::Error> {
        Ok(ShortcutStory {
            id: row.get("id")?,
            external_id: row.get("external_id")?,
            title: row.get("title")?,
            epic_name: row.get("epic_name")?,
            state: row.get("state")?,
            fetched_at: row.get("fetched_at")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 22, h, 0, 0).unwrap()
    }

    #[test]
    fn fresh_within_ttl() {
        let s = ShortcutStory {
            id: 1, external_id: 123, title: None, epic_name: None, state: None,
            fetched_at: at(10),
        };
        assert!(s.is_fresh(at(10) + Duration::minutes(59)));
    }

    #[test]
    fn stale_past_ttl() {
        let s = ShortcutStory {
            id: 1, external_id: 123, title: None, epic_name: None, state: None,
            fetched_at: at(10),
        };
        assert!(!s.is_fresh(at(10) + Duration::minutes(61)));
    }
}
```

- [ ] **Step 5: Wire domain into lib.rs**

Replace `src/lib.rs`:

```rust
//! Buckland — personal time tracker core library.

pub mod domain;
pub mod storage;
```

- [ ] **Step 6: Run tests**

Run: `cargo test`
Expected: All unit tests pass (8 tests from domain + 2 from migrations).

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock src/
git commit -m "feat(domain): Task, TimeEntry, ShortcutStory with TryFrom<&Row>"
```

---

## Task 4: Repo trait + SqliteRepo implementation

**Files:**
- Create: `src/storage/repo.rs`
- Modify: `src/storage/mod.rs`
- Test: inline in `src/storage/repo.rs`.

- [ ] **Step 1: Declare the `Repo` trait**

Create `src/storage/repo.rs`:

```rust
//! The storage contract consumed by the domain layer.
//!
//! `Repo` abstracts the concrete database. `SqliteRepo` is the production
//! implementation. Tests can build on an in-memory SqliteRepo via
//! `SqliteRepo::in_memory()`.

use crate::domain::{Task, TimeEntry};
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

    fn create_time_entry(&mut self, task_id: i64, started_at: DateTime<Utc>) -> RepoResult<TimeEntry>;
    fn end_time_entry(&mut self, id: i64, ended_at: DateTime<Utc>) -> RepoResult<TimeEntry>;
    fn active_time_entry(&self) -> RepoResult<Option<TimeEntry>>;
    fn list_entries_for_task(&self, task_id: i64) -> RepoResult<Vec<TimeEntry>>;
    fn task_total_duration(&self, task_id: i64, now: DateTime<Utc>) -> RepoResult<Duration>;
    fn delete_time_entry(&mut self, id: i64) -> RepoResult<()>;
}
```

- [ ] **Step 2: Implement `SqliteRepo`**

Append to `src/storage/repo.rs`:

```rust
pub struct SqliteRepo {
    conn: Connection,
}

impl SqliteRepo {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    #[cfg(any(test, feature = "test-support"))]
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
    let sql = format!("SELECT {TASK_COLS} FROM tasks WHERE {where_clause} ORDER BY created_at DESC");
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
            "UPDATE tasks SET completed_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![at, id],
        )?;
        if updated == 0 {
            return Err(RepoError::TaskNotFound(id));
        }
        load_task(&self.conn, id)
    }

    fn archive_task(&mut self, id: i64, at: DateTime<Utc>) -> RepoResult<Task> {
        let updated = self.conn.execute(
            "UPDATE tasks SET archived_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![at, id],
        )?;
        if updated == 0 {
            return Err(RepoError::TaskNotFound(id));
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
        let removed = self
            .conn
            .execute("DELETE FROM tasks WHERE id = ?1", [id])?;
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
                "SELECT id, task_id, started_at, ended_at, notes, created_at \
                 FROM time_entries WHERE id = ?1",
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
                "SELECT id, task_id, started_at, ended_at, notes, created_at \
                 FROM time_entries WHERE id = ?1",
                [id],
                |row| TimeEntry::try_from(row),
            )
            .map_err(RepoError::from)
    }

    fn active_time_entry(&self) -> RepoResult<Option<TimeEntry>> {
        self.conn
            .query_row(
                "SELECT id, task_id, started_at, ended_at, notes, created_at \
                 FROM time_entries WHERE ended_at IS NULL LIMIT 1",
                [],
                |row| TimeEntry::try_from(row),
            )
            .optional()
            .map_err(RepoError::from)
    }

    fn list_entries_for_task(&self, task_id: i64) -> RepoResult<Vec<TimeEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, started_at, ended_at, notes, created_at \
             FROM time_entries WHERE task_id = ?1 ORDER BY started_at DESC",
        )?;
        let rows = stmt.query_map([task_id], |row| TimeEntry::try_from(row))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    fn task_total_duration(
        &self,
        task_id: i64,
        now: DateTime<Utc>,
    ) -> RepoResult<Duration> {
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
}
```

- [ ] **Step 3: Add tests inline**

Append to `src/storage/repo.rs`:

```rust
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
        let open: Vec<i64> = r.list_open_tasks().unwrap().into_iter().map(|t| t.id).collect();
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
        let a = r.create_time_entry(t.id, Utc.with_ymd_and_hms(2026, 4, 22, 9, 0, 0).unwrap()).unwrap();
        r.end_time_entry(a.id, Utc.with_ymd_and_hms(2026, 4, 22, 9, 30, 0).unwrap()).unwrap();
        let b = r.create_time_entry(t.id, Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap()).unwrap();
        r.end_time_entry(b.id, Utc.with_ymd_and_hms(2026, 4, 22, 10, 45, 0).unwrap()).unwrap();
        let total = r.task_total_duration(t.id, Utc::now()).unwrap();
        assert_eq!(total, Duration::minutes(30 + 45));
    }
}
```

- [ ] **Step 4: Re-export from `src/storage/mod.rs`**

Replace `src/storage/mod.rs`:

```rust
//! SQLite storage layer.

pub mod migrate;
pub mod repo;

pub use repo::{Repo, RepoError, RepoResult, SqliteRepo};

use rusqlite::Connection;
use std::path::Path;

pub fn open(path: &Path) -> anyhow::Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate::run(&mut conn)?;
    Ok(conn)
}

pub fn open_memory() -> anyhow::Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate::run(&mut conn)?;
    Ok(conn)
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test`
Expected: 16 tests pass (previous 10 + 6 new repo tests).

- [ ] **Step 6: Commit**

```bash
git add src/
git commit -m "feat(storage): Repo trait with SqliteRepo implementation"
```

---

## Task 5: TimerOps — single-active-timer invariant in a transaction

**Files:**
- Create: `src/domain/timer_ops.rs`
- Modify: `src/domain/mod.rs`

TimerOps owns the rule "starting a timer atomically stops any active one." It uses `Repo` and a transaction to make the stop+start indivisible.

- [ ] **Step 1: Write `TimerOps::start` and `stop`**

Create `src/domain/timer_ops.rs`:

```rust
//! Timer operations: start, stop. Enforces the single-active-timer invariant
//! by opening a transaction, ending any active entry, then creating the new
//! one — all in one shot.

use crate::domain::TimeEntry;
use crate::storage::{RepoError, RepoResult, SqliteRepo};
use chrono::{DateTime, Utc};
use rusqlite::params;

pub struct TimerOps<'a> {
    repo: &'a mut SqliteRepo,
}

impl<'a> TimerOps<'a> {
    pub fn new(repo: &'a mut SqliteRepo) -> Self {
        Self { repo }
    }

    /// Start a timer for `task_id` at `now`. If any timer is active, end it
    /// first. Both actions share a transaction so there's never a moment
    /// with two active entries.
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
            "SELECT id, task_id, started_at, ended_at, notes, created_at \
             FROM time_entries WHERE id = ?1",
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
            .ok();
        let Some(id) = active_id else {
            tx.commit()?;
            return Ok(None);
        };
        tx.execute(
            "UPDATE time_entries SET ended_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        let entry = tx.query_row(
            "SELECT id, task_id, started_at, ended_at, notes, created_at \
             FROM time_entries WHERE id = ?1",
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
            let active_count: i64 = repo.connection().query_row(
                "SELECT COUNT(*) FROM time_entries WHERE ended_at IS NULL",
                [],
                |r| r.get(0),
            ).unwrap();
            assert_eq!(active_count, 1, "two active at hour {h}");
        }
    }

    // `RepoError` is unused here but the import keeps the file honest if we
    // later switch to typed error assertions.
    #[allow(dead_code)]
    fn _error_imported_for_future_use(_: RepoError) {}
}
```

- [ ] **Step 2: Wire TimerOps into the domain module**

Replace `src/domain/mod.rs`:

```rust
//! Domain types and operations.

pub mod task;
pub mod time_entry;
pub mod shortcut_story;
pub mod timer_ops;

pub use task::Task;
pub use time_entry::TimeEntry;
pub use shortcut_story::ShortcutStory;
pub use timer_ops::TimerOps;
```

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: 21 tests pass (16 previous + 5 new).

- [ ] **Step 4: Commit**

```bash
git add src/
git commit -m "feat(domain): TimerOps enforces single-active-timer in a transaction"
```

---

## Task 6: Config module (XDG paths + TOML)

**Files:**
- Create: `src/config.rs`
- Modify: `src/lib.rs`
- Modify: `Cargo.toml` (add `serde`, `toml`, `dirs`)

- [ ] **Step 1: Add dependencies**

Append to `Cargo.toml` under `[dependencies]`:

```toml
serde = { version = "1", features = ["derive"] }
toml = "0.8"
dirs = "5"
```

- [ ] **Step 2: Write `src/config.rs`**

Create `src/config.rs`:

```rust
//! Configuration: paths + `config.toml`.
//!
//! Paths follow the XDG Base Directory Specification:
//!   data dir  = $XDG_DATA_HOME/buckland   (default ~/.local/share/buckland)
//!   config    = $XDG_CONFIG_HOME/buckland/config.toml
//!
//! The file is optional. Missing file means "default config, no shortcut token."

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    #[serde(default)]
    pub shortcut: ShortcutConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub tray: TrayConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShortcutConfig {
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiConfig {
    #[serde(default = "default_icons")]
    pub icons: String,
    #[serde(default = "default_accent")]
    pub accent_color: String,
}

fn default_icons() -> String {
    "unicode".into()
}

fn default_accent() -> String {
    "cyan".into()
}

impl Default for UiConfig {
    fn default() -> Self {
        Self { icons: default_icons(), accent_color: default_accent() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrayConfig {
    #[serde(default = "default_poll")]
    pub poll_seconds: u64,
}

fn default_poll() -> u64 {
    30
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self { poll_seconds: default_poll() }
    }
}

/// Data file location (the SQLite database).
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("buckland")
}

pub fn db_path() -> PathBuf {
    data_dir().join("buckland.db")
}

/// Config file location (`config.toml`).
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("buckland")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// Load config from `path`. Missing file returns default.
pub fn load(path: &Path) -> anyhow::Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let text = fs::read_to_string(path)?;
    Ok(toml::from_str(&text)?)
}

/// Save config to `path`. Creates parent directories. Writes with mode 0600
/// on Unix so the token stays private.
pub fn save(path: &Path, cfg: &Config) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(cfg)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(text.as_bytes())?;
    }
    #[cfg(not(unix))]
    {
        fs::write(path, text)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_file_returns_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let cfg = load(&path).unwrap();
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut cfg = Config::default();
        cfg.shortcut.token = Some("abc123".into());
        cfg.tray.poll_seconds = 45;
        save(&path, &cfg).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded, cfg);
    }

    #[test]
    fn defaults_fill_missing_fields() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "[shortcut]\ntoken = \"xyz\"\n").unwrap();
        let cfg = load(&path).unwrap();
        assert_eq!(cfg.shortcut.token.as_deref(), Some("xyz"));
        assert_eq!(cfg.ui.icons, "unicode");
        assert_eq!(cfg.tray.poll_seconds, 30);
    }

    #[cfg(unix)]
    #[test]
    fn saved_file_has_user_only_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        save(&path, &Config::default()).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "expected 0600, got {mode:o}");
    }
}
```

- [ ] **Step 3: Re-export from `src/lib.rs`**

Replace `src/lib.rs`:

```rust
//! Buckland — personal time tracker core library.

pub mod config;
pub mod domain;
pub mod storage;
```

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: 25 tests pass (21 previous + 4 new config tests).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/
git commit -m "feat(config): TOML config with XDG paths, mode 0600 on save"
```

---

## Task 7: CLI skeleton — `bl` binary with clap dispatch

Wire up `bl` as the single CLI surface. This task defines the `clap` schema, the dispatch table, the DB path resolver (including `BUCKLAND_HOME` env override used by integration tests), and a helper to format durations. The subcommand handlers are stubs that return `todo!()`; Tasks 8–12 fill them in one by one.

**Files:**
- Create: `src/cli/mod.rs`
- Create: `src/cli/args.rs`
- Create: `src/cli/context.rs`
- Create: `src/cli/commands.rs`
- Create: `src/cli/format.rs`
- Modify: `src/main.rs`
- Modify: `src/lib.rs`
- Modify: `Cargo.toml` (add `clap`)

- [ ] **Step 1: Add clap**

Append to `Cargo.toml` under `[dependencies]`:

```toml
clap = { version = "4", features = ["derive"] }
```

Append to `[dev-dependencies]`:

```toml
assert_cmd = "2"
predicates = "3"
```

- [ ] **Step 2: Create `src/cli/args.rs`**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "bl", about = "Time tracker for developers who use Shortcut", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Add a new task.
    Add {
        title: String,
        #[arg(long)]
        description: Option<String>,
    },
    /// List tasks. Default: only open tasks.
    List {
        #[arg(long, conflicts_with_all = ["archived", "completed"])]
        all: bool,
        #[arg(long, conflicts_with = "completed")]
        archived: bool,
        #[arg(long)]
        completed: bool,
    },
    /// Start a timer. <target> can be a numeric task id or free-text title.
    Start {
        target: String,
    },
    /// Stop the active timer.
    Stop,
    /// Alias for stop.
    Pause,
    /// Print the active timer. Exit 0 if active, 1 if idle.
    Status,
    /// Mark a task as done.
    Done {
        id: i64,
    },
    /// Archive a task (hide it from default list).
    Archive {
        id: i64,
    },
    /// Hard-delete a task. Fails if the task has time entries.
    Delete {
        id: i64,
    },
}
```

- [ ] **Step 3: Create `src/cli/context.rs`**

Resolves paths and opens the DB. Respects `BUCKLAND_HOME` env override for tests.

```rust
use crate::config;
use crate::storage::SqliteRepo;
use std::path::PathBuf;

pub struct Context {
    pub repo: SqliteRepo,
    pub db_path: PathBuf,
}

pub fn open() -> anyhow::Result<Context> {
    let db_path = resolve_db_path();
    let conn = crate::storage::open(&db_path)?;
    Ok(Context {
        repo: SqliteRepo::new(conn),
        db_path,
    })
}

fn resolve_db_path() -> PathBuf {
    if let Ok(home) = std::env::var("BUCKLAND_HOME") {
        PathBuf::from(home).join("buckland.db")
    } else {
        config::db_path()
    }
}
```

- [ ] **Step 4: Create `src/cli/format.rs`**

```rust
use chrono::Duration;

/// Format as "1h 23m" or "12m" or "5s". Used in list rows and summaries.
pub fn duration_compact(d: Duration) -> String {
    let secs = d.num_seconds().max(0);
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}h {m:02}m")
    } else if m > 0 {
        format!("{m}m")
    } else {
        format!("{s}s")
    }
}

/// Format as "HH:MM:SS". Used for the active-timer header/status line.
pub fn duration_hms(d: Duration) -> String {
    let secs = d.num_seconds().max(0);
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_picks_the_right_unit() {
        assert_eq!(duration_compact(Duration::seconds(0)), "0s");
        assert_eq!(duration_compact(Duration::seconds(45)), "45s");
        assert_eq!(duration_compact(Duration::minutes(12)), "12m");
        assert_eq!(duration_compact(Duration::minutes(83)), "1h 23m");
    }

    #[test]
    fn compact_clamps_negative() {
        assert_eq!(duration_compact(Duration::seconds(-30)), "0s");
    }

    #[test]
    fn hms_pads() {
        assert_eq!(duration_hms(Duration::seconds(5)), "00:00:05");
        assert_eq!(duration_hms(Duration::seconds(3725)), "01:02:05");
    }
}
```

- [ ] **Step 5: Create `src/cli/commands.rs` with stubs**

```rust
//! Handlers for each CLI subcommand.
//!
//! Each function returns `anyhow::Result<i32>` where the integer is the exit
//! code. 0 = success; 1 = logical failure; other codes reserved.

use crate::cli::context::Context;

pub fn add(_ctx: &mut Context, _title: &str, _description: Option<&str>) -> anyhow::Result<i32> {
    todo!("Task 8")
}

pub fn list(
    _ctx: &mut Context,
    _all: bool,
    _archived: bool,
    _completed: bool,
) -> anyhow::Result<i32> {
    todo!("Task 9")
}

pub fn start(_ctx: &mut Context, _target: &str) -> anyhow::Result<i32> {
    todo!("Task 10")
}

pub fn stop(_ctx: &mut Context) -> anyhow::Result<i32> {
    todo!("Task 11")
}

pub fn status(_ctx: &mut Context) -> anyhow::Result<i32> {
    todo!("Task 11")
}

pub fn done(_ctx: &mut Context, _id: i64) -> anyhow::Result<i32> {
    todo!("Task 12")
}

pub fn archive(_ctx: &mut Context, _id: i64) -> anyhow::Result<i32> {
    todo!("Task 12")
}

pub fn delete(_ctx: &mut Context, _id: i64) -> anyhow::Result<i32> {
    todo!("Task 12")
}
```

- [ ] **Step 6: Create `src/cli/mod.rs`**

```rust
//! CLI surface for `bl`.

pub mod args;
pub mod commands;
pub mod context;
pub mod format;

use args::{Cli, Commands};
use clap::Parser;

pub fn run() -> anyhow::Result<i32> {
    let cli = Cli::parse();
    let mut ctx = context::open()?;
    match cli.command {
        Commands::Add { title, description } => commands::add(&mut ctx, &title, description.as_deref()),
        Commands::List { all, archived, completed } => commands::list(&mut ctx, all, archived, completed),
        Commands::Start { target } => commands::start(&mut ctx, &target),
        Commands::Stop | Commands::Pause => commands::stop(&mut ctx),
        Commands::Status => commands::status(&mut ctx),
        Commands::Done { id } => commands::done(&mut ctx, id),
        Commands::Archive { id } => commands::archive(&mut ctx, id),
        Commands::Delete { id } => commands::delete(&mut ctx, id),
    }
}
```

- [ ] **Step 7: Wire `cli` into the library**

Replace `src/lib.rs`:

```rust
//! Buckland — personal time tracker core library.

pub mod cli;
pub mod config;
pub mod domain;
pub mod storage;
```

- [ ] **Step 8: Rewrite `src/main.rs`**

```rust
fn main() {
    let code = match buckland::cli::run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            2
        }
    };
    std::process::exit(code);
}
```

- [ ] **Step 9: Smoke test that clap parses help**

Create `tests/cli_smoke.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn help_shows_subcommands() {
    Command::cargo_bin("bl").unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("add"))
        .stdout(contains("list"))
        .stdout(contains("start"))
        .stdout(contains("stop"))
        .stdout(contains("status"));
}

#[test]
fn unknown_subcommand_fails() {
    Command::cargo_bin("bl").unwrap()
        .arg("banana")
        .assert()
        .failure();
}
```

- [ ] **Step 10: Run tests**

Run: `cargo test`
Expected: 25 previous + 3 new format tests + 2 CLI smoke tests = 30 pass. The subcommand stubs that use `todo!()` are not exercised yet.

- [ ] **Step 11: Verify manual `bl --help`**

Run: `cargo run -- --help`
Expected: clap-rendered help listing all subcommands.

- [ ] **Step 12: Commit**

```bash
git add Cargo.toml Cargo.lock src/ tests/
git commit -m "feat(cli): clap skeleton with subcommand stubs"
```

---

## Task 8: `bl add <title> [--description <text>]`

**Files:**
- Modify: `src/cli/commands.rs` (fill in `add`)
- Create: `tests/cli_add.rs`

- [ ] **Step 1: Write integration tests**

Create `tests/cli_add.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

fn bl_in(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path());
    cmd
}

#[test]
fn add_without_description_creates_task() {
    let home = TempDir::new().unwrap();

    bl_in(&home).args(["add", "fix login"]).assert().success();

    bl_in(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("fix login"));
}

#[test]
fn add_with_description_creates_task() {
    let home = TempDir::new().unwrap();

    bl_in(&home)
        .args(["add", "ship feature", "--description", "plus docs"])
        .assert()
        .success();

    bl_in(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("ship feature"));
}

#[test]
fn add_requires_title() {
    let home = TempDir::new().unwrap();
    bl_in(&home).args(["add"]).assert().failure();
}
```

- [ ] **Step 2: Run tests — expect failures**

Run: `cargo test --test cli_add`
Expected: FAIL. Either `todo!()` panic on `add`, or `list` panics too.

- [ ] **Step 3: Implement `add`**

In `src/cli/commands.rs`, replace the `add` body:

```rust
use crate::storage::Repo;

pub fn add(ctx: &mut Context, title: &str, description: Option<&str>) -> anyhow::Result<i32> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        anyhow::bail!("title cannot be empty");
    }
    let task = ctx.repo.create_task(trimmed, description)?;
    println!("Added task #{} — {}", task.id, task.title);
    Ok(0)
}
```

- [ ] **Step 4: Implement `list` (preview version — full version in Task 9)**

Task 8's tests need `list` to print task titles. Replace the `list` stub with a first-pass implementation that renders open tasks only; Task 9 extends it.

```rust
use crate::cli::format::duration_compact;
use chrono::Utc;

pub fn list(
    ctx: &mut Context,
    _all: bool,
    _archived: bool,
    _completed: bool,
) -> anyhow::Result<i32> {
    let now = Utc::now();
    let tasks = ctx.repo.list_open_tasks()?;
    if tasks.is_empty() {
        println!("No open tasks. Use `bl add \"title\"` to create one.");
        return Ok(0);
    }
    for t in tasks {
        let total = ctx.repo.task_total_duration(t.id, now)?;
        println!("{:>4}  {}  ({})", t.id, t.title, duration_compact(total));
    }
    Ok(0)
}
```

- [ ] **Step 5: Run tests — expect pass**

Run: `cargo test --test cli_add`
Expected: all 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/ tests/
git commit -m "feat(cli): add command + first-pass list"
```

---

## Task 9: `bl list [--all | --archived | --completed]`

Extend `list` to honor the three flags. Phase A spec: default lists only open; `--all` shows every task; `--archived` shows archived; `--completed` shows completed. Flags are mutually exclusive (enforced by `clap` in `args.rs`).

**Files:**
- Modify: `src/cli/commands.rs`
- Create: `tests/cli_list.rs`

- [ ] **Step 1: Write tests**

Create `tests/cli_list.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

fn bl(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path());
    cmd
}

fn seed(home: &TempDir) {
    bl(home).args(["add", "open task"]).assert().success();
    bl(home).args(["add", "to be done"]).assert().success();
    bl(home).args(["add", "to be archived"]).assert().success();
    bl(home).args(["done", "2"]).assert().success();
    bl(home).args(["archive", "3"]).assert().success();
}

#[test]
fn default_shows_only_open() {
    let home = TempDir::new().unwrap();
    seed(&home);
    bl(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("open task"))
        .stdout(contains("to be done").not())
        .stdout(contains("to be archived").not());
}

#[test]
fn completed_flag_shows_completed() {
    let home = TempDir::new().unwrap();
    seed(&home);
    bl(&home)
        .args(["list", "--completed"])
        .assert()
        .success()
        .stdout(contains("to be done"));
}

#[test]
fn archived_flag_shows_archived() {
    let home = TempDir::new().unwrap();
    seed(&home);
    bl(&home)
        .args(["list", "--archived"])
        .assert()
        .success()
        .stdout(contains("to be archived"));
}

#[test]
fn all_flag_shows_everything() {
    let home = TempDir::new().unwrap();
    seed(&home);
    bl(&home)
        .args(["list", "--all"])
        .assert()
        .success()
        .stdout(contains("open task"))
        .stdout(contains("to be done"))
        .stdout(contains("to be archived"));
}
```

Note: Task 12 implements `done` and `archive`; Task 9 tests will fail at seed until Task 12 lands. Alternative if executing strictly in order: replace `seed` with direct `create_task` setup via environment or move the Task 9 tests to after Task 12. Keep the tests here as written — the implementation order below completes `done`/`archive` in Task 12, at which point these run green.

- [ ] **Step 2: Implement full `list` logic**

Replace the `list` function in `src/cli/commands.rs`:

```rust
use crate::domain::Task;

pub fn list(
    ctx: &mut Context,
    all: bool,
    archived: bool,
    completed: bool,
) -> anyhow::Result<i32> {
    let now = chrono::Utc::now();
    let tasks: Vec<Task> = if all {
        ctx.repo.list_all_tasks()?
    } else if archived {
        ctx.repo.list_archived_tasks()?
    } else if completed {
        ctx.repo.list_completed_tasks()?
    } else {
        ctx.repo.list_open_tasks()?
    };

    if tasks.is_empty() {
        match (all, archived, completed) {
            (true, _, _) => println!("No tasks at all. Use `bl add \"title\"`."),
            (_, true, _) => println!("No archived tasks."),
            (_, _, true) => println!("No completed tasks."),
            _ => println!("No open tasks. Use `bl add \"title\"` to create one."),
        }
        return Ok(0);
    }

    for t in tasks {
        let total = ctx.repo.task_total_duration(t.id, now)?;
        let status = status_glyph(&t);
        println!("{status} {:>4}  {:<40}  {}", t.id, truncate(&t.title, 40), crate::cli::format::duration_compact(total));
    }
    Ok(0)
}

fn status_glyph(t: &Task) -> &'static str {
    if t.completed_at.is_some() {
        "✓"
    } else if t.archived_at.is_some() {
        "·"
    } else {
        " "
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}
```

- [ ] **Step 3: Commit (tests for `done`/`archive` still failing; that's expected until Task 12)**

```bash
git add src/ tests/
git commit -m "feat(cli): list honors --all / --archived / --completed flags"
```

---

## Task 10: `bl start <target>`

Target resolution order in Phase A:

1. If `<target>` parses as a positive integer → look up task by id. Error if not found.
2. Otherwise → treat `<target>` as a new task's title, create it, start.

SC-ID resolution (`"SC-123"` / numeric-but-shortcut-id) is Phase B territory and is explicitly out of scope here.

**Files:**
- Modify: `src/cli/commands.rs`
- Create: `src/cli/resolve.rs`
- Modify: `src/cli/mod.rs`
- Create: `tests/cli_start.rs`

- [ ] **Step 1: Write resolver helper (unit tested)**

Create `src/cli/resolve.rs`:

```rust
//! Resolve a `bl start <target>` argument to a task id, creating a new task
//! if the target is non-numeric free text.

use crate::domain::Task;
use crate::storage::{Repo, SqliteRepo};

pub enum Resolved {
    Existing(i64),
    Created(Task),
}

pub fn resolve_or_create(repo: &mut SqliteRepo, target: &str) -> anyhow::Result<Resolved> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        anyhow::bail!("start target cannot be empty");
    }
    if let Ok(id) = trimmed.parse::<i64>() {
        if id <= 0 {
            anyhow::bail!("task id must be positive");
        }
        return match repo.find_task(id)? {
            Some(t) => Ok(Resolved::Existing(t.id)),
            None => anyhow::bail!("task #{id} not found"),
        };
    }
    let task = repo.create_task(trimmed, None)?;
    Ok(Resolved::Created(task))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_target_hits_existing_task() {
        let mut repo = SqliteRepo::in_memory();
        let t = repo.create_task("existing", None).unwrap();
        match resolve_or_create(&mut repo, &t.id.to_string()).unwrap() {
            Resolved::Existing(id) => assert_eq!(id, t.id),
            Resolved::Created(_) => panic!("should have found existing"),
        }
    }

    #[test]
    fn numeric_missing_errors() {
        let mut repo = SqliteRepo::in_memory();
        let err = resolve_or_create(&mut repo, "999").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn text_target_creates_task() {
        let mut repo = SqliteRepo::in_memory();
        match resolve_or_create(&mut repo, "brand new thing").unwrap() {
            Resolved::Created(t) => assert_eq!(t.title, "brand new thing"),
            Resolved::Existing(_) => panic!("should have created"),
        }
    }

    #[test]
    fn empty_target_errors() {
        let mut repo = SqliteRepo::in_memory();
        assert!(resolve_or_create(&mut repo, "   ").is_err());
    }
}
```

- [ ] **Step 2: Register the module**

Edit `src/cli/mod.rs`, add to the module list:

```rust
pub mod resolve;
```

(Keep the other `pub mod` lines; just add this one.)

- [ ] **Step 3: Write integration tests**

Create `tests/cli_start.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

fn bl(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path());
    cmd
}

#[test]
fn start_by_numeric_id_works() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "existing"]).assert().success();
    bl(&home)
        .args(["start", "1"])
        .assert()
        .success()
        .stdout(contains("existing"));
    bl(&home).args(["status"]).assert().success();
}

#[test]
fn start_by_text_creates_and_starts() {
    let home = TempDir::new().unwrap();
    bl(&home)
        .args(["start", "new quick thing"])
        .assert()
        .success()
        .stdout(contains("new quick thing"));
    bl(&home).args(["status"]).assert().success();
}

#[test]
fn start_missing_numeric_errors() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["start", "999"]).assert().failure();
}

#[test]
fn start_switches_active_task() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "first"]).assert().success();
    bl(&home).args(["add", "second"]).assert().success();
    bl(&home).args(["start", "1"]).assert().success();
    bl(&home)
        .args(["start", "2"])
        .assert()
        .success()
        .stdout(contains("second"));
    // Only one entry should be active; verified indirectly via status printing "second".
    bl(&home)
        .args(["status"])
        .assert()
        .success()
        .stdout(contains("second"));
}
```

- [ ] **Step 4: Implement `start`**

Replace the `start` function in `src/cli/commands.rs`:

```rust
use crate::domain::TimerOps;

pub fn start(ctx: &mut Context, target: &str) -> anyhow::Result<i32> {
    let resolved = crate::cli::resolve::resolve_or_create(&mut ctx.repo, target)?;
    let (task_id, task_title) = match resolved {
        crate::cli::resolve::Resolved::Existing(id) => {
            let t = ctx.repo.find_task(id)?.expect("resolved id");
            (t.id, t.title)
        }
        crate::cli::resolve::Resolved::Created(t) => (t.id, t.title),
    };

    let now = chrono::Utc::now();
    let entry = TimerOps::new(&mut ctx.repo).start(task_id, now)?;
    println!(
        "Started #{task_id} {task_title} (entry {}, {})",
        entry.id,
        entry.started_at.with_timezone(&chrono::Local).format("%H:%M:%S"),
    );
    Ok(0)
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test`
Expected: previous passing tests + `cli_start` all 4 pass + `cli::resolve` 4 unit tests. `cli_list` tests that depend on `done`/`archive` still panic (expected).

- [ ] **Step 6: Commit**

```bash
git add src/ tests/
git commit -m "feat(cli): start resolves target to task or creates from free text"
```

---

## Task 11: `bl stop`, `bl pause` (alias), `bl status`

**Files:**
- Modify: `src/cli/commands.rs`
- Create: `tests/cli_stop_status.rs`

Contract:

- `bl stop` prints `Stopped #<id> <title> (<HH:MM:SS>)` and exits 0. If idle, prints `Nothing to stop.` and exits 1.
- `bl pause` is identical to `bl stop` (alias in `args.rs` already dispatches to the same handler).
- `bl status` prints `<title> — <HH:MM:SS> (started <HH:MM:SS>)` and exits 0 if active. If idle, prints `No active timer.` and exits 1.

- [ ] **Step 1: Write tests**

Create `tests/cli_stop_status.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

fn bl(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path());
    cmd
}

#[test]
fn status_idle_exits_one() {
    let home = TempDir::new().unwrap();
    bl(&home)
        .args(["status"])
        .assert()
        .code(1)
        .stdout(contains("No active"));
}

#[test]
fn status_active_exits_zero() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["start", "do thing"]).assert().success();
    bl(&home)
        .args(["status"])
        .assert()
        .code(0)
        .stdout(contains("do thing"));
}

#[test]
fn stop_ends_the_active_entry() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["start", "do thing"]).assert().success();
    bl(&home)
        .args(["stop"])
        .assert()
        .code(0)
        .stdout(contains("Stopped"));
    bl(&home).args(["status"]).assert().code(1);
}

#[test]
fn stop_when_idle_exits_one() {
    let home = TempDir::new().unwrap();
    bl(&home)
        .args(["stop"])
        .assert()
        .code(1)
        .stdout(contains("Nothing to stop"));
}

#[test]
fn pause_is_alias_for_stop() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["start", "do thing"]).assert().success();
    bl(&home)
        .args(["pause"])
        .assert()
        .code(0)
        .stdout(contains("Stopped"));
}
```

- [ ] **Step 2: Implement `stop`**

Replace the `stop` function in `src/cli/commands.rs`:

```rust
pub fn stop(ctx: &mut Context) -> anyhow::Result<i32> {
    let now = chrono::Utc::now();
    match TimerOps::new(&mut ctx.repo).stop(now)? {
        Some(entry) => {
            let task = ctx.repo.find_task(entry.task_id)?.expect("entry has task");
            let elapsed = entry.duration(now);
            println!(
                "Stopped #{} {} ({})",
                task.id,
                task.title,
                crate::cli::format::duration_hms(elapsed),
            );
            Ok(0)
        }
        None => {
            println!("Nothing to stop.");
            Ok(1)
        }
    }
}
```

- [ ] **Step 3: Implement `status`**

Replace the `status` function:

```rust
pub fn status(ctx: &mut Context) -> anyhow::Result<i32> {
    let now = chrono::Utc::now();
    match ctx.repo.active_time_entry()? {
        Some(entry) => {
            let task = ctx.repo.find_task(entry.task_id)?.expect("entry has task");
            let elapsed = entry.duration(now);
            let started_local = entry.started_at.with_timezone(&chrono::Local);
            println!(
                "{} — {} (started {})",
                task.title,
                crate::cli::format::duration_hms(elapsed),
                started_local.format("%H:%M:%S"),
            );
            Ok(0)
        }
        None => {
            println!("No active timer.");
            Ok(1)
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test cli_stop_status`
Expected: all 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/ tests/
git commit -m "feat(cli): stop, pause alias, and status"
```

---

## Task 12: `bl done <id>`, `bl archive <id>`, `bl delete <id>`

**Files:**
- Modify: `src/cli/commands.rs`
- Create: `tests/cli_lifecycle.rs`

Contract:

- `done <id>` — marks `completed_at = now`, prints `Done: #<id> <title>`, exits 0. Unknown id exits 1.
- `archive <id>` — marks `archived_at = now`, prints `Archived: #<id> <title>`, exits 0. Unknown id exits 1.
- `delete <id>` — hard delete. If the task has time entries, exits 1 and suggests `archive`. Prints `Deleted: #<id>` on success.

- [ ] **Step 1: Write tests**

Create `tests/cli_lifecycle.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

fn bl(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path());
    cmd
}

#[test]
fn done_marks_task_complete() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "finish me"]).assert().success();
    bl(&home)
        .args(["done", "1"])
        .assert()
        .success()
        .stdout(contains("Done"));
    // Default list hides completed.
    bl(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("finish me").not());
}

#[test]
fn archive_hides_from_default_list() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "shelve me"]).assert().success();
    bl(&home).args(["archive", "1"]).assert().success();
    bl(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("shelve me").not());
}

#[test]
fn delete_empty_task_works() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "mistaken"]).assert().success();
    bl(&home)
        .args(["delete", "1"])
        .assert()
        .success()
        .stdout(contains("Deleted"));
}

#[test]
fn delete_task_with_entries_is_blocked() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["start", "real work"]).assert().success();
    bl(&home).args(["stop"]).assert().success();
    bl(&home)
        .args(["delete", "1"])
        .assert()
        .code(1)
        .stdout(contains("archive"));
}

#[test]
fn done_unknown_id_fails() {
    let home = TempDir::new().unwrap();
    bl(&home)
        .args(["done", "42"])
        .assert()
        .code(1)
        .stdout(contains("not found"));
}
```

- [ ] **Step 2: Implement `done`, `archive`, `delete`**

Replace the three stubs in `src/cli/commands.rs`:

```rust
use crate::storage::RepoError;

pub fn done(ctx: &mut Context, id: i64) -> anyhow::Result<i32> {
    match ctx.repo.mark_task_done(id, chrono::Utc::now()) {
        Ok(t) => {
            println!("Done: #{} {}", t.id, t.title);
            Ok(0)
        }
        Err(RepoError::TaskNotFound(_)) => {
            println!("Task #{id} not found.");
            Ok(1)
        }
        Err(e) => Err(e.into()),
    }
}

pub fn archive(ctx: &mut Context, id: i64) -> anyhow::Result<i32> {
    match ctx.repo.archive_task(id, chrono::Utc::now()) {
        Ok(t) => {
            println!("Archived: #{} {}", t.id, t.title);
            Ok(0)
        }
        Err(RepoError::TaskNotFound(_)) => {
            println!("Task #{id} not found.");
            Ok(1)
        }
        Err(e) => Err(e.into()),
    }
}

pub fn delete(ctx: &mut Context, id: i64) -> anyhow::Result<i32> {
    match ctx.repo.delete_task(id) {
        Ok(()) => {
            println!("Deleted: #{id}");
            Ok(0)
        }
        Err(RepoError::TaskHasEntries(_)) => {
            println!(
                "Task #{id} has time entries. Use `bl archive {id}` to hide it without losing history."
            );
            Ok(1)
        }
        Err(RepoError::TaskNotFound(_)) => {
            println!("Task #{id} not found.");
            Ok(1)
        }
        Err(e) => Err(e.into()),
    }
}
```

- [ ] **Step 3: Run full test suite**

Run: `cargo test`
Expected: every test passes, including the `cli_list` tests that were waiting on `done`/`archive`.

- [ ] **Step 4: Manual smoke test**

Run the Postconditions checklist at the top of this file. All commands should behave as documented.

- [ ] **Step 5: Lint pass**

Run: `cargo fmt --all && cargo clippy --all-targets -- -D warnings`
Expected: no output (clean).

- [ ] **Step 6: Commit**

```bash
git add src/ tests/
git commit -m "feat(cli): done, archive, delete with history-safe guard"
```

---

## Phase A complete

At this point `bl` is a usable personal time-tracker via CLI. Smoke flow:

```bash
export BUCKLAND_HOME=/tmp/bl-smoke
rm -rf "$BUCKLAND_HOME"

cargo run -- add "fix login bug"
cargo run -- add "refactor import script"
cargo run -- list                # 2 rows, both open
cargo run -- start 1
cargo run -- status              # exit 0
cargo run -- start 2             # switches active atomically
cargo run -- status              # shows #2
cargo run -- stop
cargo run -- status              # exit 1
cargo run -- done 1
cargo run -- list                # only #2 open
cargo run -- list --completed    # shows #1
cargo run -- delete 2            # fails because #2 has entries
cargo run -- archive 2           # works
cargo run -- list --all          # both visible
```

## Self-review checklist for the executing engineer

Before handing the phase off, run this top-to-bottom:

1. **Coverage of the CLI contract at the top of this document.** Every command listed in "What this phase delivers" has an integration test with at least one happy path.
2. **Exit codes.** `status` and `stop` use 0/1 correctly. `delete` blocked by entries returns 1. `done`/`archive` on unknown id return 1.
3. **`cargo test` green, `cargo clippy --all-targets -- -D warnings` clean, `cargo fmt --all --check` clean.**
4. **No `todo!()`, `unimplemented!()`, or `dbg!()` leftover in `src/`.** Run `grep -rn 'todo!\|unimplemented!\|dbg!' src/` — expected empty.
5. **`Cargo.lock` is committed alongside `Cargo.toml`** — without it, CI builds drift.
6. **`BUCKLAND_HOME` override works** — remove `~/.local/share/buckland/` if it exists and run the smoke flow with and without the env var to verify.

## What's next

When Phase A is green:

- **Phase B — Shortcut integration.** Adds `--sc` to `bl add`, adds `bl shortcut <SC-ID>` for force-refresh, extends `bl start <target>` to resolve `SC-NNN` and bare numeric-as-shortcut-id. Introduces `src/shortcut/` with typed errors and the 1h-TTL cache. File: `docs/superpowers/plans/2026-04-22-buckland-phase-b-shortcut.md` (to be drafted).
- **Phase C — Report.** `bl report` with scopes, groupings, Unicode bars, `--json`.
- **Phase D — TUI.** ratatui screens, keymap, modals.
- **Phase E — Tray + clipboard.** `bl-tray` binary, clipboard helper for "copy summary."
- **Phase F — OSS release.** CI, `.deb`, release workflow, README with screenshots, CHANGELOG, CONTRIBUTING, CoC.

See `docs/superpowers/plans/README.md` for the phase index.

