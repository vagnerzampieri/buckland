//! Domain types: Task, TimeEntry, ShortcutStory.
//!
//! These are plain data structures with minimal behavior. Persistence is
//! the storage layer's concern; domain operations live in sibling modules
//! like `timer_ops`.

pub mod shortcut_story;
pub mod task;
pub mod time_entry;

pub use shortcut_story::ShortcutStory;
pub use task::Task;
pub use time_entry::TimeEntry;
