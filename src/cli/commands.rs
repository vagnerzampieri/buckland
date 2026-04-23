//! Handlers for each CLI subcommand.
//!
//! Each function returns `anyhow::Result<i32>` where the integer is the exit
//! code. 0 = success; 1 = logical failure; other codes reserved.

use crate::cli::context::Context;
use crate::cli::format::duration_compact;
use crate::storage::Repo;
use chrono::Utc;

pub fn add(ctx: &mut Context, title: &str, description: Option<&str>) -> anyhow::Result<i32> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        anyhow::bail!("title cannot be empty");
    }
    let task = ctx.repo.create_task(trimmed, description)?;
    println!("Added task #{} — {}", task.id, task.title);
    Ok(0)
}

pub fn list(
    ctx: &mut Context,
    _all: bool,
    _archived: bool,
    _completed: bool,
) -> anyhow::Result<i32> {
    let now = Utc::now();
    let tasks = ctx.repo.list_open_tasks()?;
    if tasks.is_empty() {
        println!("No open tasks. Use `bl add \"title\"` to create one.");
        return Ok(0);
    }
    for t in tasks {
        let total = ctx.repo.task_total_duration(t.id, now)?;
        println!("{:>4}  {}  ({})", t.id, t.title, duration_compact(total));
    }
    Ok(0)
}

pub fn start(_ctx: &mut Context, _target: &str) -> anyhow::Result<i32> {
    todo!("Task 10")
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
