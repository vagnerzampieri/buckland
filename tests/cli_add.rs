use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

fn bl_in(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path());
    cmd
}

#[test]
fn add_without_description_creates_task() {
    let home = TempDir::new().unwrap();

    bl_in(&home).args(["add", "fix login"]).assert().success();

    bl_in(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("fix login"));
}

#[test]
fn add_with_description_creates_task() {
    let home = TempDir::new().unwrap();

    bl_in(&home)
        .args(["add", "ship feature", "--description", "plus docs"])
        .assert()
        .success();

    bl_in(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("ship feature"));
}

#[test]
fn add_requires_title() {
    let home = TempDir::new().unwrap();
    bl_in(&home).args(["add"]).assert().failure();
}
