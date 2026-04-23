use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use tempfile::TempDir;

fn bl(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path());
    cmd
}

#[test]
fn done_marks_task_complete() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "finish me"]).assert().success();
    bl(&home)
        .args(["done", "1"])
        .assert()
        .success()
        .stdout(contains("Done"));
    // Default list hides completed.
    bl(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("finish me").not());
}

#[test]
fn archive_hides_from_default_list() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "shelve me"]).assert().success();
    bl(&home).args(["archive", "1"]).assert().success();
    bl(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("shelve me").not());
}

#[test]
fn delete_empty_task_works() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "mistaken"]).assert().success();
    bl(&home)
        .args(["delete", "1"])
        .assert()
        .success()
        .stdout(contains("Deleted"));
}

#[test]
fn delete_task_with_entries_is_blocked() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["start", "real work"]).assert().success();
    bl(&home).args(["stop"]).assert().success();
    bl(&home)
        .args(["delete", "1"])
        .assert()
        .code(1)
        .stdout(contains("archive"));
}

#[test]
fn done_unknown_id_fails() {
    let home = TempDir::new().unwrap();
    bl(&home)
        .args(["done", "42"])
        .assert()
        .code(1)
        .stdout(contains("not found"));
}

#[test]
fn done_is_idempotent() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "t"]).assert().success();
    bl(&home)
        .args(["done", "1"])
        .assert()
        .success()
        .stdout(contains("Done"));
    bl(&home)
        .args(["done", "1"])
        .assert()
        .code(0)
        .stdout(contains("already done"));
}

#[test]
fn archive_is_idempotent() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "t"]).assert().success();
    bl(&home)
        .args(["archive", "1"])
        .assert()
        .success()
        .stdout(contains("Archived"));
    bl(&home)
        .args(["archive", "1"])
        .assert()
        .code(0)
        .stdout(contains("already archived"));
}
