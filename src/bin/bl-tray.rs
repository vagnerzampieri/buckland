//! `bl-tray` — the StatusNotifierItem tray icon.
//!
//! Reads the SQLite file in read-only mode, polls every
//! `Config.tray.poll_seconds` (default 2s — sub-ms SQLite read so cheap),
//! and ticks the tooltip locally at 1Hz so the elapsed clock advances
//! between polls.

use anyhow::Context;
use buckland::config;
use buckland::tray::runtime::{run, TrayRuntimeConfig};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let cfg = config::load(&config::config_path())
        .context("loading config from $XDG_CONFIG_HOME/buckland/config.toml")?;
    let db_path = resolve_db_path();
    let runtime_config = TrayRuntimeConfig {
        db_path,
        poll_seconds: cfg.tray.poll_seconds,
    };
    run(runtime_config)
}

fn resolve_db_path() -> PathBuf {
    match std::env::var("BUCKLAND_HOME") {
        Ok(home) if !home.trim().is_empty() => PathBuf::from(home.trim()).join("buckland.db"),
        _ => config::db_path(),
    }
}
