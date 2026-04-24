# Buckland Phase B — Shortcut Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Read-only Shortcut integration. `bl add --sc SC-123` links a task to a Shortcut story; `bl start SC-123` resumes or creates a task linked to a story; `bl shortcut SC-123` force-refreshes the cache. Story metadata lives in the `shortcut_stories` table with a 1-hour TTL.

**Architecture:** A new layer `src/shortcut/` owns three concerns: (1) ID normalization at the boundary; (2) a thin `reqwest::blocking` client with typed `thiserror` variants per HTTP failure; (3) a `Fetcher` that composes the client with the SQLite cache (cache-first, stale-on-transport-error). The CLI's `Context` gains an optional `Fetcher` built only when `config.toml` supplies a token. Existing commands (`add`, `start`, `list`) extend their surfaces; `shortcut` is a new subcommand.

**Tech Stack:** `reqwest = "0.12"` with `blocking` + `rustls-tls` + `json` features; `serde_json` for response parsing; `url = "2"` for URL composition; `mockito = "1"` as the HTTP test double (sync — keeps tests free of a tokio runtime; the spec mentions wiremock "preferred" but is explicit that mockito is acceptable, and sync-only is the right trade here).

---

## What this phase delivers

CLI surface added or extended in this phase:

```
bl add <title> [--sc <ID>] [--description <text>]
    --sc accepts "SC-123", "sc-123", or "123". On success, fetches the
    story, caches it, and links the new task to it.

bl shortcut <SC-ID>
    Forces re-fetch of a story into the cache. Prints the refreshed
    metadata (external_id, title, epic, state, fetched_at).

bl start <target>
    Extended resolution order (in order; first match wins):
      1. numeric task id (matches tasks.id)
      2. "SC-NNN" (always treated as a shortcut story id)
      3. bare "NNN" that is not a task id (treated as a shortcut story id)
      4. free text → creates a new task with that title and starts it
    For paths 2 and 3: if a task already links to that story, resume it.
    If no task links to it, fetch the story, create a new task whose
    title is the story title, link it, and start the timer. Requires a
    configured token only when no linked task exists yet.

bl list [--all | --archived | --completed]
    Output now shows an "SC-id" column when at least one task in the
    listing has a linked story. When no task has a story, the column is
    omitted (keeps pre-Phase-B output on single-user setups that never
    use Shortcut).
```

## Required reading (load before executing the first task)

- **Spec:** `docs/superpowers/specs/2026-04-22-buckland-design.md` — especially §"Shortcut Integration" (Client, Cache policy, Token storage), §"Data Model" (shortcut_stories table), §"Command Grammar (CLI)" (verbs affected by this phase).
- **Project guidelines:** `CLAUDE.md` — especially the §"Rust Idioms First" block about `reqwest::blocking`, error handling with `thiserror`, and "mock only at HTTP and filesystem boundaries."
- **Prior phase:** `docs/superpowers/plans/2026-04-22-buckland-phase-a-cli-core.md` — to understand Repo trait, Context shape, and existing resolver in `src/cli/resolve.rs`.

## Preconditions

Before starting Task 1:

- [ ] `git status` on `main` is clean.
- [ ] Phase A is marked `done` in `docs/superpowers/plans/README.md`.
- [ ] `cargo test` passes (run it once to confirm green baseline).
- [ ] `cargo clippy --all-targets -- -D warnings` is clean.

## Postconditions (how to verify Phase B is done)

After the final task:

- [ ] `cargo test` green, `cargo clippy --all-targets -- -D warnings` clean, `cargo fmt --all --check` clean.
- [ ] No real HTTP calls in tests — only mockito.
- [ ] With no `shortcut.token` configured, `bl add "t"` (no `--sc`) and all existing Phase A commands still succeed unchanged.
- [ ] With no token configured, `bl add "t" --sc SC-1`, `bl shortcut SC-1`, and `bl start SC-1` (when no linked task exists) each print a clear "configure shortcut.token" error and exit 1 — never panic.
- [ ] With a token configured against a mockito server, the full smoke flow at the end of this document runs green.
- [ ] `docs/superpowers/plans/README.md` has Phase B marked `done (<date>)` and Phase C marked `ready` with the draft file path set (task 10 handles this).

## Architecture (in scope for this phase)

```
src/
├── shortcut/
│   ├── mod.rs          # pub use Client, Fetcher, Story, Id, errors
│   ├── id.rs           # pure normalization: &str -> Result<i64, IdError>
│   ├── client.rs       # HTTP client, typed ShortcutError
│   ├── story.rs        # Story DTO + From<Story> for a repo upsert record
│   └── fetcher.rs      # cache composition (Repo + Client)
├── storage/
│   └── repo.rs         # +Repo methods for shortcut_stories + task linking
├── cli/
│   ├── args.rs         # +--sc flag on add, +shortcut subcommand
│   ├── commands.rs     # +bl add --sc, +bl shortcut, +bl start SC-NNN
│   ├── context.rs      # +Context::fetcher: Option<Fetcher>
│   ├── format.rs       # +format for list's SC-id column
│   └── resolve.rs      # rewritten to return richer Resolved enum
└── config.rs           # unchanged (token already wired in Phase A)
```

## Tech stack (this phase)

| Concern | Choice | Why |
|---------|--------|-----|
| HTTP client | `reqwest::blocking` + `rustls-tls` + `json` | Spec decision. rustls avoids OpenSSL system dep. Sync fits the "one short-lived process" model. |
| JSON parsing | `serde_json` via `reqwest`'s `.json()` | Standard. No manual parsing. |
| HTTP test double | `mockito = "1"` | Synchronous — no tokio dep for sync-only code. Listed as acceptable in spec ("wiremock preferred OR mockito"). |
| Error enum | `thiserror` (already a dep) | Matches existing error style in `storage::RepoError`. |
| URL composition | `url = "2"` | Safe join of base + path; prevents accidental `//`. |
| Logging | `tracing` **not** added in this phase | Spec mentions "filter token from log formatter"; we have no logger yet. Defer until a logging phase lands. For now, release builds simply do not print request bodies. |

## Next phase

Phase C — `bl report` with scope (today/week/month/all), grouping (task/epic/day), Unicode block bars, and `--json`. Phase B's `tasks.shortcut_story_id` and `shortcut_stories.epic_name` become the data source for `--by-epic`.

---

## Task 1: Dependencies + `src/shortcut/` skeleton

**Files:**
- Modify: `Cargo.toml`
- Create: `src/shortcut/mod.rs`
- Create: `src/shortcut/story.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Add runtime and dev dependencies**

Edit `Cargo.toml`. In `[dependencies]`, append after the last entry:

```toml
reqwest = { version = "0.12", default-features = false, features = ["blocking", "rustls-tls", "json"] }
serde_json = "1"
url = "2"
```

In `[dev-dependencies]`, append:

```toml
mockito = "1"
```

- [ ] **Step 2: Sanity-build to fetch the new crates**

Run: `cargo build`
Expected: compiles clean. This also populates `Cargo.lock`.

- [ ] **Step 3: Create `src/shortcut/mod.rs`**

```rust
//! Read-only Shortcut API integration.
//!
//! Three concerns, each in its own submodule:
//! - `id`      — normalize human input ("SC-123", "sc-123", "123") to i64.
//! - `client`  — thin `reqwest::blocking` wrapper with typed errors.
//! - `fetcher` — composes the client with the `shortcut_stories` cache.

pub mod client;
pub mod fetcher;
pub mod id;
pub mod story;

pub use client::{Client, ShortcutError};
pub use fetcher::{Fetcher, FetcherError};
pub use id::{normalize, IdError};
pub use story::Story;
```

- [ ] **Step 4: Create `src/shortcut/story.rs` with the DTO**

```rust
//! Story — the fetched representation of a Shortcut story.
//!
//! This is the value returned by the HTTP client. The repo caches it into
//! the `shortcut_stories` table via [`crate::domain::ShortcutStory`].

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Story {
    pub external_id: i64,
    pub title: Option<String>,
    pub epic_name: Option<String>,
    pub state: Option<String>,
}
```

- [ ] **Step 5: Create empty placeholders for the other submodules**

Create `src/shortcut/id.rs`:

```rust
//! ID normalization — filled in in Task 2.
```

Create `src/shortcut/client.rs`:

```rust
//! HTTP client — filled in in Task 3.
```

Create `src/shortcut/fetcher.rs`:

```rust
//! Cache composition — filled in in Task 5.
```

This lets `mod.rs` compile clean today; each task then replaces a placeholder.

- [ ] **Step 6: Wire the module into `src/lib.rs`**

Edit `src/lib.rs`. Replace its contents with:

```rust
//! Buckland — personal time tracker core library.

pub mod cli;
pub mod config;
pub mod domain;
pub mod shortcut;
pub mod storage;
```

- [ ] **Step 7: Temporarily stub the public re-exports so mod.rs compiles**

Because tasks 2/3/5 provide the real symbols, `pub use client::{Client, ShortcutError}` etc. will not resolve yet. Replace the `pub use` block in `src/shortcut/mod.rs` with placeholders until the respective tasks land:

```rust
pub mod client;
pub mod fetcher;
pub mod id;
pub mod story;

pub use story::Story;
// Note: re-exports for `id::normalize`, `client::Client`,
// `fetcher::Fetcher`, and their error types are added in tasks 2, 3, and 5.
```

- [ ] **Step 8: Build to confirm the scaffold compiles**

Run: `cargo build`
Expected: green build.

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml Cargo.lock src/shortcut/ src/lib.rs
git commit -m "feat(shortcut): scaffold module + add reqwest/mockito deps"
```

---

## Task 2: `shortcut::id::normalize` — pure ID validation

**Files:**
- Modify: `src/shortcut/id.rs`
- Modify: `src/shortcut/mod.rs`

Normalizes any of `"SC-123"`, `"sc-123"`, `"123"`, `" 123 "` to `123_i64`. Rejects empty strings, non-digit bodies, and non-positive numbers. Pure function, no I/O.

- [ ] **Step 1: Write the failing unit tests in `src/shortcut/id.rs`**

Replace the placeholder file contents with:

```rust
//! Normalize human input to a Shortcut story external id.
//!
//! Accepts any of:
//!   "SC-123", "sc-123", "123", "  123  "
//!
//! Rejects empty strings, non-digit bodies, and non-positive numbers.

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IdError {
    #[error("shortcut id cannot be empty")]
    Empty,
    #[error("shortcut id must be positive")]
    NonPositive,
    #[error("shortcut id must be digits (optionally prefixed with SC-): {0}")]
    NotDigits(String),
}

pub fn normalize(raw: &str) -> Result<i64, IdError> {
    unimplemented!("written in step 3")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_bare_digits() {
        assert_eq!(normalize("123").unwrap(), 123);
    }

    #[test]
    fn strips_sc_prefix_uppercase() {
        assert_eq!(normalize("SC-123").unwrap(), 123);
    }

    #[test]
    fn strips_sc_prefix_lowercase() {
        assert_eq!(normalize("sc-123").unwrap(), 123);
    }

    #[test]
    fn strips_whitespace() {
        assert_eq!(normalize("   SC-123  ").unwrap(), 123);
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(normalize("").unwrap_err(), IdError::Empty);
        assert_eq!(normalize("   ").unwrap_err(), IdError::Empty);
    }

    #[test]
    fn rejects_zero() {
        assert_eq!(normalize("0").unwrap_err(), IdError::NonPositive);
        assert_eq!(normalize("SC-0").unwrap_err(), IdError::NonPositive);
    }

    #[test]
    fn rejects_negative() {
        assert_eq!(normalize("-5").unwrap_err(), IdError::NonPositive);
    }

    #[test]
    fn rejects_non_digits_body() {
        match normalize("SC-12a") {
            Err(IdError::NotDigits(s)) => assert_eq!(s, "SC-12a"),
            other => panic!("expected NotDigits, got {other:?}"),
        }
    }

    #[test]
    fn rejects_sc_alone() {
        match normalize("SC-") {
            Err(IdError::NotDigits(_)) => {}
            other => panic!("expected NotDigits, got {other:?}"),
        }
    }

    #[test]
    fn rejects_other_prefixes() {
        match normalize("ABC-123") {
            Err(IdError::NotDigits(_)) => {}
            other => panic!("expected NotDigits, got {other:?}"),
        }
    }
}
```

- [ ] **Step 2: Run the tests — expect compile failure**

Run: `cargo test --lib shortcut::id`
Expected: `unimplemented!` panic on every test (that's the red state we want).

- [ ] **Step 3: Implement `normalize`**

Replace the `unimplemented!` body with:

```rust
pub fn normalize(raw: &str) -> Result<i64, IdError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(IdError::Empty);
    }

    let body = match trimmed.strip_prefix("SC-").or_else(|| trimmed.strip_prefix("sc-")) {
        Some(rest) => rest,
        None => trimmed,
    };

    if body.is_empty() || !body.chars().all(|c| c.is_ascii_digit() || c == '-') {
        return Err(IdError::NotDigits(trimmed.to_string()));
    }

    let n: i64 = body
        .parse()
        .map_err(|_| IdError::NotDigits(trimmed.to_string()))?;

    if n <= 0 {
        return Err(IdError::NonPositive);
    }

    Ok(n)
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib shortcut::id`
Expected: all 9 tests pass.

- [ ] **Step 5: Re-export from `src/shortcut/mod.rs`**

Edit `src/shortcut/mod.rs`. Change the re-export block to:

```rust
pub use id::{normalize, IdError};
pub use story::Story;
// `Client`, `ShortcutError`, `Fetcher`, `FetcherError` are added in tasks 3 and 5.
```

- [ ] **Step 6: Build**

Run: `cargo build`
Expected: green.

- [ ] **Step 7: Commit**

```bash
git add src/shortcut/id.rs src/shortcut/mod.rs
git commit -m "feat(shortcut): normalize SC-NNN / NNN id input with typed errors"
```

---

## Task 3: `shortcut::Client::fetch_story`

**Files:**
- Modify: `src/shortcut/client.rs`
- Modify: `src/shortcut/mod.rs`

Thin blocking HTTP client. `fetch_story(id: i64) -> Result<Story, ShortcutError>`. Tested with `mockito` — covers 200, 401, 404, 429, 5xx, timeout, malformed JSON.

- [ ] **Step 1: Write the client, its errors, and the tests in one go**

Replace `src/shortcut/client.rs` with:

```rust
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
            code if (500..600).contains(&code) => Err(ShortcutError::Transient { status: code }),
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
    #[serde(default)]
    epic_id: Option<i64>,
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
            // In v1 we do not resolve epic_id -> epic_name (extra API call).
            // We keep the slot so Phase C's --by-epic grouping has a place
            // to deposit a real epic name once epics are fetched.
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
```

- [ ] **Step 2: Run the tests**

Run: `cargo test --lib shortcut::client`
Expected: all 10 tests pass.

- [ ] **Step 3: Update `src/shortcut/mod.rs` re-exports**

Edit to:

```rust
pub mod client;
pub mod fetcher;
pub mod id;
pub mod story;

pub use client::{Client, ShortcutError, DEFAULT_BASE_URL};
pub use id::{normalize, IdError};
pub use story::Story;
// `Fetcher` and `FetcherError` are added in task 5.
```

- [ ] **Step 4: Run the full test suite**

Run: `cargo test`
Expected: green.

- [ ] **Step 5: Commit**

```bash
git add src/shortcut/client.rs src/shortcut/mod.rs
git commit -m "feat(shortcut): blocking HTTP client with typed errors"
```

---

## Task 4: Repo methods for `shortcut_stories` + task linking

**Files:**
- Modify: `src/storage/repo.rs`

Add four methods to the `Repo` trait and implement them on `SqliteRepo`:
- `upsert_shortcut_story(story: &Story, fetched_at: DateTime<Utc>) -> RepoResult<ShortcutStory>` — insert or update keyed by `external_id`.
- `find_shortcut_story_by_external_id(external_id: i64) -> RepoResult<Option<ShortcutStory>>`.
- `link_task_to_story(task_id: i64, story_row_id: i64) -> RepoResult<Task>` — sets `tasks.shortcut_story_id`.
- `find_task_by_story_external_id(external_id: i64) -> RepoResult<Option<Task>>` — used by `bl start SC-NNN` to check "is there already a task for this story?"

- [ ] **Step 1: Write inline tests**

Add this test block at the bottom of `src/storage/repo.rs` (before the closing `}` of the existing `#[cfg(test)] mod tests`):

```rust
    #[test]
    fn upsert_shortcut_story_inserts_then_updates() {
        use crate::shortcut::Story;
        let mut r = repo();
        let s1 = Story {
            external_id: 42,
            title: Some("first".into()),
            epic_name: None,
            state: None,
        };
        let now1 = Utc::now();
        let row1 = r.upsert_shortcut_story(&s1, now1).unwrap();
        assert_eq!(row1.external_id, 42);
        assert_eq!(row1.title.as_deref(), Some("first"));

        let s2 = Story {
            external_id: 42,
            title: Some("second".into()),
            epic_name: Some("Epic X".into()),
            state: Some("backlog".into()),
        };
        let now2 = Utc::now();
        let row2 = r.upsert_shortcut_story(&s2, now2).unwrap();
        assert_eq!(row2.id, row1.id, "upsert must reuse the same PK");
        assert_eq!(row2.title.as_deref(), Some("second"));
        assert_eq!(row2.epic_name.as_deref(), Some("Epic X"));
    }

    #[test]
    fn find_shortcut_story_by_external_id_returns_none_when_absent() {
        let r = repo();
        assert!(r.find_shortcut_story_by_external_id(999).unwrap().is_none());
    }

    #[test]
    fn link_task_to_story_sets_shortcut_story_id() {
        use crate::shortcut::Story;
        let mut r = repo();
        let t = r.create_task("t", None).unwrap();
        let row = r
            .upsert_shortcut_story(
                &Story {
                    external_id: 7,
                    title: Some("story".into()),
                    epic_name: None,
                    state: None,
                },
                Utc::now(),
            )
            .unwrap();
        let linked = r.link_task_to_story(t.id, row.id).unwrap();
        assert_eq!(linked.shortcut_story_id, Some(row.id));
    }

    #[test]
    fn link_task_to_story_errors_on_missing_task() {
        use crate::shortcut::Story;
        let mut r = repo();
        let row = r
            .upsert_shortcut_story(
                &Story {
                    external_id: 1,
                    title: None,
                    epic_name: None,
                    state: None,
                },
                Utc::now(),
            )
            .unwrap();
        match r.link_task_to_story(999, row.id) {
            Err(RepoError::TaskNotFound(id)) => assert_eq!(id, 999),
            other => panic!("expected TaskNotFound, got {other:?}"),
        }
    }

    #[test]
    fn find_task_by_story_external_id_finds_linked_task() {
        use crate::shortcut::Story;
        let mut r = repo();
        let row = r
            .upsert_shortcut_story(
                &Story {
                    external_id: 88,
                    title: Some("s".into()),
                    epic_name: None,
                    state: None,
                },
                Utc::now(),
            )
            .unwrap();
        let t = r.create_task("t", None).unwrap();
        r.link_task_to_story(t.id, row.id).unwrap();
        let found = r.find_task_by_story_external_id(88).unwrap().unwrap();
        assert_eq!(found.id, t.id);
        assert_eq!(found.shortcut_story_id, Some(row.id));
    }

    #[test]
    fn find_task_by_story_external_id_none_when_story_absent() {
        let r = repo();
        assert!(r.find_task_by_story_external_id(404).unwrap().is_none());
    }
```

- [ ] **Step 2: Run tests — expect compile errors**

Run: `cargo test --lib storage::repo`
Expected: compile errors — methods not on the trait yet. That's the red state.

- [ ] **Step 3: Extend the `Repo` trait**

In `src/storage/repo.rs`, add to the top of the file:

```rust
use crate::domain::ShortcutStory;
use crate::shortcut::Story;
```

Add these method signatures to the `pub trait Repo` block (at the end, before the closing `}`):

```rust
    fn upsert_shortcut_story(
        &mut self,
        story: &Story,
        fetched_at: DateTime<Utc>,
    ) -> RepoResult<ShortcutStory>;
    fn find_shortcut_story_by_external_id(
        &self,
        external_id: i64,
    ) -> RepoResult<Option<ShortcutStory>>;
    fn link_task_to_story(&mut self, task_id: i64, story_row_id: i64) -> RepoResult<Task>;
    fn find_task_by_story_external_id(
        &self,
        external_id: i64,
    ) -> RepoResult<Option<Task>>;
```

- [ ] **Step 4: Implement the four methods on `SqliteRepo`**

Add this helper constant next to `TASK_COLS`:

```rust
const SHORTCUT_STORY_COLS: &str =
    "id, external_id, title, epic_name, state, fetched_at";
```

Add these methods inside `impl Repo for SqliteRepo` (anywhere after the existing methods):

```rust
    fn upsert_shortcut_story(
        &mut self,
        story: &Story,
        fetched_at: DateTime<Utc>,
    ) -> RepoResult<ShortcutStory> {
        self.conn.execute(
            "INSERT INTO shortcut_stories \
                 (external_id, title, epic_name, state, fetched_at) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(external_id) DO UPDATE SET \
                 title = excluded.title, \
                 epic_name = excluded.epic_name, \
                 state = excluded.state, \
                 fetched_at = excluded.fetched_at",
            params![
                story.external_id,
                story.title,
                story.epic_name,
                story.state,
                fetched_at,
            ],
        )?;
        self.conn
            .query_row(
                &format!(
                    "SELECT {SHORTCUT_STORY_COLS} FROM shortcut_stories \
                     WHERE external_id = ?1"
                ),
                [story.external_id],
                |row| ShortcutStory::try_from(row),
            )
            .map_err(RepoError::from)
    }

    fn find_shortcut_story_by_external_id(
        &self,
        external_id: i64,
    ) -> RepoResult<Option<ShortcutStory>> {
        self.conn
            .query_row(
                &format!(
                    "SELECT {SHORTCUT_STORY_COLS} FROM shortcut_stories \
                     WHERE external_id = ?1"
                ),
                [external_id],
                |row| ShortcutStory::try_from(row),
            )
            .optional()
            .map_err(RepoError::from)
    }

    fn link_task_to_story(&mut self, task_id: i64, story_row_id: i64) -> RepoResult<Task> {
        let updated = self.conn.execute(
            "UPDATE tasks SET shortcut_story_id = ?1, updated_at = ?2 \
             WHERE id = ?3",
            params![story_row_id, Utc::now(), task_id],
        )?;
        if updated == 0 {
            return Err(RepoError::TaskNotFound(task_id));
        }
        load_task(&self.conn, task_id)
    }

    fn find_task_by_story_external_id(
        &self,
        external_id: i64,
    ) -> RepoResult<Option<Task>> {
        self.conn
            .query_row(
                &format!(
                    "SELECT {TASK_COLS} FROM tasks \
                     WHERE shortcut_story_id = \
                         (SELECT id FROM shortcut_stories WHERE external_id = ?1) \
                     ORDER BY created_at DESC LIMIT 1"
                ),
                [external_id],
                |row| Task::try_from(row),
            )
            .optional()
            .map_err(RepoError::from)
    }
```

- [ ] **Step 5: Run the tests**

Run: `cargo test --lib storage::repo`
Expected: all tests pass, including the 6 new ones.

- [ ] **Step 6: Commit**

```bash
git add src/storage/repo.rs
git commit -m "feat(storage): upsert shortcut_stories and link tasks to them"
```

---

## Task 5: `shortcut::Fetcher` — cache + client composition

**Files:**
- Modify: `src/shortcut/fetcher.rs`
- Modify: `src/shortcut/mod.rs`

Composes `Client` with the Repo. Policy:

1. If a cached row exists and `fetched_at > now - 1h`, return it (no HTTP).
2. Otherwise call the client. On success, upsert, return.
3. On client error, if a cached row exists (even stale), return it and set an `is_stale: true` flag the CLI can surface. Otherwise propagate the error.
4. `NotFound` from the client is never covered by a stale fallback — if the upstream says the story is gone, we want the caller to know.

The fetcher exposes `refresh(id)` (force re-fetch, used by `bl shortcut <SC-ID>`) and `get(id)` (cache-first, used by `bl add --sc` and `bl start SC-NNN`).

- [ ] **Step 1: Replace `src/shortcut/fetcher.rs` with the Fetcher + tests**

```rust
//! Cache-first access to Shortcut stories.
//!
//! See the phase plan §"Task 5" for the policy this implements.

use crate::domain::ShortcutStory;
use crate::shortcut::client::{Client, ShortcutError};
use crate::shortcut::story::Story;
use crate::storage::{Repo, RepoError, SqliteRepo};
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
    pub fn get(
        &self,
        repo: &mut SqliteRepo,
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
    pub fn refresh(
        &self,
        repo: &mut SqliteRepo,
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
```

- [ ] **Step 2: Re-export from `src/shortcut/mod.rs`**

Change the re-export block to:

```rust
pub mod client;
pub mod fetcher;
pub mod id;
pub mod story;

pub use client::{Client, ShortcutError, DEFAULT_BASE_URL};
pub use fetcher::{Cached, Fetcher, FetcherError, CACHE_TTL};
pub use id::{normalize, IdError};
pub use story::Story;
```

- [ ] **Step 3: Run the tests**

Run: `cargo test --lib shortcut::fetcher`
Expected: all 7 tests pass.

- [ ] **Step 4: Full-suite sanity**

Run: `cargo test`
Expected: green.

- [ ] **Step 5: Commit**

```bash
git add src/shortcut/fetcher.rs src/shortcut/mod.rs
git commit -m "feat(shortcut): cache-first Fetcher with stale-on-error fallback"
```

---

## Task 6: `bl add --sc <ID>`

**Files:**
- Modify: `src/cli/args.rs`
- Modify: `src/cli/context.rs`
- Modify: `src/cli/commands.rs`
- Create: `tests/cli_shortcut_add.rs`

Wire `Fetcher` into `Context` (built only when a token is configured). Extend `bl add` to accept `--sc <ID>`. When present: normalize the id, fetch the story (via Fetcher), upsert the cache row, create the task, link it to the story. If the token is missing or the fetch fails, print a clear error and exit 1 without persisting a half-done task.

- [ ] **Step 1: Write the integration tests**

Create `tests/cli_shortcut_add.rs`:

```rust
use assert_cmd::Command;
use mockito::Server;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::fs;
use tempfile::TempDir;

struct Env {
    _home: TempDir,
    config_dir: TempDir,
    mock: Server,
}

impl Env {
    fn new_with_token(token: &str) -> Self {
        let _home = TempDir::new().unwrap();
        let config_dir = TempDir::new().unwrap();
        let mock = Server::new();
        let buckland_cfg = config_dir.path().join("buckland");
        fs::create_dir_all(&buckland_cfg).unwrap();
        fs::write(
            buckland_cfg.join("config.toml"),
            format!(
                "[shortcut]\ntoken = \"{token}\"\napi_base_url = \"{}\"\n",
                mock.url()
            ),
        )
        .unwrap();
        Self {
            _home,
            config_dir,
            mock,
        }
    }

    fn bl(&self) -> Command {
        let mut cmd = Command::cargo_bin("bl").unwrap();
        cmd.env("BUCKLAND_HOME", self._home.path())
            .env("XDG_CONFIG_HOME", self.config_dir.path());
        cmd
    }
}

#[test]
fn add_with_sc_fetches_and_links_story() {
    let mut env = Env::new_with_token("abc");
    let _m = env
        .mock
        .mock("GET", "/api/v3/stories/123")
        .match_header("Shortcut-Token", "abc")
        .with_status(200)
        .with_body(r#"{"id":123,"name":"Story title","workflow_state_id":500000001}"#)
        .create();

    env.bl()
        .args(["add", "my task", "--sc", "SC-123"])
        .assert()
        .success()
        .stdout(contains("SC-123"));

    env.bl()
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("my task"))
        .stdout(contains("SC-123"));
}

#[test]
fn add_with_sc_without_token_errors() {
    let home = TempDir::new().unwrap();
    let config_dir = TempDir::new().unwrap();
    // No config.toml written.
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path())
        .env("XDG_CONFIG_HOME", config_dir.path())
        .args(["add", "x", "--sc", "SC-1"])
        .assert()
        .code(1)
        .stdout(contains("shortcut.token"));
}

#[test]
fn add_with_sc_404_reports_and_does_not_create_task() {
    let mut env = Env::new_with_token("abc");
    let _m = env
        .mock
        .mock("GET", "/api/v3/stories/999")
        .with_status(404)
        .create();

    env.bl()
        .args(["add", "x", "--sc", "999"])
        .assert()
        .code(1)
        .stdout(contains("not found"));

    env.bl()
        .args(["list", "--all"])
        .assert()
        .success()
        .stdout(contains("x").not());
}

#[test]
fn add_with_malformed_sc_rejects_before_http() {
    let env = Env::new_with_token("abc");
    // No mock — any request would 501 mockito.
    env.bl()
        .args(["add", "x", "--sc", "ABC-1"])
        .assert()
        .code(1)
        .stdout(contains("shortcut id"));
}
```

- [ ] **Step 2: Add `api_base_url` to `ShortcutConfig`**

Integration tests point `bl` at mockito. `Fetcher` already takes a base URL via `Client::new`; we expose it through the existing `ShortcutConfig` struct.

Edit `src/config.rs`. Replace the `ShortcutConfig` struct with:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShortcutConfig {
    pub token: Option<String>,
    /// Override for tests and on-prem Shortcut deployments. Unset in prod.
    #[serde(default)]
    pub api_base_url: Option<String>,
}
```

Existing config tests still pass because `api_base_url` defaults to `None`. No migration needed — TOML is append-only additive.

- [ ] **Step 3: Add the `--sc` flag to `Commands::Add`**

Edit `src/cli/args.rs`. Replace the `Add` variant with:

```rust
    /// Add a new task.
    Add {
        /// Short, human-readable title shown in list views.
        title: String,
        /// Optional longer description attached to the task.
        #[arg(long)]
        description: Option<String>,
        /// Link the task to a Shortcut story. Accepts "SC-123", "sc-123", or "123".
        #[arg(long)]
        sc: Option<String>,
    },
```

Also add the `Shortcut` variant (used in Task 7 — add it now so args.rs stays stable):

```rust
    /// Force re-fetch of a Shortcut story into the local cache.
    Shortcut {
        /// Story id: "SC-123", "sc-123", or "123".
        id: String,
    },
```

- [ ] **Step 4: Extend `Context` with an optional `Fetcher`**

Edit `src/cli/context.rs`:

```rust
use crate::config;
use crate::shortcut::{Client, Fetcher, DEFAULT_BASE_URL};
use crate::storage::SqliteRepo;
use std::path::PathBuf;

pub struct Context {
    pub repo: SqliteRepo,
    pub db_path: PathBuf,
    pub fetcher: Option<Fetcher>,
}

pub fn open() -> anyhow::Result<Context> {
    let db_path = resolve_db_path();
    let conn = crate::storage::open(&db_path)?;
    let fetcher = build_fetcher()?;
    Ok(Context {
        repo: SqliteRepo::new(conn),
        db_path,
        fetcher,
    })
}

fn resolve_db_path() -> PathBuf {
    match std::env::var("BUCKLAND_HOME") {
        Ok(home) if !home.trim().is_empty() => PathBuf::from(home.trim()).join("buckland.db"),
        _ => config::db_path(),
    }
}

fn build_fetcher() -> anyhow::Result<Option<Fetcher>> {
    let cfg = config::load(&config::config_path())?;
    let Some(token) = cfg.shortcut.token else {
        return Ok(None);
    };
    let base = cfg
        .shortcut
        .api_base_url
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    Ok(Some(Fetcher::new(Client::new(base, token))))
}
```

- [ ] **Step 5: Update `src/cli/mod.rs` dispatch and `src/cli/commands.rs`**

Edit `src/cli/mod.rs`. Update the match arm for `Add` and add one for `Shortcut`:

```rust
        Commands::Add {
            title,
            description,
            sc,
        } => commands::add(ctx, &title, description.as_deref(), sc.as_deref()),
        Commands::Shortcut { id } => commands::shortcut_refresh(ctx, &id),
```

Edit `src/cli/commands.rs`. Replace the existing `add` with:

```rust
pub fn add(
    ctx: &mut Context,
    title: &str,
    description: Option<&str>,
    sc: Option<&str>,
) -> anyhow::Result<i32> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        anyhow::bail!("title cannot be empty");
    }
    let description = description.map(|s| s.trim()).filter(|s| !s.is_empty());

    let sc_link = match sc {
        Some(raw) => match prepare_sc_link(ctx, raw)? {
            Ok(link) => Some(link),
            Err(code) => return Ok(code),
        },
        None => None,
    };

    let task = ctx.repo.create_task(trimmed, description)?;
    if let Some(link) = sc_link {
        let linked = ctx.repo.link_task_to_story(task.id, link.story_row_id)?;
        println!(
            "Added: #{} {} (SC-{})",
            linked.id, linked.title, link.external_id
        );
    } else {
        println!("Added: #{} {}", task.id, task.title);
    }
    Ok(0)
}

struct ScLink {
    story_row_id: i64,
    external_id: i64,
}

/// Returns Ok(Ok(link)) on success, Ok(Err(exit_code)) on user-facing failures.
fn prepare_sc_link(ctx: &mut Context, raw: &str) -> anyhow::Result<Result<ScLink, i32>> {
    use crate::shortcut::{normalize, FetcherError, IdError, ShortcutError};

    let external_id = match normalize(raw) {
        Ok(n) => n,
        Err(IdError::Empty | IdError::NonPositive) | Err(IdError::NotDigits(_)) => {
            println!("invalid shortcut id: {raw}");
            return Ok(Err(1));
        }
    };

    let Some(fetcher) = ctx.fetcher.as_ref() else {
        println!("shortcut.token is not configured in config.toml");
        return Ok(Err(1));
    };

    match fetcher.get(&mut ctx.repo, external_id, chrono::Utc::now()) {
        Ok(cached) => Ok(Ok(ScLink {
            story_row_id: cached.story.id,
            external_id,
        })),
        Err(FetcherError::Shortcut(ShortcutError::NotFound)) => {
            println!("shortcut story SC-{external_id} not found");
            Ok(Err(1))
        }
        Err(FetcherError::Shortcut(ShortcutError::Auth(msg))) => {
            println!("shortcut auth failed: {msg}. Check shortcut.token.");
            Ok(Err(1))
        }
        Err(e) => {
            println!("shortcut fetch failed: {e}");
            Ok(Err(1))
        }
    }
}
```

(`shortcut_refresh` lands in Task 7.)

- [ ] **Step 6: Add a no-op stub for `shortcut_refresh` so the build stays green**

Append to `src/cli/commands.rs`:

```rust
pub fn shortcut_refresh(_ctx: &mut Context, _raw: &str) -> anyhow::Result<i32> {
    // Implemented in Task 7.
    anyhow::bail!("bl shortcut is not implemented yet")
}
```

- [ ] **Step 7: Run the tests**

Run: `cargo test --test cli_shortcut_add`
Expected: all 4 tests pass.

Run: `cargo test`
Expected: full suite green (existing tests unaffected).

- [ ] **Step 8: Lint**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 9: Commit**

```bash
git add src/cli/args.rs src/cli/context.rs src/cli/commands.rs src/cli/mod.rs src/config.rs tests/cli_shortcut_add.rs
git commit -m "feat(cli): bl add --sc links a task to a Shortcut story"
```

---

## Task 7: `bl shortcut <SC-ID>` — force refresh

**Files:**
- Modify: `src/cli/commands.rs`
- Create: `tests/cli_shortcut_refresh.rs`

Implements the `Shortcut` subcommand that was already declared in `args.rs` in Task 6. Forces a cache refresh via `Fetcher::refresh` and prints the new row.

- [ ] **Step 1: Write the integration tests**

Create `tests/cli_shortcut_refresh.rs`:

```rust
use assert_cmd::Command;
use mockito::Server;
use predicates::str::contains;
use std::fs;
use tempfile::TempDir;

struct Env {
    home: TempDir,
    config_dir: TempDir,
    mock: Server,
}

impl Env {
    fn new_with_token(token: &str) -> Self {
        let home = TempDir::new().unwrap();
        let config_dir = TempDir::new().unwrap();
        let mock = Server::new();
        let buckland_cfg = config_dir.path().join("buckland");
        fs::create_dir_all(&buckland_cfg).unwrap();
        fs::write(
            buckland_cfg.join("config.toml"),
            format!(
                "[shortcut]\ntoken = \"{token}\"\napi_base_url = \"{}\"\n",
                mock.url()
            ),
        )
        .unwrap();
        Self {
            home,
            config_dir,
            mock,
        }
    }

    fn bl(&self) -> Command {
        let mut cmd = Command::cargo_bin("bl").unwrap();
        cmd.env("BUCKLAND_HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.config_dir.path());
        cmd
    }
}

#[test]
fn shortcut_refresh_fetches_and_prints() {
    let mut env = Env::new_with_token("abc");
    let _m = env
        .mock
        .mock("GET", "/api/v3/stories/88")
        .with_status(200)
        .with_body(r#"{"id":88,"name":"Forced refresh","workflow_state_id":500000001}"#)
        .create();

    env.bl()
        .args(["shortcut", "SC-88"])
        .assert()
        .success()
        .stdout(contains("SC-88"))
        .stdout(contains("Forced refresh"));
}

#[test]
fn shortcut_refresh_without_token_errors() {
    let home = TempDir::new().unwrap();
    let config_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path())
        .env("XDG_CONFIG_HOME", config_dir.path())
        .args(["shortcut", "SC-1"])
        .assert()
        .code(1)
        .stdout(contains("shortcut.token"));
}

#[test]
fn shortcut_refresh_reports_not_found() {
    let mut env = Env::new_with_token("abc");
    let _m = env
        .mock
        .mock("GET", "/api/v3/stories/404")
        .with_status(404)
        .create();

    env.bl()
        .args(["shortcut", "404"])
        .assert()
        .code(1)
        .stdout(contains("not found"));
}
```

- [ ] **Step 2: Replace the stub with the real implementation**

Edit `src/cli/commands.rs`. Replace the `shortcut_refresh` stub with:

```rust
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
```

- [ ] **Step 3: Run the tests**

Run: `cargo test --test cli_shortcut_refresh`
Expected: all 3 tests pass.

Run: `cargo test`
Expected: full suite green.

- [ ] **Step 4: Commit**

```bash
git add src/cli/commands.rs tests/cli_shortcut_refresh.rs
git commit -m "feat(cli): bl shortcut <SC-ID> force-refreshes the story cache"
```

---

## Task 8: `bl start` resolves `SC-NNN` and bare numeric story ids

**Files:**
- Modify: `src/cli/resolve.rs`
- Modify: `src/cli/commands.rs`
- Create: `tests/cli_start_shortcut.rs`

Extend the start-target resolver. New resolution order:

1. Parse as digits-only: if there is an existing task with that id, use it (path 1).
2. Parse as a shortcut id (either `SC-NNN` form or bare digits-as-story-id): if a task already links to that story, use it. Otherwise fetch the story, create a task whose title is the story title, link it, use it.
3. Otherwise treat the target as free text and create a new task (unchanged from Phase A).

Important: a plain `123` hits path 1 first (task id). Only when no task with id 123 exists does it fall through to path 2 (story external_id 123). The `SC-` prefix jumps straight to path 2 — it never tries to match a task id.

- [ ] **Step 1: Write the integration tests**

Create `tests/cli_start_shortcut.rs`:

```rust
use assert_cmd::Command;
use mockito::Server;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::fs;
use tempfile::TempDir;

struct Env {
    home: TempDir,
    config_dir: TempDir,
    mock: Server,
}

impl Env {
    fn new_with_token(token: &str) -> Self {
        let home = TempDir::new().unwrap();
        let config_dir = TempDir::new().unwrap();
        let mock = Server::new();
        let buckland_cfg = config_dir.path().join("buckland");
        fs::create_dir_all(&buckland_cfg).unwrap();
        fs::write(
            buckland_cfg.join("config.toml"),
            format!(
                "[shortcut]\ntoken = \"{token}\"\napi_base_url = \"{}\"\n",
                mock.url()
            ),
        )
        .unwrap();
        Self {
            home,
            config_dir,
            mock,
        }
    }

    fn bl(&self) -> Command {
        let mut cmd = Command::cargo_bin("bl").unwrap();
        cmd.env("BUCKLAND_HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.config_dir.path());
        cmd
    }
}

#[test]
fn start_sc_without_existing_task_creates_and_links() {
    let mut env = Env::new_with_token("abc");
    let _m = env
        .mock
        .mock("GET", "/api/v3/stories/321")
        .with_status(200)
        .with_body(r#"{"id":321,"name":"Fix login flow","workflow_state_id":500000001}"#)
        .create();

    env.bl()
        .args(["start", "SC-321"])
        .assert()
        .success()
        .stdout(contains("Fix login flow"));

    env.bl()
        .args(["status"])
        .assert()
        .code(0)
        .stdout(contains("Fix login flow"));

    env.bl()
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("SC-321"));
}

#[test]
fn start_sc_with_existing_linked_task_resumes_without_duplicate() {
    let mut env = Env::new_with_token("abc");
    let _m = env
        .mock
        .mock("GET", "/api/v3/stories/42")
        .with_status(200)
        .expect_at_most(1) // only called the first time; cache serves the second
        .with_body(r#"{"id":42,"name":"The answer","workflow_state_id":500000001}"#)
        .create();

    env.bl().args(["start", "SC-42"]).assert().success();
    env.bl().args(["stop"]).assert().success();
    env.bl().args(["start", "SC-42"]).assert().success();

    // Exactly one task should exist linked to SC-42.
    env.bl()
        .args(["list", "--all"])
        .assert()
        .success()
        .stdout(contains("The answer"));

    // Sanity: listing should contain SC-42 exactly once across the rows.
    let out = env
        .bl()
        .args(["list", "--all"])
        .output()
        .unwrap()
        .stdout;
    let text = String::from_utf8(out).unwrap();
    assert_eq!(text.matches("SC-42").count(), 1, "got:\n{text}");
}

#[test]
fn bare_numeric_prefers_task_id_over_story_id() {
    let mut env = Env::new_with_token("abc");
    // Create a task with id=1 and title "direct".
    env.bl().args(["add", "direct"]).assert().success();
    // No mock registered — a call to HTTP would 501 mockito.

    env.bl()
        .args(["start", "1"])
        .assert()
        .success()
        .stdout(contains("direct"));
}

#[test]
fn bare_numeric_falls_through_to_story_when_no_task_id_matches() {
    let mut env = Env::new_with_token("abc");
    let _m = env
        .mock
        .mock("GET", "/api/v3/stories/555")
        .with_status(200)
        .with_body(r#"{"id":555,"name":"Via bare number","workflow_state_id":500000001}"#)
        .create();

    env.bl()
        .args(["start", "555"])
        .assert()
        .success()
        .stdout(contains("Via bare number"));
}

#[test]
fn start_sc_without_token_errors_when_task_not_yet_linked() {
    let home = TempDir::new().unwrap();
    let config_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path())
        .env("XDG_CONFIG_HOME", config_dir.path())
        .args(["start", "SC-1"])
        .assert()
        .code(1)
        .stdout(contains("shortcut.token"));
}

#[test]
fn start_sc_404_surfaces_and_creates_nothing() {
    let mut env = Env::new_with_token("abc");
    let _m = env
        .mock
        .mock("GET", "/api/v3/stories/777")
        .with_status(404)
        .create();

    env.bl()
        .args(["start", "SC-777"])
        .assert()
        .code(1)
        .stdout(contains("not found"));

    env.bl()
        .args(["list", "--all"])
        .assert()
        .success()
        .stdout(contains("777").not());
}
```

- [ ] **Step 2: Replace `src/cli/resolve.rs` with the extended resolver**

Replace the whole file:

```rust
//! Resolve a `bl start <target>` argument to a task id.
//!
//! Order of attempts (first match wins):
//!   1. Digits-only that match an existing `tasks.id`.
//!   2. `SC-NNN` (always) or digits-only (only if path 1 did not match),
//!      mapped to a Shortcut story. If a task already links to that story,
//!      use it. Otherwise fetch the story, create a task, link it.
//!   3. Free text — create a new task.

use crate::domain::Task;
use crate::shortcut::{self, Fetcher, FetcherError, IdError, ShortcutError};
use crate::storage::{Repo, SqliteRepo};

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("start target cannot be empty")]
    EmptyTarget,
    #[error("task id must be positive")]
    NonPositiveId,
    #[error("task #{0} not found")]
    TaskNotFound(i64),
    #[error("invalid shortcut id: {0}")]
    InvalidShortcutId(String),
    #[error("shortcut.token is not configured in config.toml")]
    MissingToken,
    #[error("shortcut story SC-{0} not found")]
    ShortcutNotFound(i64),
    #[error("shortcut auth failed: {0}. Check shortcut.token.")]
    ShortcutAuth(String),
    #[error("shortcut fetch failed: {0}")]
    Shortcut(#[from] FetcherError),
    #[error(transparent)]
    Repo(#[from] crate::storage::RepoError),
}

#[derive(Debug)]
pub enum Resolved {
    Existing(Task),
    Created(Task),
}

pub fn resolve_start_target(
    repo: &mut SqliteRepo,
    fetcher: Option<&Fetcher>,
    target: &str,
) -> Result<Resolved, ResolveError> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err(ResolveError::EmptyTarget);
    }

    // Path 1: bare digits → existing task id.
    let looks_bare_numeric = trimmed.chars().all(|c| c.is_ascii_digit());
    if looks_bare_numeric {
        let id: i64 = trimmed.parse().map_err(|_| {
            // unreachable in practice: all-digits string must parse as i64
            // unless it overflows, which we treat as "not a task id, try story."
            ResolveError::InvalidShortcutId(trimmed.to_string())
        })?;
        if id <= 0 {
            return Err(ResolveError::NonPositiveId);
        }
        if let Some(task) = repo.find_task(id)? {
            return Ok(Resolved::Existing(task));
        }
        // Fall through to path 2 with the same digits interpreted as SC id.
    }

    // Path 2: `SC-NNN` always, or bare digits when no task id matched.
    let is_sc_prefixed = trimmed.to_ascii_uppercase().starts_with("SC-");
    if is_sc_prefixed || looks_bare_numeric {
        let external_id = shortcut::normalize(trimmed).map_err(|e| match e {
            IdError::Empty => ResolveError::EmptyTarget,
            IdError::NonPositive => ResolveError::NonPositiveId,
            IdError::NotDigits(s) => ResolveError::InvalidShortcutId(s),
        })?;

        if let Some(existing) = repo.find_task_by_story_external_id(external_id)? {
            return Ok(Resolved::Existing(existing));
        }

        let Some(fetcher) = fetcher else {
            return Err(ResolveError::MissingToken);
        };

        let cached = match fetcher.get(repo, external_id, chrono::Utc::now()) {
            Ok(c) => c,
            Err(FetcherError::Shortcut(ShortcutError::NotFound)) => {
                return Err(ResolveError::ShortcutNotFound(external_id));
            }
            Err(FetcherError::Shortcut(ShortcutError::Auth(msg))) => {
                return Err(ResolveError::ShortcutAuth(msg));
            }
            Err(e) => return Err(ResolveError::Shortcut(e)),
        };

        let title = cached
            .story
            .title
            .clone()
            .unwrap_or_else(|| format!("SC-{external_id}"));
        let task = repo.create_task(&title, None)?;
        let linked = repo.link_task_to_story(task.id, cached.story.id)?;
        return Ok(Resolved::Created(linked));
    }

    // Path 3: free text → new task.
    let task = repo.create_task(trimmed, None)?;
    Ok(Resolved::Created(task))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn numeric_target_hits_existing_task() {
        let mut repo = SqliteRepo::in_memory();
        let t = repo.create_task("existing", None).unwrap();
        match resolve_start_target(&mut repo, None, &t.id.to_string()).unwrap() {
            Resolved::Existing(found) => assert_eq!(found.id, t.id),
            Resolved::Created(_) => panic!("should have found existing"),
        }
    }

    #[test]
    fn text_target_creates_task() {
        let mut repo = SqliteRepo::in_memory();
        match resolve_start_target(&mut repo, None, "brand new thing").unwrap() {
            Resolved::Created(t) => assert_eq!(t.title, "brand new thing"),
            Resolved::Existing(_) => panic!("should have created"),
        }
    }

    #[test]
    fn empty_target_errors() {
        let mut repo = SqliteRepo::in_memory();
        assert!(matches!(
            resolve_start_target(&mut repo, None, "   "),
            Err(ResolveError::EmptyTarget)
        ));
    }

    #[test]
    fn sc_without_fetcher_missing_token() {
        let mut repo = SqliteRepo::in_memory();
        assert!(matches!(
            resolve_start_target(&mut repo, None, "SC-1"),
            Err(ResolveError::MissingToken)
        ));
    }

    #[test]
    fn sc_with_existing_linked_task_returns_it_without_fetcher() {
        use crate::shortcut::Story;
        let mut repo = SqliteRepo::in_memory();
        let row = repo
            .upsert_shortcut_story(
                &Story {
                    external_id: 9,
                    title: Some("cached".into()),
                    epic_name: None,
                    state: None,
                },
                Utc::now(),
            )
            .unwrap();
        let t = repo.create_task("cached", None).unwrap();
        repo.link_task_to_story(t.id, row.id).unwrap();

        match resolve_start_target(&mut repo, None, "SC-9").unwrap() {
            Resolved::Existing(found) => assert_eq!(found.id, t.id),
            Resolved::Created(_) => panic!("should have reused linked task"),
        }
    }

    #[test]
    fn bare_numeric_prefers_task_id() {
        let mut repo = SqliteRepo::in_memory();
        let t = repo.create_task("task-one", None).unwrap();
        // No fetcher — if the resolver fell through to SC, it would error.
        match resolve_start_target(&mut repo, None, &t.id.to_string()).unwrap() {
            Resolved::Existing(found) => assert_eq!(found.id, t.id),
            Resolved::Created(_) => panic!("should have matched task id"),
        }
    }
}
```

- [ ] **Step 3: Update `src/cli/commands.rs` `start`**

Replace the existing `start` function with:

```rust
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
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test cli_start_shortcut`
Expected: all 6 tests pass.

Run: `cargo test`
Expected: full suite green. Existing `tests/cli_start.rs` tests should keep passing because the resolver still honors numeric task ids and free text unchanged.

- [ ] **Step 5: Lint**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add src/cli/resolve.rs src/cli/commands.rs tests/cli_start_shortcut.rs
git commit -m "feat(cli): bl start resolves SC-NNN (create-and-link if unknown)"
```

---

## Task 9: `bl list` surfaces SC-id when present

**Files:**
- Modify: `src/cli/commands.rs`
- Modify: `tests/cli_list.rs`

Add an SC-id column to `bl list` output when at least one task in the returned list has a linked story. When no task in the list has a story, the column is omitted so users who never touch Shortcut see unchanged output.

- [ ] **Step 1: Extend `tests/cli_list.rs`**

Append these tests at the bottom of `tests/cli_list.rs`:

```rust
use std::fs;

struct ScEnv {
    home: TempDir,
    config_dir: TempDir,
    mock: mockito::Server,
}

impl ScEnv {
    fn new() -> Self {
        let home = TempDir::new().unwrap();
        let config_dir = TempDir::new().unwrap();
        let mock = mockito::Server::new();
        let buckland_cfg = config_dir.path().join("buckland");
        fs::create_dir_all(&buckland_cfg).unwrap();
        fs::write(
            buckland_cfg.join("config.toml"),
            format!(
                "[shortcut]\ntoken = \"abc\"\napi_base_url = \"{}\"\n",
                mock.url()
            ),
        )
        .unwrap();
        Self {
            home,
            config_dir,
            mock,
        }
    }

    fn bl(&self) -> Command {
        let mut cmd = Command::cargo_bin("bl").unwrap();
        cmd.env("BUCKLAND_HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.config_dir.path());
        cmd
    }
}

#[test]
fn list_shows_sc_column_when_a_task_is_linked() {
    let mut env = ScEnv::new();
    let _m = env
        .mock
        .mock("GET", "/api/v3/stories/7")
        .with_status(200)
        .with_body(r#"{"id":7,"name":"linked","workflow_state_id":500000001}"#)
        .create();

    env.bl().args(["add", "plain"]).assert().success();
    env.bl()
        .args(["add", "linked", "--sc", "7"])
        .assert()
        .success();

    env.bl()
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("SC-7"))
        .stdout(contains("plain"))
        .stdout(contains("linked"));
}

#[test]
fn list_hides_sc_column_when_no_task_is_linked() {
    let home = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path())
        .args(["add", "only one"])
        .assert()
        .success();

    Command::cargo_bin("bl")
        .unwrap()
        .env("BUCKLAND_HOME", home.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("only one"))
        .stdout(contains("SC-").not());
}
```

Also, at the top of `tests/cli_list.rs`, add the mockito import (next to the existing uses):

```rust
use mockito; // already implicit via dev-dependency
```

(If `mockito` isn't already imported, add `use mockito;` — the rest of the file just uses its types qualified as `mockito::Server`.)

- [ ] **Step 2: Implement the conditional SC-id column**

Replace the existing `list` function in `src/cli/commands.rs` with:

```rust
pub fn list(ctx: &mut Context, all: bool, archived: bool, completed: bool) -> anyhow::Result<i32> {
    let now = chrono::Utc::now();
    let tasks: Vec<Task> = if all {
        ctx.repo.list_all_tasks()?
    } else if archived {
        ctx.repo.list_archived_tasks()?
    } else if completed {
        ctx.repo.list_completed_tasks()?
    } else {
        ctx.repo.list_open_tasks()?
    };

    if tasks.is_empty() {
        match (all, archived, completed) {
            (true, _, _) => println!("No tasks at all. Use `bl add \"title\"`."),
            (_, true, _) => println!("No archived tasks."),
            (_, _, true) => println!("No completed tasks."),
            _ => println!("No open tasks. Use `bl add \"title\"` to create one."),
        }
        return Ok(0);
    }

    // Collect SC-ids up front so the column-presence decision is one pass.
    let sc_ids: Vec<Option<i64>> = tasks
        .iter()
        .map(|t| match t.shortcut_story_id {
            Some(row_id) => external_id_for_row(ctx, row_id),
            None => Ok(None),
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let show_sc = sc_ids.iter().any(|v| v.is_some());

    for (t, sc) in tasks.iter().zip(sc_ids.iter()) {
        let total = ctx.repo.task_total_duration(t.id, now)?;
        let status = status_glyph(t);
        if show_sc {
            let sc_str = sc
                .map(|n| format!("SC-{n}"))
                .unwrap_or_else(|| "—".to_string());
            println!(
                "{status} {:>4}  {:<8}  {:<40}  {}",
                t.id,
                sc_str,
                truncate(&t.title, 40),
                crate::cli::format::duration_compact(total)
            );
        } else {
            println!(
                "{status} {:>4}  {:<40}  {}",
                t.id,
                truncate(&t.title, 40),
                crate::cli::format::duration_compact(total)
            );
        }
    }
    Ok(0)
}

fn external_id_for_row(ctx: &Context, row_id: i64) -> anyhow::Result<Option<i64>> {
    let conn = ctx.repo.connection();
    let id: Option<i64> = conn
        .query_row(
            "SELECT external_id FROM shortcut_stories WHERE id = ?1",
            [row_id],
            |row| row.get(0),
        )
        .ok();
    Ok(id)
}
```

(This relies on `SqliteRepo::connection()` which is already public per Phase A Task 4 step 2.)

- [ ] **Step 3: Run tests**

Run: `cargo test --test cli_list`
Expected: all tests pass, including the 2 new ones.

Run: `cargo test`
Expected: full suite green.

- [ ] **Step 4: Manual smoke check (optional but recommended)**

```bash
export BUCKLAND_HOME=/tmp/bl-phase-b-smoke
rm -rf "$BUCKLAND_HOME"
cargo run -- add "plain one"
cargo run -- list
# Expect: no SC-id column.
```

- [ ] **Step 5: Commit**

```bash
git add src/cli/commands.rs tests/cli_list.rs
git commit -m "feat(cli): list shows SC-id column when any task is linked"
```

---

## Task 10: Self-review, README update, and phase handoff

**Files:**
- Modify: `docs/superpowers/plans/README.md`

- [ ] **Step 1: Run the self-review checklist**

Top-to-bottom:

1. **Spec coverage.** Every CLI addition listed in "What this phase delivers" has an integration test:
   - `bl add --sc` → `tests/cli_shortcut_add.rs` ✅
   - `bl shortcut <SC-ID>` → `tests/cli_shortcut_refresh.rs` ✅
   - `bl start SC-NNN` → `tests/cli_start_shortcut.rs` ✅
   - `bl list` SC column → `tests/cli_list.rs` ✅
2. **Exit codes.** Missing token, invalid id, 404 all exit 1 and print a message.
3. **`cargo test` green.** `cargo clippy --all-targets -- -D warnings` clean. `cargo fmt --all --check` clean.
4. **No `todo!()`, `unimplemented!()`, or `dbg!()` in `src/`.** Run `grep -rn 'todo!\|unimplemented!\|dbg!' src/` — expected empty.
5. **`Cargo.lock` is committed.**
6. **Smoke flow below runs green.**

- [ ] **Step 2: Run the end-to-end smoke flow**

Pre-requisite: you need a Shortcut token with read access to at least one story, OR a local mockito server. For the manual smoke below, configure against the real Shortcut API using your own token (replace `<ID>` with a real story id in your workspace):

```bash
export BUCKLAND_HOME=/tmp/bl-phase-b-smoke
rm -rf "$BUCKLAND_HOME"

mkdir -p ~/.config/buckland
cat > ~/.config/buckland/config.toml <<EOF
[shortcut]
token = "$SHORTCUT_TOKEN"
EOF
chmod 600 ~/.config/buckland/config.toml

cargo run -- shortcut SC-<ID>            # prints fetched_at and title
cargo run -- add "manual verify" --sc <ID>
cargo run -- list                        # SC-id column appears
cargo run -- start SC-<ID>               # resumes the task we just added
cargo run -- status                      # exit 0
cargo run -- stop
```

If you do not have a token handy, the full integration-test suite (`cargo test`) is sufficient proof — it exercises every path against mockito.

- [ ] **Step 3: Update the phase index**

Edit `docs/superpowers/plans/README.md`. Change the Phase B row from `ready` to `done (<today's date>)`. Change the Phase C row to `ready` and set its file path to `2026-04-22-buckland-phase-c-report.md` (the file does not yet exist — it will be drafted before execution).

- [ ] **Step 4: Commit the index update**

```bash
git add docs/superpowers/plans/README.md
git commit -m "docs(plans): mark Phase B done, promote Phase C to ready"
```

---

## Phase B complete

At this point `bl` can tie tasks to Shortcut stories. Smoke flow:

```bash
# With config.toml set up with a token.
cargo run -- add "local" --sc SC-12345   # creates local task, caches story
cargo run -- list                          # SC-id column visible
cargo run -- start SC-99999                # creates a new task from the story
cargo run -- shortcut SC-12345             # forces cache refresh
```

## Self-review checklist for the executing engineer

Before handing the phase off, run this top-to-bottom:

1. **Every CLI addition has an integration test with at least one happy path.**
2. **Exit codes.** Missing token, invalid id, 404 return 1.
3. **`cargo test` green, `cargo clippy --all-targets -- -D warnings` clean, `cargo fmt --all --check` clean.**
4. **No `todo!()`, `unimplemented!()`, or `dbg!()` in `src/`.**
5. **`Cargo.lock` committed.**
6. **Token never leaks to stdout/stderr.** Grep the test output for any literal token string — the test token is `"abc"`, acceptable only because the tests themselves print it when needed.
7. **No network calls in tests.** `grep -rn "api.app.shortcut.com" tests/` should return empty (only mockito URLs are used).

## What's next

Phase C — `bl report` with scope (today/week/month/all), grouping (task/epic/day), Unicode block bars, `--json`. The `--by-epic` grouping will consume `shortcut_stories.epic_name`, which is left as `None` in this phase (`client.rs` has a comment explaining the deferred epic lookup). Phase C decides whether to resolve epics during fetch, during report render, or not at all for v1.
