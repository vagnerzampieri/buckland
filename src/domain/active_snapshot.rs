use chrono::{DateTime, Utc};

/// A flat view of the currently active timer with everything the tray
/// needs to render its tooltip in one allocation.
///
/// Produced by [`crate::storage::Repo::active_snapshot`] from a single
/// `JOIN` across `time_entries`, `tasks`, and `shortcut_stories`. Pure
/// data; the tray's display logic lives in `crate::tray::state`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveSnapshot {
    /// The active task's primary key.
    pub task_id: i64,
    /// The task's user-facing title.
    pub task_title: String,
    /// The Shortcut story external id, if the task is linked. `None`
    /// for unlinked tasks.
    pub sc_external_id: Option<i64>,
    /// The timer's started timestamp, in UTC.
    pub started_at: DateTime<Utc>,
}
