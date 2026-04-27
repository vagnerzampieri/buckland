use assert_cmd::Command;
use chrono::{Duration, Local, TimeZone, Utc};
use predicates::str::contains;
use rusqlite::params;
use tempfile::TempDir;

fn bl(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path());
    cmd
}

/// Insert a closed entry directly into the SQLite file. Used to seed entries
/// with chosen wall-clock timestamps without sleeping in tests.
fn seed_closed_entry(
    home: &TempDir,
    task_id: i64,
    started_at_utc: chrono::DateTime<Utc>,
    ended_at_utc: chrono::DateTime<Utc>,
) {
    let db = home.path().join("buckland.db");
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute(
        "INSERT INTO time_entries (task_id, started_at, ended_at) VALUES (?1, ?2, ?3)",
        params![task_id, started_at_utc, ended_at_utc],
    )
    .unwrap();
}

fn local_today_at(hour: u32, minute: u32) -> chrono::DateTime<Utc> {
    let local_today = Local::now().date_naive();
    let naive = local_today.and_hms_opt(hour, minute, 0).unwrap();
    Local
        .from_local_datetime(&naive)
        .single()
        .unwrap()
        .with_timezone(&Utc)
}

#[test]
fn empty_report_prints_message_and_exits_zero() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "noop"]).assert().success();
    bl(&home)
        .args(["report"])
        .assert()
        .success()
        .stdout(contains("No time tracked"));
}

#[test]
fn default_report_groups_by_task_today() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "alpha"]).assert().success();
    bl(&home).args(["add", "beta"]).assert().success();

    // Two entries totalling 90m on task #1; one entry of 30m on task #2.
    seed_closed_entry(
        &home,
        1,
        local_today_at(9, 0),
        local_today_at(9, 0) + Duration::minutes(60),
    );
    seed_closed_entry(
        &home,
        1,
        local_today_at(11, 0),
        local_today_at(11, 0) + Duration::minutes(30),
    );
    seed_closed_entry(
        &home,
        2,
        local_today_at(13, 0),
        local_today_at(13, 0) + Duration::minutes(30),
    );

    bl(&home)
        .args(["report"])
        .assert()
        .success()
        .stdout(contains("alpha"))
        .stdout(contains("beta"))
        .stdout(contains("1h 30m"))
        .stdout(contains("Total"));
}
