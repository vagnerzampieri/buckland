use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "bl",
    about = "Time tracker for developers who use Shortcut",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
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
}
