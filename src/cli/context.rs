use crate::config;
use crate::shortcut::{Client, Fetcher, DEFAULT_BASE_URL};
use crate::storage::SqliteRepo;
use std::path::PathBuf;

pub struct Context {
    pub repo: SqliteRepo,
    pub db_path: PathBuf,
    pub fetcher: Option<Fetcher>,
}

pub fn open() -> anyhow::Result<Context> {
    let db_path = resolve_db_path();
    let conn = crate::storage::open(&db_path)?;
    let fetcher = build_fetcher()?;
    Ok(Context {
        repo: SqliteRepo::new(conn),
        db_path,
        fetcher,
    })
}

fn resolve_db_path() -> PathBuf {
    match std::env::var("BUCKLAND_HOME") {
        Ok(home) if !home.trim().is_empty() => PathBuf::from(home.trim()).join("buckland.db"),
        _ => config::db_path(),
    }
}

fn build_fetcher() -> anyhow::Result<Option<Fetcher>> {
    let cfg = config::load(&config::config_path())?;
    let Some(token) = cfg.shortcut.token else {
        return Ok(None);
    };
    let base = cfg
        .shortcut
        .api_base_url
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    Ok(Some(Fetcher::new(Client::new(base, token))))
}
