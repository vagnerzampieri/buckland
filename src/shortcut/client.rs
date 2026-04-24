//! HTTP client for the Shortcut REST API.
//!
//! Only one endpoint is used in this phase: `GET /api/v3/stories/{id}`.
//! The base URL is injected to make tests deterministic (see mockito in
//! this module's tests).

use crate::shortcut::story::Story;
use serde::Deserialize;
use std::time::Duration;

/// Production Shortcut base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.app.shortcut.com";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, thiserror::Error)]
pub enum ShortcutError {
    #[error("shortcut auth failed: {0}")]
    Auth(String),
    #[error("shortcut story not found")]
    NotFound,
    #[error("shortcut rate-limited")]
    RateLimited { retry_after: Option<Duration> },
    #[error("shortcut transient error (status {status})")]
    Transient { status: u16 },
    #[error("shortcut request timed out")]
    Timeout,
    #[error("shortcut network error: {0}")]
    Network(String),
    #[error("shortcut malformed response: {0}")]
    MalformedResponse(String),
}

pub struct Client {
    base_url: String,
    token: String,
    http: reqwest::blocking::Client,
}

impl Client {
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        let http = reqwest::blocking::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("building reqwest blocking client should not fail");
        Self {
            base_url: base_url.into(),
            token: token.into(),
            http,
        }
    }

    pub fn fetch_story(&self, id: i64) -> Result<Story, ShortcutError> {
        let url = format!(
            "{}/api/v3/stories/{}",
            self.base_url.trim_end_matches('/'),
            id
        );
        let resp = self
            .http
            .get(&url)
            .header("Shortcut-Token", &self.token)
            .header("Accept", "application/json")
            .send()
            .map_err(map_transport_error)?;

        let status = resp.status();
        if status.is_success() {
            let payload: StoryPayload = resp
                .json()
                .map_err(|e| ShortcutError::MalformedResponse(e.to_string()))?;
            return Ok(payload.into_story());
        }

        match status.as_u16() {
            401 | 403 => Err(ShortcutError::Auth(format!(
                "token rejected (status {})",
                status.as_u16()
            ))),
            404 => Err(ShortcutError::NotFound),
            429 => Err(ShortcutError::RateLimited {
                retry_after: resp
                    .headers()
                    .get("Retry-After")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(Duration::from_secs),
            }),
            code => Err(ShortcutError::Transient { status: code }),
        }
    }
}

fn map_transport_error(e: reqwest::Error) -> ShortcutError {
    if e.is_timeout() {
        ShortcutError::Timeout
    } else {
        ShortcutError::Network(e.to_string())
    }
}

/// Shape we deserialize from `GET /stories/{id}`. Fields that the spec marks
/// as "sometimes present" are wrapped in `Option`. We only keep what we use.
#[derive(Debug, Deserialize)]
struct StoryPayload {
    id: i64,
    #[serde(default)]
    name: Option<String>,
    /// The workflow_state_id is always present; we don't bother turning it
    /// into a human-readable state in this phase (one extra round-trip per
    /// workspace). `Fetcher` logs a TODO comment referencing this choice.
    #[serde(default)]
    workflow_state_id: Option<i64>,
}

impl StoryPayload {
    fn into_story(self) -> Story {
        Story {
            external_id: self.id,
            title: self.name,
            // In v1 we do not resolve epics; Phase C adds epic_id fetching when needed.
            epic_name: None,
            state: self.workflow_state_id.map(|id| id.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    fn mocked(server: &Server, token: &str) -> Client {
        Client::new(server.url(), token)
    }

    #[test]
    fn success_returns_story() {
        let mut server = Server::new();
        let _m = server
            .mock("GET", "/api/v3/stories/123")
            .match_header("Shortcut-Token", "abc")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":123,"name":"Fix login","workflow_state_id":500000001,"epic_id":9}"#)
            .create();
        let client = mocked(&server, "abc");
        let story = client.fetch_story(123).unwrap();
        assert_eq!(story.external_id, 123);
        assert_eq!(story.title.as_deref(), Some("Fix login"));
        assert_eq!(story.state.as_deref(), Some("500000001"));
        assert_eq!(story.epic_name, None);
    }

    #[test]
    fn auth_401_maps_to_auth_error() {
        let mut server = Server::new();
        let _m = server
            .mock("GET", "/api/v3/stories/1")
            .with_status(401)
            .create();
        let client = mocked(&server, "wrong");
        match client.fetch_story(1).unwrap_err() {
            ShortcutError::Auth(_) => {}
            other => panic!("expected Auth, got {other:?}"),
        }
    }

    #[test]
    fn auth_403_maps_to_auth_error() {
        let mut server = Server::new();
        let _m = server
            .mock("GET", "/api/v3/stories/1")
            .with_status(403)
            .create();
        let client = mocked(&server, "wrong");
        assert!(matches!(
            client.fetch_story(1).unwrap_err(),
            ShortcutError::Auth(_)
        ));
    }

    #[test]
    fn not_found_404_maps_to_not_found() {
        let mut server = Server::new();
        let _m = server
            .mock("GET", "/api/v3/stories/999")
            .with_status(404)
            .create();
        let client = mocked(&server, "abc");
        assert!(matches!(
            client.fetch_story(999).unwrap_err(),
            ShortcutError::NotFound
        ));
    }

    #[test]
    fn rate_limit_captures_retry_after() {
        let mut server = Server::new();
        let _m = server
            .mock("GET", "/api/v3/stories/1")
            .with_status(429)
            .with_header("Retry-After", "7")
            .create();
        let client = mocked(&server, "abc");
        match client.fetch_story(1).unwrap_err() {
            ShortcutError::RateLimited { retry_after } => {
                assert_eq!(retry_after, Some(Duration::from_secs(7)));
            }
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[test]
    fn rate_limit_without_header_still_rate_limited() {
        let mut server = Server::new();
        let _m = server
            .mock("GET", "/api/v3/stories/1")
            .with_status(429)
            .create();
        let client = mocked(&server, "abc");
        match client.fetch_story(1).unwrap_err() {
            ShortcutError::RateLimited { retry_after } => assert_eq!(retry_after, None),
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[test]
    fn server_500_maps_to_transient() {
        let mut server = Server::new();
        let _m = server
            .mock("GET", "/api/v3/stories/1")
            .with_status(503)
            .create();
        let client = mocked(&server, "abc");
        match client.fetch_story(1).unwrap_err() {
            ShortcutError::Transient { status } => assert_eq!(status, 503),
            other => panic!("expected Transient, got {other:?}"),
        }
    }

    #[test]
    fn malformed_json_surfaces_malformed_response() {
        let mut server = Server::new();
        let _m = server
            .mock("GET", "/api/v3/stories/1")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{not json")
            .create();
        let client = mocked(&server, "abc");
        assert!(matches!(
            client.fetch_story(1).unwrap_err(),
            ShortcutError::MalformedResponse(_)
        ));
    }

    #[test]
    fn missing_fields_default_to_none() {
        let mut server = Server::new();
        let _m = server
            .mock("GET", "/api/v3/stories/1")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":1}"#)
            .create();
        let client = mocked(&server, "abc");
        let story = client.fetch_story(1).unwrap();
        assert_eq!(story.title, None);
        assert_eq!(story.state, None);
        assert_eq!(story.epic_name, None);
    }

    #[test]
    fn token_is_sent_as_shortcut_token_header() {
        let mut server = Server::new();
        // If the match fails mockito will 501; assert_header is the guard.
        let _m = server
            .mock("GET", "/api/v3/stories/1")
            .match_header("Shortcut-Token", "super-secret")
            .with_status(200)
            .with_body(r#"{"id":1}"#)
            .create();
        let client = mocked(&server, "super-secret");
        assert!(client.fetch_story(1).is_ok());
    }
}
