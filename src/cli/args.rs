use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "bl",
    about = "Time tracker for developers who use Shortcut",
    version,
    subcommand_required = false,
    arg_required_else_help = false
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Add a new task.
    Add {
        /// Short, human-readable title shown in list views.
        title: String,
        /// Optional longer description attached to the task.
        #[arg(long)]
        description: Option<String>,
        /// Link the task to a Shortcut story. Accepts "SC-123", "sc-123", or "123".
        #[arg(long)]
        sc: Option<String>,
    },
    /// List tasks. Default: only open tasks.
    List {
        /// Show all tasks regardless of status.
        #[arg(long, conflicts_with_all = ["archived", "completed"])]
        all: bool,
        /// Show only archived tasks.
        #[arg(long, conflicts_with = "completed")]
        archived: bool,
        /// Show only completed tasks.
        #[arg(long)]
        completed: bool,
    },
    /// Start a timer. <target> can be a numeric task id or free-text title.
    Start {
        /// Numeric task id to resume, or free-text title to create a new task.
        target: String,
    },
    /// Stop the active timer.
    Stop,
    /// Alias for stop.
    Pause,
    /// Print the active timer. Exit 0 if active, 1 if idle.
    Status,
    /// Mark a task as done.
    Done {
        /// Id of the task to mark as done.
        id: i64,
    },
    /// Archive a task (hide it from default list).
    Archive {
        /// Id of the task to archive.
        id: i64,
    },
    /// Hard-delete a task. Fails if the task has time entries.
    Delete {
        /// Id of the task to permanently delete.
        id: i64,
    },
    /// Force re-fetch of a Shortcut story into the local cache.
    Shortcut {
        /// Story id: "SC-123", "sc-123", or "123".
        id: String,
    },
    /// Report time totals over a scope, grouped by task / epic / day.
    Report {
        /// Time tracked today (default).
        #[arg(long, group = "scope")]
        today: bool,
        /// Time tracked in the current ISO week (Monday–Sunday, local).
        #[arg(long, group = "scope")]
        week: bool,
        /// Time tracked in the current calendar month (local).
        #[arg(long, group = "scope")]
        month: bool,
        /// Time tracked across the entire database.
        #[arg(long, group = "scope")]
        all: bool,
        /// Custom range FROM..TO with both endpoints as YYYY-MM-DD (inclusive).
        #[arg(long, group = "scope", value_name = "FROM..TO")]
        range: Option<String>,
        /// Group rows by task (default).
        #[arg(long, group = "grouping")]
        by_task: bool,
        /// Group rows by Shortcut epic (uses cached epic_name).
        #[arg(long, group = "grouping")]
        by_epic: bool,
        /// Group rows by local calendar day.
        #[arg(long, group = "grouping")]
        by_day: bool,
        /// Emit a JSON object instead of a table.
        #[arg(long)]
        json: bool,
    },
    /// Open the TUI. Same as running `bl` with no subcommand.
    Tui,
    /// Run the tray icon process. Same loop as the `bl-tray` binary.
    #[cfg(feature = "tray")]
    Tray,
}
