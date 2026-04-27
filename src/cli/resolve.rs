//! Resolve a `bl start <target>` argument to a task id.
//!
//! Order of attempts (first match wins):
//!   1. Digits-only that match an existing `tasks.id`.
//!   2. `SC-NNN` (always) or digits-only (only if path 1 did not match),
//!      mapped to a Shortcut story. If a task already links to that story,
//!      use it. Otherwise fetch the story, create a task, link it.
//!   3. Free text — create a new task.

use crate::domain::Task;
use crate::shortcut::{self, Fetcher, FetcherError, IdError, ShortcutError};
use crate::storage::{Repo, SqliteRepo};

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("start target cannot be empty")]
    EmptyTarget,
    #[error("task id must be positive")]
    NonPositiveId,
    #[error("task #{0} not found")]
    TaskNotFound(i64),
    #[error("invalid shortcut id: {0}")]
    InvalidShortcutId(String),
    #[error("shortcut.token is not configured in config.toml")]
    MissingToken,
    #[error("shortcut story SC-{0} not found")]
    ShortcutNotFound(i64),
    #[error("shortcut auth failed: {0}. Check shortcut.token.")]
    ShortcutAuth(String),
    #[error("shortcut fetch failed: {0}")]
    Shortcut(#[from] FetcherError),
    #[error(transparent)]
    Repo(#[from] crate::storage::RepoError),
}

#[derive(Debug)]
pub enum Resolved {
    Existing(Task),
    Created(Task),
}

pub fn resolve_start_target(
    repo: &mut SqliteRepo,
    fetcher: Option<&Fetcher>,
    target: &str,
) -> Result<Resolved, ResolveError> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err(ResolveError::EmptyTarget);
    }

    // Path 1: bare digits → existing task id.
    let looks_bare_numeric = trimmed.chars().all(|c| c.is_ascii_digit());
    if looks_bare_numeric {
        let id: i64 = trimmed.parse().map_err(|_| {
            // unreachable in practice: all-digits string must parse as i64
            // unless it overflows, which we treat as "not a task id, try story."
            ResolveError::InvalidShortcutId(trimmed.to_string())
        })?;
        if id <= 0 {
            return Err(ResolveError::NonPositiveId);
        }
        if let Some(task) = repo.find_task(id)? {
            return Ok(Resolved::Existing(task));
        }
        // Fall through to path 2 with the same digits interpreted as SC id.
    }

    // Path 2: `SC-NNN` always, or bare digits when no task id matched.
    let is_sc_prefixed = trimmed.to_ascii_uppercase().starts_with("SC-");
    if is_sc_prefixed || looks_bare_numeric {
        let external_id = shortcut::normalize(trimmed).map_err(|e| match e {
            IdError::Empty => ResolveError::EmptyTarget,
            IdError::NonPositive => ResolveError::NonPositiveId,
            IdError::NotDigits(s) => ResolveError::InvalidShortcutId(s),
        })?;

        if let Some(existing) = repo.find_task_by_story_external_id(external_id)? {
            return Ok(Resolved::Existing(existing));
        }

        let Some(fetcher) = fetcher else {
            return Err(ResolveError::MissingToken);
        };

        let cached = match fetcher.get(repo, external_id, chrono::Utc::now()) {
            Ok(c) => c,
            Err(FetcherError::Shortcut(ShortcutError::NotFound)) => {
                return Err(ResolveError::ShortcutNotFound(external_id));
            }
            Err(FetcherError::Shortcut(ShortcutError::Auth(msg))) => {
                return Err(ResolveError::ShortcutAuth(msg));
            }
            Err(e) => return Err(ResolveError::Shortcut(e)),
        };

        let title = cached
            .story
            .title
            .clone()
            .unwrap_or_else(|| format!("SC-{external_id}"));
        let task = repo.create_task(&title, None)?;
        let linked = repo.link_task_to_story(task.id, cached.story.id, chrono::Utc::now())?;
        return Ok(Resolved::Created(linked));
    }

    // Path 3: free text → new task.
    let task = repo.create_task(trimmed, None)?;
    Ok(Resolved::Created(task))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn numeric_target_hits_existing_task() {
        let mut repo = SqliteRepo::in_memory();
        let t = repo.create_task("existing", None).unwrap();
        match resolve_start_target(&mut repo, None, &t.id.to_string()).unwrap() {
            Resolved::Existing(found) => assert_eq!(found.id, t.id),
            Resolved::Created(_) => panic!("should have found existing"),
        }
    }

    #[test]
    fn text_target_creates_task() {
        let mut repo = SqliteRepo::in_memory();
        match resolve_start_target(&mut repo, None, "brand new thing").unwrap() {
            Resolved::Created(t) => assert_eq!(t.title, "brand new thing"),
            Resolved::Existing(_) => panic!("should have created"),
        }
    }

    #[test]
    fn empty_target_errors() {
        let mut repo = SqliteRepo::in_memory();
        assert!(matches!(
            resolve_start_target(&mut repo, None, "   "),
            Err(ResolveError::EmptyTarget)
        ));
    }

    #[test]
    fn sc_without_fetcher_missing_token() {
        let mut repo = SqliteRepo::in_memory();
        assert!(matches!(
            resolve_start_target(&mut repo, None, "SC-1"),
            Err(ResolveError::MissingToken)
        ));
    }

    #[test]
    fn sc_with_existing_linked_task_returns_it_without_fetcher() {
        use crate::shortcut::Story;
        let mut repo = SqliteRepo::in_memory();
        let row = repo
            .upsert_shortcut_story(
                &Story {
                    external_id: 9,
                    title: Some("cached".into()),
                    epic_id: None,
                    epic_name: None,
                    state: None,
                },
                Utc::now(),
            )
            .unwrap();
        let t = repo.create_task("cached", None).unwrap();
        repo.link_task_to_story(t.id, row.id, Utc::now()).unwrap();

        match resolve_start_target(&mut repo, None, "SC-9").unwrap() {
            Resolved::Existing(found) => assert_eq!(found.id, t.id),
            Resolved::Created(_) => panic!("should have reused linked task"),
        }
    }

    #[test]
    fn bare_numeric_prefers_task_id() {
        let mut repo = SqliteRepo::in_memory();
        let t = repo.create_task("task-one", None).unwrap();
        // No fetcher — if the resolver fell through to SC, it would error.
        match resolve_start_target(&mut repo, None, &t.id.to_string()).unwrap() {
            Resolved::Existing(found) => assert_eq!(found.id, t.id),
            Resolved::Created(_) => panic!("should have matched task id"),
        }
    }
}
