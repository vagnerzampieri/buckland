use assert_cmd::Command;
use chrono::{Datelike, Duration, Local, TimeZone, Utc};
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

#[test]
fn week_scope_includes_an_entry_from_earlier_this_week() {
    let home = TempDir::new().unwrap();
    bl(&home)
        .args(["add", "earlier this week"])
        .assert()
        .success();

    // Seed an entry at Monday 09:00 of the current week. This is always
    // within the ISO week window regardless of what day of the week today is.
    let local_today = Local::now().date_naive();
    let days_since_monday = local_today.weekday().num_days_from_monday() as i64;
    let monday = local_today - Duration::days(days_since_monday);
    let monday_9am = Local
        .from_local_datetime(&monday.and_hms_opt(9, 0, 0).unwrap())
        .single()
        .unwrap()
        .with_timezone(&Utc);
    seed_closed_entry(&home, 1, monday_9am, monday_9am + Duration::minutes(45));

    bl(&home)
        .args(["report", "--week"])
        .assert()
        .success()
        .stdout(contains("earlier this week"))
        .stdout(contains("45m"));
}

#[test]
fn month_scope_works() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "anytime"]).assert().success();
    seed_closed_entry(
        &home,
        1,
        local_today_at(9, 0) - Duration::days(2),
        local_today_at(9, 0) - Duration::days(2) + Duration::minutes(20),
    );

    bl(&home)
        .args(["report", "--month"])
        .assert()
        .success()
        .stdout(contains("anytime"));
}

#[test]
fn all_scope_includes_very_old_entries() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "ancient"]).assert().success();
    let long_ago = Utc.with_ymd_and_hms(2020, 1, 1, 9, 0, 0).unwrap();
    seed_closed_entry(&home, 1, long_ago, long_ago + Duration::minutes(10));

    bl(&home)
        .args(["report", "--all"])
        .assert()
        .success()
        .stdout(contains("ancient"));
}

#[test]
fn range_scope_filters_to_the_window() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "in window"]).assert().success();
    bl(&home).args(["add", "out window"]).assert().success();

    let in_window = Utc.with_ymd_and_hms(2026, 3, 15, 10, 0, 0).unwrap();
    let out_window = Utc.with_ymd_and_hms(2026, 4, 15, 10, 0, 0).unwrap();
    seed_closed_entry(&home, 1, in_window, in_window + Duration::minutes(30));
    seed_closed_entry(&home, 2, out_window, out_window + Duration::minutes(30));

    bl(&home)
        .args(["report", "--range", "2026-03-01..2026-03-31"])
        .assert()
        .success()
        .stdout(contains("in window"))
        .stdout(predicates::prelude::PredicateBooleanExt::not(contains(
            "out window",
        )));
}

#[test]
fn range_with_bad_shape_exits_one() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "noop"]).assert().success();
    bl(&home)
        .args(["report", "--range", "garbage"])
        .assert()
        .code(1)
        .stdout(contains("range"));
}

#[test]
fn scope_flags_are_mutually_exclusive() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "noop"]).assert().success();
    bl(&home)
        .args(["report", "--today", "--week"])
        .assert()
        .failure();
}

#[test]
fn by_day_groups_under_local_date_labels() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "anything"]).assert().success();
    seed_closed_entry(
        &home,
        1,
        local_today_at(10, 0),
        local_today_at(10, 0) + Duration::minutes(30),
    );

    let today_label = Local::now().date_naive().format("%Y-%m-%d").to_string();
    bl(&home)
        .args(["report", "--by-day"])
        .assert()
        .success()
        .stdout(contains(today_label));
}
