use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

fn bl(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path());
    cmd
}

#[test]
fn start_by_numeric_id_works() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "existing"]).assert().success();
    bl(&home)
        .args(["start", "1"])
        .assert()
        .success()
        .stdout(contains("existing"));
    bl(&home).args(["status"]).assert().success();
}

#[test]
fn start_by_text_creates_and_starts() {
    let home = TempDir::new().unwrap();
    bl(&home)
        .args(["start", "new quick thing"])
        .assert()
        .success()
        .stdout(contains("new quick thing"));
    bl(&home).args(["status"]).assert().success();
}

#[test]
fn start_missing_numeric_errors() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["start", "999"]).assert().code(1);
}

#[test]
fn start_missing_id_exits_one_not_two() {
    let home = TempDir::new().unwrap();
    bl(&home)
        .args(["start", "999"])
        .assert()
        .code(1)
        .stdout(contains("not found"));
}

#[test]
fn start_on_completed_task_is_refused() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "t"]).assert().success();
    bl(&home).args(["done", "1"]).assert().success();
    bl(&home)
        .args(["start", "1"])
        .assert()
        .code(1)
        .stdout(contains("is done"));
}

#[test]
fn start_on_archived_task_is_refused() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "t"]).assert().success();
    bl(&home).args(["archive", "1"]).assert().success();
    bl(&home)
        .args(["start", "1"])
        .assert()
        .code(1)
        .stdout(contains("is archived"));
}

#[test]
fn start_switches_active_task() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "first"]).assert().success();
    bl(&home).args(["add", "second"]).assert().success();
    bl(&home).args(["start", "1"]).assert().success();
    bl(&home)
        .args(["start", "2"])
        .assert()
        .success()
        .stdout(contains("second"));
    // Only one entry should be active; verified indirectly via status printing "second".
    bl(&home)
        .args(["status"])
        .assert()
        .success()
        .stdout(contains("second"));
}
