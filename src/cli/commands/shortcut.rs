use crate::cli::context::Context;

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
