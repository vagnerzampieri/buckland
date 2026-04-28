//! Buckland TUI — ratatui surface composed on top of the `Repo` trait.

pub mod terminal;

use crate::cli::context::Context;

pub fn run(_ctx: Context) -> anyhow::Result<i32> {
    // Real loop lands in Task 5. Until then we still return 0 so the
    // subcommand smoke tests stay green.
    Ok(0)
}
