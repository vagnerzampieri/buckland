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
    let description = description.map(|s| s.trim()).filter(|s| !s.is_empty());
    let task = ctx.repo.create_task(trimmed, description)?;
    println!("Added: #{} {}", task.id, task.title);
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
    use crate::cli::resolve::{resolve_or_create, ResolveError, Resolved};

    let resolved = match resolve_or_create(&mut ctx.repo, target) {
        Ok(r) => r,
        Err(ResolveError::TaskNotFound(id)) => {
            println!("Task #{id} not found.");
            return Ok(1);
        }
        Err(ResolveError::EmptyTarget) => {
            println!("start target cannot be empty");
            return Ok(1);
        }
        Err(ResolveError::NonPositiveId) => {
            println!("task id must be positive");
            return Ok(1);
        }
        Err(ResolveError::Repo(e)) => return Err(e.into()),
    };

    let task_id = match resolved {
        Resolved::Existing(id) => id,
        Resolved::Created(t) => t.id,
    };

    let task = ctx
        .repo
        .find_task(task_id)?
        .ok_or_else(|| anyhow::anyhow!("internal: resolved task #{task_id} not found"))?;

    if task.completed_at.is_some() {
        println!("Task #{task_id} is done. Create a new task with `bl start \"<title>\"`.");
        return Ok(1);
    }
    if task.archived_at.is_some() {
        println!("Task #{task_id} is archived. Create a new task with `bl start \"<title>\"`.");
        return Ok(1);
    }

    let now = chrono::Utc::now();
    let entry = TimerOps::new(&mut ctx.repo).start(task.id, now)?;
    println!(
        "Started: #{} {} ({})",
        task.id,
        task.title,
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
            let task = ctx.repo.find_task(entry.task_id)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "internal: time entry #{} references missing task #{}",
                    entry.id,
                    entry.task_id
                )
            })?;
            let elapsed = entry.duration(now);
            println!(
                "Stopped: #{} {} ({})",
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
            let task = ctx.repo.find_task(entry.task_id)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "internal: time entry #{} references missing task #{}",
                    entry.id,
                    entry.task_id
                )
            })?;
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
    let before = ctx.repo.find_task(id)?;
    let was_already_done = before
        .as_ref()
        .map(|t| t.completed_at.is_some())
        .unwrap_or(false);

    match ctx.repo.mark_task_done(id, chrono::Utc::now()) {
        Ok(t) => {
            if was_already_done {
                println!("Task #{} was already done.", t.id);
            } else {
                println!("Done: #{} {}", t.id, t.title);
            }
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
    let before = ctx.repo.find_task(id)?;
    let was_already_archived = before
        .as_ref()
        .map(|t| t.archived_at.is_some())
        .unwrap_or(false);

    match ctx.repo.archive_task(id, chrono::Utc::now()) {
        Ok(t) => {
            if was_already_archived {
                println!("Task #{} was already archived.", t.id);
            } else {
                println!("Archived: #{} {}", t.id, t.title);
            }
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
