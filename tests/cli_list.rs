use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
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
#[ignore = "unlocks in Task 12 when done/archive commands land"]
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
#[ignore = "unlocks in Task 12 when done/archive commands land"]
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
#[ignore = "unlocks in Task 12 when done/archive commands land"]
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
#[ignore = "unlocks in Task 12 when done/archive commands land"]
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
