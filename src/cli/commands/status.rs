use crate::cli::context::Context;
use crate::storage::Repo;

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
