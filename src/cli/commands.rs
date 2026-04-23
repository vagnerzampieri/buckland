//! Handlers for each CLI subcommand.
//!
//! Each function returns `anyhow::Result<i32>` where the integer is the exit
//! code. 0 = success; 1 = logical failure; other codes reserved.

use crate::cli::context::Context;
use crate::domain::{Task, TimerOps};
use crate::storage::{Repo, RepoError};

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

pub fn stop(ctx: &mut Context) -> anyhow::Result<i32> {
    let now = chrono::Utc::now();
    match TimerOps::new(&mut ctx.repo).stop(now)? {
        Some(entry) => {
            let task = ctx.repo.find_task(entry.task_id)?.expect("entry has task");
            let elapsed = entry.duration(now);
            println!(
                "Stopped #{} {} ({})",
                task.id,
                task.title,
                crate::cli::format::duration_hms(elapsed),
            );
            Ok(0)
        }
        None => {
            println!("Nothing to stop.");
            Ok(1)
        }
    }
}

pub fn status(ctx: &mut Context) -> anyhow::Result<i32> {
    let now = chrono::Utc::now();
    match ctx.repo.active_time_entry()? {
        Some(entry) => {
            let task = ctx.repo.find_task(entry.task_id)?.expect("entry has task");
            let elapsed = entry.duration(now);
            let started_local = entry.started_at.with_timezone(&chrono::Local);
            println!(
                "{} — {} (started {})",
                task.title,
                crate::cli::format::duration_hms(elapsed),
                started_local.format("%H:%M:%S"),
            );
            Ok(0)
        }
        None => {
            println!("No active timer.");
            Ok(1)
        }
    }
}

pub fn done(ctx: &mut Context, id: i64) -> anyhow::Result<i32> {
    match ctx.repo.mark_task_done(id, chrono::Utc::now()) {
        Ok(t) => {
            println!("Done: #{} {}", t.id, t.title);
            Ok(0)
        }
        Err(RepoError::TaskNotFound(_)) => {
            println!("Task #{id} not found.");
            Ok(1)
        }
        Err(e) => Err(e.into()),
    }
}

pub fn archive(ctx: &mut Context, id: i64) -> anyhow::Result<i32> {
    match ctx.repo.archive_task(id, chrono::Utc::now()) {
        Ok(t) => {
            println!("Archived: #{} {}", t.id, t.title);
            Ok(0)
        }
        Err(RepoError::TaskNotFound(_)) => {
            println!("Task #{id} not found.");
            Ok(1)
        }
        Err(e) => Err(e.into()),
    }
}

pub fn delete(ctx: &mut Context, id: i64) -> anyhow::Result<i32> {
    match ctx.repo.delete_task(id) {
        Ok(()) => {
            println!("Deleted: #{id}");
            Ok(0)
        }
        Err(RepoError::TaskHasEntries(_)) => {
            println!(
                "Task #{id} has time entries. Use `bl archive {id}` to hide it without losing history."
            );
            Ok(1)
        }
        Err(RepoError::TaskNotFound(_)) => {
            println!("Task #{id} not found.");
            Ok(1)
        }
        Err(e) => Err(e.into()),
    }
}
