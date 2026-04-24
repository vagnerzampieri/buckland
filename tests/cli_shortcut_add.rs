use assert_cmd::Command;
use mockito::{Server, ServerGuard};
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::fs;
use tempfile::TempDir;

struct Env {
    _home: TempDir,
    config_dir: TempDir,
    mock: ServerGuard,
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
