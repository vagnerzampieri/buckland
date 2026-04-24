//! Cache-first access to Shortcut stories.
//!
//! See the phase plan §"Task 5" for the policy this implements.

use crate::domain::ShortcutStory;
use crate::shortcut::client::{Client, ShortcutError};
use crate::storage::{Repo, RepoError};
use chrono::{DateTime, Duration, Utc};

/// 1h cache TTL, per the spec.
pub const CACHE_TTL: Duration = Duration::hours(1);

#[derive(Debug, thiserror::Error)]
pub enum FetcherError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error(transparent)]
    Shortcut(#[from] ShortcutError),
}

/// Result wrapper that tells the caller whether the returned row came from a
/// stale-but-present cache after a failed refresh.
#[derive(Debug, Clone)]
pub struct Cached {
    pub story: ShortcutStory,
    pub is_stale: bool,
}

pub struct Fetcher {
    client: Client,
}

impl Fetcher {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Cache-first get. Returns cached row if fresh; otherwise refreshes.
    /// If the refresh fails but a (stale) cached row exists, returns that
    /// with `is_stale: true`. A 404 from upstream always propagates.
    pub fn get<R: Repo>(
        &self,
        repo: &mut R,
        external_id: i64,
        now: DateTime<Utc>,
    ) -> Result<Cached, FetcherError> {
        if let Some(row) = repo.find_shortcut_story_by_external_id(external_id)? {
            if now - row.fetched_at < CACHE_TTL {
                return Ok(Cached {
                    story: row,
                    is_stale: false,
                });
            }
            // Stale cache; try a refresh, fall back to the stale row on
            // transport failure.
            match self.client.fetch_story(external_id) {
                Ok(fresh) => {
                    let saved = repo.upsert_shortcut_story(&fresh, now)?;
                    Ok(Cached {
                        story: saved,
                        is_stale: false,
                    })
                }
                Err(ShortcutError::NotFound) => Err(ShortcutError::NotFound.into()),
                Err(_) => Ok(Cached {
                    story: row,
                    is_stale: true,
                }),
            }
        } else {
            let fresh = self.client.fetch_story(external_id)?;
            let saved = repo.upsert_shortcut_story(&fresh, now)?;
            Ok(Cached {
                story: saved,
                is_stale: false,
            })
        }
    }

    /// Force a refresh regardless of cache freshness. Used by `bl shortcut`.
    pub fn refresh<R: Repo>(
        &self,
        repo: &mut R,
        external_id: i64,
        now: DateTime<Utc>,
    ) -> Result<ShortcutStory, FetcherError> {
        let fresh = self.client.fetch_story(external_id)?;
        Ok(repo.upsert_shortcut_story(&fresh, now)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shortcut::story::Story;
    use crate::storage::SqliteRepo;
    use mockito::Server;

    fn story_body(id: i64, name: &str) -> String {
        format!(r#"{{"id":{id},"name":"{name}","workflow_state_id":500000001}}"#)
    }

    #[test]
    fn get_returns_fresh_cache_without_http() {
        // No mockito mock at all — if we call the network this test fails.
        let server = Server::new();
        let client = Client::new(server.url(), "tok");
        let fetcher = Fetcher::new(client);

        let mut repo = SqliteRepo::in_memory();
        let now = Utc::now();
        repo.upsert_shortcut_story(
            &Story {
                external_id: 10,
                title: Some("cached".into()),
                epic_name: None,
                state: None,
            },
            now - Duration::minutes(10),
        )
        .unwrap();

        let out = fetcher.get(&mut repo, 10, now).unwrap();
        assert!(!out.is_stale);
        assert_eq!(out.story.title.as_deref(), Some("cached"));
    }

    #[test]
    fn get_with_no_cache_calls_http_and_persists() {
        let mut server = Server::new();
        let _m = server
            .mock("GET", "/api/v3/stories/20")
            .with_status(200)
            .with_body(story_body(20, "brand new"))
            .create();
        let fetcher = Fetcher::new(Client::new(server.url(), "tok"));
        let mut repo = SqliteRepo::in_memory();
        let now = Utc::now();

        let out = fetcher.get(&mut repo, 20, now).unwrap();
        assert!(!out.is_stale);
        assert_eq!(out.story.title.as_deref(), Some("brand new"));
        assert!(repo
            .find_shortcut_story_by_external_id(20)
            .unwrap()
            .is_some());
    }

    #[test]
    fn get_with_stale_cache_refreshes_on_success() {
        let mut server = Server::new();
        let _m = server
            .mock("GET", "/api/v3/stories/30")
            .with_status(200)
            .with_body(story_body(30, "refreshed"))
            .create();
        let fetcher = Fetcher::new(Client::new(server.url(), "tok"));
        let mut repo = SqliteRepo::in_memory();
        let now = Utc::now();
        repo.upsert_shortcut_story(
            &Story {
                external_id: 30,
                title: Some("old".into()),
                epic_name: None,
                state: None,
            },
            now - Duration::hours(2),
        )
        .unwrap();

        let out = fetcher.get(&mut repo, 30, now).unwrap();
        assert!(!out.is_stale);
        assert_eq!(out.story.title.as_deref(), Some("refreshed"));
    }

    #[test]
    fn get_with_stale_cache_falls_back_on_transient_error() {
        let mut server = Server::new();
        let _m = server
            .mock("GET", "/api/v3/stories/40")
            .with_status(503)
            .create();
        let fetcher = Fetcher::new(Client::new(server.url(), "tok"));
        let mut repo = SqliteRepo::in_memory();
        let now = Utc::now();
        repo.upsert_shortcut_story(
            &Story {
                external_id: 40,
                title: Some("survivor".into()),
                epic_name: None,
                state: None,
            },
            now - Duration::hours(2),
        )
        .unwrap();

        let out = fetcher.get(&mut repo, 40, now).unwrap();
        assert!(out.is_stale);
        assert_eq!(out.story.title.as_deref(), Some("survivor"));
    }

    #[test]
    fn get_propagates_not_found_even_with_stale_cache() {
        let mut server = Server::new();
        let _m = server
            .mock("GET", "/api/v3/stories/50")
            .with_status(404)
            .create();
        let fetcher = Fetcher::new(Client::new(server.url(), "tok"));
        let mut repo = SqliteRepo::in_memory();
        let now = Utc::now();
        repo.upsert_shortcut_story(
            &Story {
                external_id: 50,
                title: Some("ghost".into()),
                epic_name: None,
                state: None,
            },
            now - Duration::hours(2),
        )
        .unwrap();

        match fetcher.get(&mut repo, 50, now).unwrap_err() {
            FetcherError::Shortcut(ShortcutError::NotFound) => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn get_propagates_error_when_no_cache() {
        let mut server = Server::new();
        let _m = server
            .mock("GET", "/api/v3/stories/60")
            .with_status(500)
            .create();
        let fetcher = Fetcher::new(Client::new(server.url(), "tok"));
        let mut repo = SqliteRepo::in_memory();
        let err = fetcher.get(&mut repo, 60, Utc::now()).unwrap_err();
        assert!(matches!(
            err,
            FetcherError::Shortcut(ShortcutError::Transient { status: 500 })
        ));
    }

    #[test]
    fn refresh_always_hits_http_and_upserts() {
        let mut server = Server::new();
        let _m = server
            .mock("GET", "/api/v3/stories/70")
            .with_status(200)
            .with_body(story_body(70, "forced"))
            .create();
        let fetcher = Fetcher::new(Client::new(server.url(), "tok"));
        let mut repo = SqliteRepo::in_memory();
        let now = Utc::now();
        // Fresh cache exists — refresh should still call HTTP.
        repo.upsert_shortcut_story(
            &Story {
                external_id: 70,
                title: Some("stale-on-purpose".into()),
                epic_name: None,
                state: None,
            },
            now - Duration::seconds(10),
        )
        .unwrap();

        let row = fetcher.refresh(&mut repo, 70, now).unwrap();
        assert_eq!(row.title.as_deref(), Some("forced"));
    }
}
