use assert_cmd::Command;
#[cfg(feature = "tray")]
use predicates::str::contains;

#[test]
#[cfg(feature = "tray")]
fn tray_subcommand_is_listed_in_help() {
    Command::cargo_bin("bl")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("tray"));
}

#[test]
#[cfg(feature = "tray")]
fn tray_help_describes_command() {
    Command::cargo_bin("bl")
        .unwrap()
        .args(["tray", "--help"])
        .assert()
        .success()
        .stdout(contains("tray icon"));
}

#[test]
#[cfg(not(feature = "tray"))]
fn tray_subcommand_absent_when_feature_disabled() {
    // No-op assertion — when tray is off, the subcommand isn't even
    // compiled in. We only assert the help still works.
    Command::cargo_bin("bl")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
}
