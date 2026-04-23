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
    match std::env::var("BUCKLAND_HOME") {
        Ok(home) if !home.trim().is_empty() => PathBuf::from(home.trim()).join("buckland.db"),
        _ => config::db_path(),
    }
}
