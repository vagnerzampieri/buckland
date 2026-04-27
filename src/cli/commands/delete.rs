use crate::cli::context::Context;
use crate::storage::{Repo, RepoError};

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
