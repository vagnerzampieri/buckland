use assert_cmd::Command;
use chrono::{Duration, Local, TimeZone, Utc};
use rusqlite::params;
use serde_json::Value;
use tempfile::TempDir;

fn bl(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path());
    cmd
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

fn seed_closed_entry(
    home: &TempDir,
    task_id: i64,
    started: chrono::DateTime<Utc>,
    ended: chrono::DateTime<Utc>,
) {
    let db = home.path().join("buckland.db");
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute(
        "INSERT INTO time_entries (task_id, started_at, ended_at) VALUES (?1, ?2, ?3)",
        params![task_id, started, ended],
    )
    .unwrap();
}

#[test]
fn json_emits_a_valid_object_with_required_keys() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "alpha"]).assert().success();
    seed_closed_entry(
        &home,
        1,
        local_today_at(9, 0),
        local_today_at(9, 0) + Duration::minutes(45),
    );

    let output = bl(&home)
        .args(["report", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let body = String::from_utf8(output).unwrap();
    let v: Value = serde_json::from_str(&body).expect("output must parse as JSON");

    assert_eq!(v["scope"]["kind"], "today");
    assert!(v["scope"]["from"].is_string());
    assert!(v["scope"]["to"].is_string());
    assert_eq!(v["grouping"], "task");
    assert_eq!(v["total_seconds"], 45 * 60);
    assert_eq!(v["rows"].as_array().unwrap().len(), 1);
    let row = &v["rows"][0];
    assert_eq!(row["task_id"], 1);
    assert_eq!(row["duration_seconds"], 45 * 60);
    assert_eq!(row["label"], "alpha");
    // No date field on a by-task row.
    assert!(row.get("date").is_none());
    // No shortcut_external_id on an unlinked task.
    assert!(row.get("shortcut_external_id").is_none());
}

#[test]
fn json_empty_report_still_returns_object_and_zero() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "empty"]).assert().success();
    let output = bl(&home)
        .args(["report", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let body = String::from_utf8(output).unwrap();
    let v: Value = serde_json::from_str(&body).expect("output must parse as JSON");
    assert_eq!(v["total_seconds"], 0);
    assert_eq!(v["rows"].as_array().unwrap().len(), 0);
}
