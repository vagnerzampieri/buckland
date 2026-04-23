use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn help_shows_subcommands() {
    Command::cargo_bin("bl")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("add"))
        .stdout(contains("list"))
        .stdout(contains("start"))
        .stdout(contains("stop"))
        .stdout(contains("status"));
}

#[test]
fn unknown_subcommand_fails() {
    Command::cargo_bin("bl")
        .unwrap()
        .arg("banana")
        .assert()
        .failure();
}
