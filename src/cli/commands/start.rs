use crate::cli::context::Context;
use crate::domain::TimerOps;

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
