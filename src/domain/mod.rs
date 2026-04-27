//! Domain types and operations.

pub mod report;
pub mod shortcut_story;
pub mod task;
pub mod time_entry;
pub mod timer_ops;

pub use report::{Grouping, Scope, ScopeError, ScopeKind};
pub use shortcut_story::ShortcutStory;
pub use task::Task;
pub use time_entry::TimeEntry;
pub use timer_ops::TimerOps;
