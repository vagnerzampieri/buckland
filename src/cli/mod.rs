//! CLI surface for `bl`.

pub mod args;
pub mod commands;
pub mod context;
pub mod format;
pub mod resolve;

pub use commands::helpers::truncate as commands_truncate;

use args::{Cli, Commands};
use clap::Parser;

pub fn run() -> anyhow::Result<i32> {
    let cli = Cli::parse();
    let mut ctx = context::open()?;
    let command = cli.command.unwrap_or(Commands::Tui);
    match command {
        Commands::Add {
            title,
            description,
            sc,
        } => commands::add(&mut ctx, &title, description.as_deref(), sc.as_deref()),
        Commands::Shortcut { id } => commands::shortcut_refresh(&mut ctx, &id),
        Commands::List {
            all,
            archived,
            completed,
        } => commands::list(&mut ctx, all, archived, completed),
        Commands::Start { target } => commands::start(&mut ctx, &target),
        Commands::Stop | Commands::Pause => commands::stop(&mut ctx),
        Commands::Status => commands::status(&mut ctx),
        Commands::Done { id } => commands::done(&mut ctx, id),
        Commands::Archive { id } => commands::archive(&mut ctx, id),
        Commands::Delete { id } => commands::delete(&mut ctx, id),
        Commands::Report {
            today,
            week,
            month,
            all,
            range,
            by_task,
            by_epic,
            by_day,
            json,
            copy,
        } => commands::report(
            &mut ctx,
            commands::ReportArgs {
                today,
                week,
                month,
                all,
                range,
                by_task,
                by_epic,
                by_day,
                json,
                copy,
            },
        ),
        Commands::Tui => crate::tui::run(ctx),
        #[cfg(feature = "tray")]
        Commands::Tray => commands::tray(&ctx),
    }
}
