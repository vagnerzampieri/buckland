//! Integration tests for `bl report --copy` / `-c`.
//!
//! We avoid touching the real clipboard by shimming `wl-copy` on PATH:
//! a tempdir contains a wrapper shell script that records its stdin to
//! a known file. Setting `WAYLAND_DISPLAY=mock` makes `clipboard::detect_tool`
//! pick `wl-copy`, and the PATH override makes our shim run instead of
//! the real binary.

use assert_cmd::Command;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

fn write_shim(dir: &TempDir, name: &str, body: &str) {
    let path = dir.path().join(name);
    fs::write(&path, body).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn bl(home: &TempDir, path_dir: &TempDir) -> Command {
    let path = format!(
        "{}:{}",
        path_dir.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .env("WAYLAND_DISPLAY", "mock")
        .env_remove("DISPLAY")
        .env("PATH", path);
    cmd
}

#[test]
fn report_copy_pipes_one_liner_to_wl_copy() {
    let home = TempDir::new().unwrap();
    let path_dir = TempDir::new().unwrap();
    let captured = path_dir.path().join("wl-copy.in");

    let shim = format!(
        "#!/usr/bin/env bash\ncat > {}\n",
        shell_escape::unix::escape(captured.to_string_lossy()).into_owned()
    );
    write_shim(&path_dir, "wl-copy", &shim);

    bl(&home, &path_dir)
        .args(["add", "smoke"])
        .assert()
        .success();
    bl(&home, &path_dir)
        .args(["report", "--copy"])
        .assert()
        .success();

    let recorded = fs::read_to_string(&captured).expect("wl-copy shim should have run");
    assert!(!recorded.trim().is_empty(), "expected non-empty payload");
    assert!(
        recorded.to_lowercase().contains("today")
            || recorded.contains("rows")
            || recorded.contains("0h")
            || recorded.contains("No time"),
        "unexpected payload: {recorded:?}"
    );
}

#[test]
fn report_copy_with_json_pipes_json_body() {
    let home = TempDir::new().unwrap();
    let path_dir = TempDir::new().unwrap();
    let captured = path_dir.path().join("wl-copy.in");

    let shim = format!(
        "#!/usr/bin/env bash\ncat > {}\n",
        shell_escape::unix::escape(captured.to_string_lossy()).into_owned()
    );
    write_shim(&path_dir, "wl-copy", &shim);

    bl(&home, &path_dir)
        .args(["add", "smoke"])
        .assert()
        .success();
    bl(&home, &path_dir)
        .args(["report", "--copy", "--json"])
        .assert()
        .success();

    let recorded = fs::read_to_string(&captured).expect("wl-copy shim should have run");
    let trimmed = recorded.trim();
    assert!(
        trimmed.starts_with('{') && trimmed.ends_with('}'),
        "expected JSON object, got {trimmed:?}"
    );
}

#[test]
fn report_copy_without_display_server_prints_error_and_exits_nonzero() {
    let home = TempDir::new().unwrap();
    let path_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("DISPLAY")
        .env("PATH", path_dir.path()); // empty PATH so even xclip can't be found
    cmd.args(["add", "smoke"]).assert().success();

    let mut report = Command::cargo_bin("bl").unwrap();
    report
        .env("BUCKLAND_HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("DISPLAY")
        .env("PATH", path_dir.path());
    report
        .args(["report", "--copy"])
        .assert()
        .code(1)
        .stderr(predicates::str::contains("clipboard"));
}
