//! Buckland — personal time tracker core library.

pub mod cli;
pub mod config;
pub mod domain;
pub mod shortcut;
pub mod storage;
#[cfg(feature = "tray")]
pub mod tray;
pub mod tui;
