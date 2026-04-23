//! Resolve a `bl start <target>` argument to a task id, creating a new task
//! if the target is non-numeric free text.

use crate::domain::Task;
use crate::storage::{Repo, SqliteRepo};

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("start target cannot be empty")]
    EmptyTarget,
    #[error("task id must be positive")]
    NonPositiveId,
    #[error("task #{0} not found")]
    TaskNotFound(i64),
    #[error(transparent)]
    Repo(#[from] crate::storage::RepoError),
}

#[derive(Debug)]
pub enum Resolved {
    Existing(i64),
    Created(Task),
}

pub fn resolve_or_create(repo: &mut SqliteRepo, target: &str) -> Result<Resolved, ResolveError> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err(ResolveError::EmptyTarget);
    }
    if let Ok(id) = trimmed.parse::<i64>() {
        if id <= 0 {
            return Err(ResolveError::NonPositiveId);
        }
        return match repo.find_task(id)? {
            Some(t) => Ok(Resolved::Existing(t.id)),
            None => Err(ResolveError::TaskNotFound(id)),
        };
    }
    let task = repo.create_task(trimmed, None)?;
    Ok(Resolved::Created(task))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_target_hits_existing_task() {
        let mut repo = SqliteRepo::in_memory();
        let t = repo.create_task("existing", None).unwrap();
        match resolve_or_create(&mut repo, &t.id.to_string()).unwrap() {
            Resolved::Existing(id) => assert_eq!(id, t.id),
            Resolved::Created(_) => panic!("should have found existing"),
        }
    }

    #[test]
    fn numeric_missing_errors() {
        let mut repo = SqliteRepo::in_memory();
        let err = resolve_or_create(&mut repo, "999").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn text_target_creates_task() {
        let mut repo = SqliteRepo::in_memory();
        match resolve_or_create(&mut repo, "brand new thing").unwrap() {
            Resolved::Created(t) => assert_eq!(t.title, "brand new thing"),
            Resolved::Existing(_) => panic!("should have created"),
        }
    }

    #[test]
    fn empty_target_errors() {
        let mut repo = SqliteRepo::in_memory();
        assert!(resolve_or_create(&mut repo, "   ").is_err());
    }
}
