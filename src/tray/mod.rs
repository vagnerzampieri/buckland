//! The `bl-tray` surface — a passive StatusNotifierItem icon that
//! polls the SQLite file in read-only mode and surfaces the active
//! timer's state via tooltip + icon swap.
//!
//! Layered as:
//!
//! - [`assets`]  — embedded SVG byte slices.
//! - [`state`]   — pure `TrayState` + tooltip/icon mappers (Task E3).
//! - [`runtime`] — ksni service + polling thread + glib tick (Task E4).

pub mod assets;
// state and runtime are added by Tasks E3 and E4.
