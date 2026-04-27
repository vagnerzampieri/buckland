//! Handlers for each CLI subcommand.
//!
//! Each function returns `anyhow::Result<i32>` where the integer is the exit
//! code. 0 = success; 1 = logical failure; other codes reserved.

use crate::cli::context::Context;
use crate::domain::{Task, TimerOps};
use crate::storage::{Repo, RepoError};

pub fn add(
    ctx: &mut Context,
    title: &str,
    description: Option<&str>,
    sc: Option<&str>,
) -> anyhow::Result<i32> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        anyhow::bail!("title cannot be empty");
    }
    let description = description.map(|s| s.trim()).filter(|s| !s.is_empty());

    let sc_link = match sc {
        Some(raw) => match prepare_sc_link(ctx, raw)? {
            ScLinkOutcome::Linked(link) => Some(link),
            ScLinkOutcome::Exit(code) => return Ok(code),
        },
        None => None,
    };

    let task = ctx.repo.create_task(trimmed, description)?;
    if let Some(link) = sc_link {
        let linked = ctx
            .repo
            .link_task_to_story(task.id, link.story_row_id, chrono::Utc::now())?;
        println!(
            "Added: #{} {} (SC-{})",
            linked.id, linked.title, link.external_id
        );
    } else {
        println!("Added: #{} {}", task.id, task.title);
    }
    Ok(0)
}

struct ScLink {
    story_row_id: i64,
    external_id: i64,
}

enum ScLinkOutcome {
    Linked(ScLink),
    Exit(i32),
}

/// Returns Linked(link) on success, Exit(code) on user-facing failures.
fn prepare_sc_link(ctx: &mut Context, raw: &str) -> anyhow::Result<ScLinkOutcome> {
    use crate::shortcut::{normalize, FetcherError, IdError, ShortcutError};

    let external_id = match normalize(raw) {
        Ok(n) => n,
        Err(IdError::Empty | IdError::NonPositive) | Err(IdError::NotDigits(_)) => {
            println!("invalid shortcut id: {raw}");
            return Ok(ScLinkOutcome::Exit(1));
        }
    };

    let Some(fetcher) = ctx.fetcher.as_ref() else {
        println!("shortcut.token is not configured in config.toml");
        return Ok(ScLinkOutcome::Exit(1));
    };

    match fetcher.get(&mut ctx.repo, external_id, chrono::Utc::now()) {
        Ok(cached) => Ok(ScLinkOutcome::Linked(ScLink {
            story_row_id: cached.story.id,
            external_id,
        })),
        Err(FetcherError::Shortcut(ShortcutError::NotFound)) => {
            println!("shortcut story SC-{external_id} not found");
            Ok(ScLinkOutcome::Exit(1))
        }
        Err(FetcherError::Shortcut(ShortcutError::Auth(msg))) => {
            println!("shortcut auth failed: {msg}. Check shortcut.token.");
            Ok(ScLinkOutcome::Exit(1))
        }
        Err(e) => {
            println!("shortcut fetch failed: {e}");
            Ok(ScLinkOutcome::Exit(1))
        }
    }
}

pub fn shortcut_refresh(ctx: &mut Context, raw: &str) -> anyhow::Result<i32> {
    use crate::shortcut::{normalize, FetcherError, IdError, ShortcutError};

    let external_id = match normalize(raw) {
        Ok(n) => n,
        Err(IdError::Empty | IdError::NonPositive) | Err(IdError::NotDigits(_)) => {
            println!("invalid shortcut id: {raw}");
            return Ok(1);
        }
    };

    let Some(fetcher) = ctx.fetcher.as_ref() else {
        println!("shortcut.token is not configured in config.toml");
        return Ok(1);
    };

    match fetcher.refresh(&mut ctx.repo, external_id, chrono::Utc::now()) {
        Ok(row) => {
            println!(
                "SC-{} {} — fetched_at {}",
                row.external_id,
                row.title.as_deref().unwrap_or("(no title)"),
                row.fetched_at
                    .with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M:%S"),
            );
            Ok(0)
        }
        Err(FetcherError::Shortcut(ShortcutError::NotFound)) => {
            println!("shortcut story SC-{external_id} not found");
            Ok(1)
        }
        Err(FetcherError::Shortcut(ShortcutError::Auth(msg))) => {
            println!("shortcut auth failed: {msg}. Check shortcut.token.");
            Ok(1)
        }
        Err(e) => {
            println!("shortcut refresh failed: {e}");
            Ok(1)
        }
    }
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
    use crate::cli::resolve::{resolve_start_target, ResolveError, Resolved};

    let resolved = match resolve_start_target(&mut ctx.repo, ctx.fetcher.as_ref(), target) {
        Ok(r) => r,
        Err(ResolveError::Repo(e)) => return Err(e.into()),
        Err(e) => {
            // Every other ResolveError variant carries a user-ready message
            // via Display; print it and exit 1.
            println!("{e}");
            return Ok(1);
        }
    };

    let task = match resolved {
        Resolved::Existing(t) => t,
        Resolved::Created(t) => t,
    };

    if task.completed_at.is_some() {
        println!(
            "Task #{} is done. Create a new task with `bl start \"<title>\"`.",
            task.id
        );
        return Ok(1);
    }
    if task.archived_at.is_some() {
        println!(
            "Task #{} is archived. Create a new task with `bl start \"<title>\"`.",
            task.id
        );
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

pub struct ReportArgs {
    pub today: bool,
    pub week: bool,
    pub month: bool,
    pub all: bool,
    pub range: Option<String>,
    pub by_task: bool,
    pub by_epic: bool,
    pub by_day: bool,
    pub json: bool,
}

pub fn report(_ctx: &mut Context, _args: ReportArgs) -> anyhow::Result<i32> {
    // Stub — fully implemented in Tasks 4–11.
    println!("report: not yet implemented");
    Ok(0)
}
