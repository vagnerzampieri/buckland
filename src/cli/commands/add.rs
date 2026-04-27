use crate::cli::context::Context;
use crate::storage::Repo;

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
