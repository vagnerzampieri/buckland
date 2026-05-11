//! `bl tray` — runs the tray-icon loop in this process.
//!
//! Equivalent to invoking the `bl-tray` binary; the subcommand exists
//! so users who installed `bl` without that binary on PATH can still
//! launch the tray with one command. Reads the same config and DB path
//! as the CLI surface (no special context).

use crate::cli::context::Context;

#[cfg(feature = "tray")]
pub fn tray(ctx: &Context) -> anyhow::Result<i32> {
    use crate::config;
    let cfg = config::load(&config::config_path())?;
    let runtime = crate::tray::runtime::TrayRuntimeConfig {
        db_path: ctx.db_path.clone(),
        poll_seconds: cfg.tray.poll_seconds,
    };
    crate::tray::runtime::run(runtime)?;
    Ok(0)
}

#[cfg(not(feature = "tray"))]
pub fn tray(_ctx: &Context) -> anyhow::Result<i32> {
    // Unreachable in practice — clap won't surface the variant when
    // the feature is off — but a defensive stub makes the dispatch in
    // `cli/mod.rs` compile under both feature configurations.
    eprintln!("`bl tray` requires the `tray` feature. Reinstall with `cargo install buckland`.");
    Ok(2)
}
