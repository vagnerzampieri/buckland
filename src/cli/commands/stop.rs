use crate::cli::context::Context;
use crate::domain::TimerOps;
use crate::storage::Repo;

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
