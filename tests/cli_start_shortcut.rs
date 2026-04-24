use assert_cmd::Command;
use mockito::{Server, ServerGuard};
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::fs;
use tempfile::TempDir;

struct Env {
    home: TempDir,
    config_dir: TempDir,
    mock: ServerGuard,
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
        .expect_at_most(1) // second call short-circuits via the linked-task lookup, never hitting the fetcher
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
    let env = Env::new_with_token("abc");
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
