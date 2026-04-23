//! Handlers for each CLI subcommand.
//!
//! Each function returns `anyhow::Result<i32>` where the integer is the exit
//! code. 0 = success; 1 = logical failure; other codes reserved.

use crate::cli::context::Context;
use crate::domain::{Task, TimerOps};
use crate::storage::Repo;

pub fn add(ctx: &mut Context, title: &str, description: Option<&str>) -> anyhow::Result<i32> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        anyhow::bail!("title cannot be empty");
    }
    let task = ctx.repo.create_task(trimmed, description)?;
    println!("Added task #{} — {}", task.id, task.title);
    Ok(0)
}

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

    for t in tasks {
        let total = ctx.repo.task_total_duration(t.id, now)?;
        let status = status_glyph(&t);
        println!(
            "{status} {:>4}  {:<40}  {}",
            t.id,
            truncate(&t.title, 40),
            crate::cli::format::duration_compact(total)
        );
    }
    Ok(0)
}

fn status_glyph(t: &Task) -> &'static str {
    if t.completed_at.is_some() {
        "✓"
    } else if t.archived_at.is_some() {
        "·"
    } else {
        " "
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}

pub fn start(ctx: &mut Context, target: &str) -> anyhow::Result<i32> {
    let resolved = crate::cli::resolve::resolve_or_create(&mut ctx.repo, target)?;
    let (task_id, task_title) = match resolved {
        crate::cli::resolve::Resolved::Existing(id) => {
            let t = ctx.repo.find_task(id)?.expect("resolved id");
            (t.id, t.title)
        }
        crate::cli::resolve::Resolved::Created(t) => (t.id, t.title),
    };

    let now = chrono::Utc::now();
    let entry = TimerOps::new(&mut ctx.repo).start(task_id, now)?;
    println!(
        "Started #{task_id} {task_title} (entry {}, {})",
        entry.id,
        entry
            .started_at
            .with_timezone(&chrono::Local)
            .format("%H:%M:%S"),
    );
    Ok(0)
}

pub fn stop(_ctx: &mut Context) -> anyhow::Result<i32> {
    todo!("Task 11")
}

pub fn status(_ctx: &mut Context) -> anyhow::Result<i32> {
    todo!("Task 11")
}

pub fn done(_ctx: &mut Context, _id: i64) -> anyhow::Result<i32> {
    todo!("Task 12")
}

pub fn archive(_ctx: &mut Context, _id: i64) -> anyhow::Result<i32> {
    todo!("Task 12")
}

pub fn delete(_ctx: &mut Context, _id: i64) -> anyhow::Result<i32> {
    todo!("Task 12")
}
