use assert_cmd::Command;
use mockito::{Server, ServerGuard};
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
