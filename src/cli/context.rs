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
