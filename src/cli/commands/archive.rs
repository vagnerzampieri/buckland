use crate::cli::context::Context;
use crate::storage::{Repo, RepoError};

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
