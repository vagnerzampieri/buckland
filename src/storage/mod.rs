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
