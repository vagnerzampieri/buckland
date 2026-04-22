CREATE TABLE tasks (
  id                INTEGER PRIMARY KEY,
  title             TEXT NOT NULL,
  description       TEXT,
  shortcut_story_id INTEGER REFERENCES shortcut_stories(id),
  completed_at      DATETIME,
  archived_at       DATETIME,
  created_at        DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at        DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE time_entries (
  id          INTEGER PRIMARY KEY,
  task_id     INTEGER NOT NULL REFERENCES tasks(id) ON DELETE RESTRICT,
  started_at  DATETIME NOT NULL,
  ended_at    DATETIME,
  notes       TEXT,
  created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX idx_single_active
  ON time_entries((1))
  WHERE ended_at IS NULL;

CREATE TABLE shortcut_stories (
  id          INTEGER PRIMARY KEY,
  external_id INTEGER NOT NULL UNIQUE,
  title       TEXT,
  epic_name   TEXT,
  state       TEXT,
  fetched_at  DATETIME NOT NULL
);

CREATE INDEX idx_tasks_open
  ON tasks(created_at)
  WHERE completed_at IS NULL AND archived_at IS NULL;

CREATE INDEX idx_time_entries_task_started
  ON time_entries(task_id, started_at);
