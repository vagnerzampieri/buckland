//! Story — the fetched representation of a Shortcut story.
//!
//! This is the value returned by the HTTP client. The repo caches it into
//! the `shortcut_stories` table via [`crate::domain::ShortcutStory`].

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Story {
    pub external_id: i64,
    pub title: Option<String>,
    /// Shortcut epic id, when the story is attached to one. Transient — not
    /// persisted in `shortcut_stories` (we resolve `epic_name` at fetch time).
    pub epic_id: Option<i64>,
    pub epic_name: Option<String>,
    pub state: Option<String>,
}
