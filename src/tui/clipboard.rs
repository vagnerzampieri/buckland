//! Phase D wired the TUI Report screen against `tui::clipboard::copy`.
//! Phase E promoted the helper to the top-level `crate::clipboard`
//! module so the CLI's `bl report --copy` can use it without a TUI
//! dependency. We keep the old import path alive via a re-export.

pub use crate::clipboard::{copy, ClipboardError, Tool};
