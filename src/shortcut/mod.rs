//! Read-only Shortcut API integration.
//!
//! Three concerns, each in its own submodule:
//! - `id`      — normalize human input ("SC-123", "sc-123", "123") to i64.
//! - `client`  — thin `reqwest::blocking` wrapper with typed errors.
//! - `fetcher` — composes the client with the `shortcut_stories` cache.

pub mod client;
pub mod fetcher;
pub mod id;
pub mod story;

pub use id::{normalize, IdError};
pub use story::Story;
// `Client`, `ShortcutError`, `Fetcher`, `FetcherError` are added in tasks 3 and 5.
