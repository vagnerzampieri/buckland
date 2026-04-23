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
        title: String,
        #[arg(long)]
        description: Option<String>,
    },
    /// List tasks. Default: only open tasks.
    List {
        #[arg(long, conflicts_with_all = ["archived", "completed"])]
        all: bool,
        #[arg(long, conflicts_with = "completed")]
        archived: bool,
        #[arg(long)]
        completed: bool,
    },
    /// Start a timer. <target> can be a numeric task id or free-text title.
    Start { target: String },
    /// Stop the active timer.
    Stop,
    /// Alias for stop.
    Pause,
    /// Print the active timer. Exit 0 if active, 1 if idle.
    Status,
    /// Mark a task as done.
    Done { id: i64 },
    /// Archive a task (hide it from default list).
    Archive { id: i64 },
    /// Hard-delete a task. Fails if the task has time entries.
    Delete { id: i64 },
}
