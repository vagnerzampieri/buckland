//! SQLite storage layer.

pub mod migrate;
pub mod repo;

pub use repo::{Repo, RepoError, RepoResult, SqliteRepo};

use rusqlite::Connection;
use std::path::Path;

/// Open a SQLite connection at `path`, apply migrations, and return it.
pub fn open(path: &Path) -> anyhow::Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "busy_timeout", 3000)?;
    migrate::run(&mut conn)?;
    Ok(conn)
}

/// Open an in-memory SQLite connection with migrations applied. Used in tests.
pub fn open_memory() -> anyhow::Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "busy_timeout", 3000)?;
    migrate::run(&mut conn)?;
    Ok(conn)
}
