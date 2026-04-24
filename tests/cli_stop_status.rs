use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

fn bl(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path());
    cmd
}

#[test]
fn status_idle_exits_one() {
    let home = TempDir::new().unwrap();
    bl(&home)
        .args(["status"])
        .assert()
        .code(1)
        .stdout(contains("No active"));
}

#[test]
fn status_active_exits_zero() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["start", "do thing"]).assert().success();
    bl(&home)
        .args(["status"])
        .assert()
        .code(0)
        .stdout(contains("do thing"));
}

#[test]
fn stop_ends_the_active_entry() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["start", "do thing"]).assert().success();
    bl(&home)
        .args(["stop"])
        .assert()
        .code(0)
        .stdout(contains("Stopped"));
    bl(&home).args(["status"]).assert().code(1);
}

#[test]
fn stop_when_idle_exits_one() {
    let home = TempDir::new().unwrap();
    bl(&home)
        .args(["stop"])
        .assert()
        .code(1)
        .stdout(contains("Nothing to stop"));
}

#[test]
fn pause_is_alias_for_stop() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["start", "do thing"]).assert().success();
    bl(&home)
        .args(["pause"])
        .assert()
        .code(0)
        .stdout(contains("Stopped"));
}
