use crate::cli::context::Context;
use crate::domain::Task;
use crate::storage::Repo;

use super::helpers::{status_glyph, truncate};

pub fn list(ctx: &mut Context, all: bool, archived: bool, completed: bool) -> anyhow::Result<i32> {
    let now = chrono::Utc::now();
    let tasks: Vec<Task> = if all {
        ctx.repo.list_all_tasks()?
    } else if archived {
        ctx.repo.list_archived_tasks()?
    } else if completed {
        ctx.repo.list_completed_tasks()?
    } else {
        ctx.repo.list_open_tasks()?
    };

    if tasks.is_empty() {
        match (all, archived, completed) {
            (true, _, _) => println!("No tasks at all. Use `bl add \"title\"`."),
            (_, true, _) => println!("No archived tasks."),
            (_, _, true) => println!("No completed tasks."),
            _ => println!("No open tasks. Use `bl add \"title\"` to create one."),
        }
        return Ok(0);
    }

    let show_sc = tasks.iter().any(|t| t.shortcut_story_id.is_some());

    for t in tasks {
        let total = ctx.repo.task_total_duration(t.id, now)?;
        let status = status_glyph(&t);
        if show_sc {
            let sc_label = match t.shortcut_story_id {
                Some(row_id) => ctx
                    .repo
                    .find_shortcut_story_by_row_id(row_id)?
                    .map(|s| format!("SC-{}", s.external_id))
                    .unwrap_or_default(),
                None => String::new(),
            };
            println!(
                "{status} {:>4}  {:<40}  {:<8}  {}",
                t.id,
                truncate(&t.title, 40),
                sc_label,
                crate::cli::format::duration_compact(total)
            );
        } else {
            println!(
                "{status} {:>4}  {:<40}  {}",
                t.id,
                truncate(&t.title, 40),
                crate::cli::format::duration_compact(total)
            );
        }
    }
    Ok(0)
}
