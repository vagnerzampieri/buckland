use buckland::storage;

#[test]
fn migrations_create_expected_tables_and_indexes() {
    let conn = storage::open_memory().expect("open in-memory DB with migrations");

    // All three tables exist.
    let table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type='table' AND name IN ('tasks', 'time_entries', 'shortcut_stories')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(table_count, 3);

    // Partial unique index on active timer exists.
    let has_active_idx: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type='index' AND name='idx_single_active'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(has_active_idx, 1);
}

#[test]
fn partial_unique_index_rejects_two_active_entries() {
    let conn = storage::open_memory().unwrap();

    conn.execute(
        "INSERT INTO tasks (title, created_at, updated_at) \
         VALUES ('t', '2026-04-22 10:00:00', '2026-04-22 10:00:00')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO time_entries (task_id, started_at) VALUES (1, '2026-04-22 10:00:00')",
        [],
    )
    .unwrap();

    let second = conn.execute(
        "INSERT INTO time_entries (task_id, started_at) VALUES (1, '2026-04-22 10:01:00')",
        [],
    );

    assert!(
        second.is_err(),
        "expected UNIQUE constraint failure, got {:?}",
        second
    );
}
