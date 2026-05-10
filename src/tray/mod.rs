//! The `bl-tray` surface — a passive StatusNotifierItem icon that
//! polls the SQLite file in read-only mode and surfaces the active
//! timer's state via tooltip + icon swap.

pub mod assets;
pub mod runtime;
pub mod state;

pub use runtime::{run, TrayRuntimeConfig};
