//! Story — the fetched representation of a Shortcut story.
//!
//! This is the value returned by the HTTP client. The repo caches it into
//! the `shortcut_stories` table via [`crate::domain::ShortcutStory`].

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Story {
    pub external_id: i64,
    pub title: Option<String>,
    pub epic_name: Option<String>,
    pub state: Option<String>,
}
