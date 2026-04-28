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
        .stdout(contains("pause"))
        .stdout(contains("status"))
        .stdout(contains("done"))
        .stdout(contains("archive"))
        .stdout(contains("delete"))
        .stdout(contains("report"));
}

#[test]
fn report_help_lists_flags() {
    Command::cargo_bin("bl")
        .unwrap()
        .args(["report", "--help"])
        .assert()
        .success()
        .stdout(contains("--today"))
        .stdout(contains("--week"))
        .stdout(contains("--month"))
        .stdout(contains("--all"))
        .stdout(contains("--range"))
        .stdout(contains("--by-task"))
        .stdout(contains("--by-epic"))
        .stdout(contains("--by-day"))
        .stdout(contains("--json"));
}

#[test]
fn unknown_subcommand_fails() {
    Command::cargo_bin("bl")
        .unwrap()
        .arg("banana")
        .assert()
        .failure();
}

#[test]
fn tui_subcommand_listed_in_help() {
    Command::cargo_bin("bl")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("tui"));
}

#[test]
fn tui_help_describes_command() {
    Command::cargo_bin("bl")
        .unwrap()
        .args(["tui", "--help"])
        .assert()
        .success()
        .stdout(contains("Open the TUI"));
}
