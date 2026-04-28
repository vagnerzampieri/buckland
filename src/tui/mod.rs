//! Buckland TUI — ratatui surface composed on top of the `Repo` trait.

pub mod keymap;
pub mod terminal;
pub mod theme;

use crate::cli::context::Context;

pub fn run(_ctx: Context) -> anyhow::Result<i32> {
    Ok(0)
}
