use assert_cmd::Command;
use chrono::{Duration, Local, TimeZone, Utc};
use mockito::{Server, ServerGuard};
use predicates::str::contains;
use rusqlite::params;
use std::fs;
use tempfile::TempDir;

struct Env {
    home: TempDir,
    config_dir: TempDir,
    mock: ServerGuard,
}

impl Env {
    fn new_with_token(token: &str) -> Self {
        let home = TempDir::new().unwrap();
        let config_dir = TempDir::new().unwrap();
        let mock = Server::new();
        let buckland_cfg = config_dir.path().join("buckland");
        fs::create_dir_all(&buckland_cfg).unwrap();
        fs::write(
            buckland_cfg.join("config.toml"),
            format!(
                "[shortcut]\ntoken = \"{token}\"\napi_base_url = \"{}\"\n",
                mock.url()
            ),
        )
        .unwrap();
        Self {
            home,
            config_dir,
            mock,
        }
    }

    fn bl(&self) -> Command {
        let mut cmd = Command::cargo_bin("bl").unwrap();
        cmd.env("BUCKLAND_HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.config_dir.path());
        cmd
    }

    fn seed_closed_entry(
        &self,
        task_id: i64,
        started_at_utc: chrono::DateTime<Utc>,
        ended_at_utc: chrono::DateTime<Utc>,
    ) {
        let db = self.home.path().join("buckland.db");
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute(
            "INSERT INTO time_entries (task_id, started_at, ended_at) VALUES (?1, ?2, ?3)",
            params![task_id, started_at_utc, ended_at_utc],
        )
        .unwrap();
    }
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
fn by_epic_groups_two_tasks_under_their_shared_epic() {
    let mut env = Env::new_with_token("abc");
    let _m_story1 = env
        .mock
        .mock("GET", "/api/v3/stories/1")
        .with_status(200)
        .with_body(r#"{"id":1,"name":"Story one","workflow_state_id":1,"epic_id":50}"#)
        .create();
    let _m_story2 = env
        .mock
        .mock("GET", "/api/v3/stories/2")
        .with_status(200)
        .with_body(r#"{"id":2,"name":"Story two","workflow_state_id":1,"epic_id":50}"#)
        .create();
    let _m_epic = env
        .mock
        .mock("GET", "/api/v3/epics/50")
        .with_status(200)
        .with_body(r#"{"id":50,"name":"Big initiative"}"#)
        .expect_at_least(1)
        .create();

    env.bl().args(["add", "x", "--sc", "1"]).assert().success();
    env.bl().args(["add", "y", "--sc", "2"]).assert().success();

    env.seed_closed_entry(
        1,
        local_today_at(9, 0),
        local_today_at(9, 0) + Duration::minutes(30),
    );
    env.seed_closed_entry(
        2,
        local_today_at(10, 0),
        local_today_at(10, 0) + Duration::minutes(45),
    );

    env.bl()
        .args(["report", "--by-epic"])
        .assert()
        .success()
        .stdout(contains("Big initiative"))
        .stdout(contains("1h 15m"));
}

#[test]
fn by_epic_collects_tasks_without_epic_under_no_epic() {
    let home = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .args(["add", "no link"])
        .assert()
        .success();

    let db = home.path().join("buckland.db");
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute(
        "INSERT INTO time_entries (task_id, started_at, ended_at) VALUES (?1, ?2, ?3)",
        params![
            1i64,
            local_today_at(9, 0),
            local_today_at(9, 0) + Duration::minutes(15)
        ],
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("bl").unwrap();
    cmd.env("BUCKLAND_HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .args(["report", "--by-epic"])
        .assert()
        .success()
        .stdout(contains("(no epic)"))
        .stdout(contains("15m"));
}
