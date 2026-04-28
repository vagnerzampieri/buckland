//! Buckland TUI — ratatui surface composed on top of the `Repo` trait.
//!
//! Phase D landing point. Real terminal lifecycle and screens land in later
//! tasks of the same phase; for now `run()` is a no-op that returns 0 so
//! `bl tui --help` and `bl` (no-args) compile and exit cleanly.

use crate::cli::context::Context;

pub fn run(_ctx: Context) -> anyhow::Result<i32> {
    Ok(0)
}
