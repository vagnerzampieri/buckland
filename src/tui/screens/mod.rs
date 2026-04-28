//! Per-screen state + rendering. Each screen owns a `*State` struct and
//! exposes `render` + `handle_key`. Mutations always go through the
//! parent `App<R>`'s `Repo`.

pub mod tasks;
