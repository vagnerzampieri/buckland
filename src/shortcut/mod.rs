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

pub use story::Story;
// Note: re-exports for `id::normalize`, `client::Client`,
// `fetcher::Fetcher`, and their error types are added in tasks 2, 3, and 5.
