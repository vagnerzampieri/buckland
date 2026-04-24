use assert_cmd::Command;
use mockito::{Server, ServerGuard};
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::fs;
use tempfile::TempDir;

fn bl(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path());
    cmd
}

fn seed(home: &TempDir) {
    bl(home).args(["add", "open task"]).assert().success();
    bl(home).args(["add", "to be done"]).assert().success();
    bl(home).args(["add", "to be archived"]).assert().success();
    bl(home).args(["done", "2"]).assert().success();
    bl(home).args(["archive", "3"]).assert().success();
}

#[test]
fn default_shows_only_open() {
    let home = TempDir::new().unwrap();
    seed(&home);
    bl(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("open task"))
        .stdout(contains("to be done").not())
        .stdout(contains("to be archived").not());
}

#[test]
fn completed_flag_shows_completed() {
    let home = TempDir::new().unwrap();
    seed(&home);
    bl(&home)
        .args(["list", "--completed"])
        .assert()
        .success()
        .stdout(contains("to be done"));
}

#[test]
fn archived_flag_shows_archived() {
    let home = TempDir::new().unwrap();
    seed(&home);
    bl(&home)
        .args(["list", "--archived"])
        .assert()
        .success()
        .stdout(contains("to be archived"));
}

#[test]
fn all_flag_shows_everything() {
    let home = TempDir::new().unwrap();
    seed(&home);
    bl(&home)
        .args(["list", "--all"])
        .assert()
        .success()
        .stdout(contains("open task"))
        .stdout(contains("to be done"))
        .stdout(contains("to be archived"));
}

struct ScEnv {
    home: TempDir,
    config_dir: TempDir,
    mock: ServerGuard,
}

impl ScEnv {
    fn new() -> Self {
        let home = TempDir::new().unwrap();
        let config_dir = TempDir::new().unwrap();
        let mock = Server::new();
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
