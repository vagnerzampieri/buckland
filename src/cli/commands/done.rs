use crate::cli::context::Context;
use crate::storage::{Repo, RepoError};

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
