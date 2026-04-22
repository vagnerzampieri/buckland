---
title: "Buckland — design spec"
type: spec
status: draft
date: 2026-04-22
supersedes: docs/plans/2026-04-22-001-feat-buckland-timetracking-plan.md
---

# Buckland — Design Spec

An all-Rust, Linux-first, open-source time tracker for developers who use Shortcut. A CLI + TUI + passive tray that share one SQLite file. No daemon, no web UI, no cloud.

## Problem

The author uses Toggl today and is happy with most of it, but it has four gaps that compound:

1. **No Shortcut integration.** Every timer starts with "copy story title, paste into Toggl."
2. **Todo-list is separate from the timer.** Managing what-to-do in one tool and timing it in another doubles the bookkeeping.
3. **Capture UX is not keyboard-first.** The Toggl Desktop app still needs mouse interactions for common actions.
4. **Data is in someone else's cloud**, with rate limits on the free plan.

Alternatives like Hamster, Timewarrior, Watson, and ActivityWatch were considered. None of them solve all four gaps in a single tool, and none of them integrate with Shortcut.

## Goals

- **Low-friction capture** from the keyboard: `bl start SC-123` as the fast path, TUI as the rich path, tray as the ambient monitor.
- **Todo-list and time tracking on the same object.** A `Task` is both an item in the list and a group of time entries; you never have to reconcile two sides.
- **Shortcut integration** (read-only) that pre-fills task titles and epic names from story IDs.
- **Reports focused on the author's workflow**: total time per task today/week/month, grouped by task or epic, primarily to copy-paste into Shortcut manually.
- **Local-first**: all state in one SQLite file, no network required after an initial Shortcut fetch.
- **Open source release**: GPL-3.0-or-later, `.deb` packages, `cargo install`, public on crates.io once the CLI stabilizes.

## Non-Goals

- **Multi-user / multi-device sync.** Single user, single machine, no cloud back-end.
- **Shortcut write-back.** v1 is read-only; users copy time totals into Shortcut manually. Revisit in v1.x based on real usage.
- **macOS or Windows binaries.** The core library is portable; the tray is Linux-only via `ksni`. Cross-platform tray ships when someone asks for it.
- **A web UI or any HTTP surface.** No Rails, no Tauri, no Electron, no local JSON API.
- **Rich charts.** Reports render as Unicode-block bars sized to the terminal; anyone who wants a full dashboard can query the SQLite file or `bl report --json`.
- **Pomodoro / parallel timers.** One active timer at a time; starting a new one stops the current one.
- **Mouse interactions** in the TUI. Keyboard only in v1.

## Architecture

```
┌────────────────────────────────────────────────────────────────────┐
│                                                                    │
│   bl (binary)                          bl-tray (binary, optional)  │
│   ├── CLI mode (clap)                  ├── ksni (SNI host)         │
│   └── TUI mode (ratatui + crossterm)   └── glib 1Hz tick           │
│           │                                    │                   │
│           └──────────────┬─────────────────────┘                   │
│                          │                                         │
│                      ┌───▼───┐                                     │
│                      │ core  │ (src/lib.rs)                        │
│                      ├───────┤                                     │
│                      │domain │ TimerOps, TaskRepo, ReportBuilder   │
│                      │storage│ rusqlite wrappers                   │
│                      │shortcut│ reqwest::blocking + cache policy   │
│                      └───┬───┘                                     │
│                          │                                         │
│                  ┌───────▼────────┐                                │
│                  │ SQLite file    │   $XDG_DATA_HOME/buckland/     │
│                  └────────────────┘                                │
│                                                                    │
│   External: https://api.app.shortcut.com (read-only)               │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

### Crate layout

One crate (`buckland`), multiple binaries, one feature flag for the tray.

```
buckland/
├── Cargo.toml
├── README.md
├── CHANGELOG.md
├── CONTRIBUTING.md
├── CODE_OF_CONDUCT.md
├── LICENSE                  # GPL-3.0-or-later
├── migrations/
│   └── 0001_init.sql
├── resources/
│   ├── buckland.svg
│   ├── buckland-tray.desktop
│   ├── tray-idle.svg
│   └── tray-running.svg
├── src/
│   ├── lib.rs               # core re-exports
│   ├── main.rs              # bin `bl` (CLI dispatch + TUI entry)
│   ├── bin/
│   │   └── bl-tray.rs       # bin `bl-tray`, feature-gated
│   ├── cli/                 # clap derive structs + dispatch
│   ├── tui/                 # ratatui app, screens, keymap
│   │   ├── app.rs
│   │   ├── keymap.rs
│   │   ├── theme.rs
│   │   └── screens/
│   │       ├── tasks.rs
│   │       ├── agenda.rs
│   │       ├── report.rs
│   │       ├── edit.rs
│   │       └── help.rs
│   ├── domain/
│   │   ├── task.rs
│   │   ├── time_entry.rs
│   │   ├── timer_ops.rs
│   │   └── report.rs
│   ├── storage/
│   │   ├── mod.rs
│   │   └── repo.rs          # rusqlite wrappers
│   ├── shortcut/
│   │   ├── client.rs        # reqwest::blocking
│   │   └── fetcher.rs       # cache-policy wrapper
│   └── config.rs            # toml + XDG paths
└── tests/
    ├── cli_add.rs
    ├── cli_start_stop.rs
    ├── cli_report.rs
    ├── shortcut_client.rs   # wiremock
    └── tui_tasks.rs         # TestBackend + insta snapshots
```

### Feature flags

```toml
[features]
default = ["tray"]
tray = ["dep:ksni", "dep:glib"]
```

Users who do not want tray deps (`libdbus`, `glib`, `ksni`) install with `cargo install buckland --no-default-features` and get only `bl`.

## Data Model

SQLite, one file. Schema shipped as a single initial migration; subsequent changes are additive migrations, never edits.

```sql
CREATE TABLE tasks (
  id                INTEGER PRIMARY KEY,
  title             TEXT NOT NULL,
  description       TEXT,
  shortcut_story_id INTEGER REFERENCES shortcut_stories(id),
  completed_at      DATETIME,       -- "done"
  archived_at       DATETIME,       -- "out of my face"
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

-- Single-active-timer invariant. Indexes the NOT NULL column
-- (`started_at`) filtered to active rows, avoiding any confusion
-- about NULL distinctness that a `(ended_at) WHERE ended_at IS NULL`
-- index would raise.
CREATE UNIQUE INDEX idx_single_active
  ON time_entries(started_at)
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
```

### Key decisions

- **Storage** — datetimes written in UTC as ISO8601 (`rusqlite` default with `chrono` feature). Conversion to `Local` happens at the boundary (TUI display, agenda bucketing).
- **Durations computed in Rust, not SQL.** `ended_at.unwrap_or_else(Utc::now) - started_at` returns a `chrono::Duration`. SQL aggregation (`SUM(COALESCE(...) - ...)`) is unreliable in SQLite and we don't need it — reports iterate a small row set and sum in Ruby... err, Rust.
- **Task destroy cascades are blocked.** `ON DELETE RESTRICT` on the FK means deleting a task with time entries errors out. `bl delete <id>` works only on empty tasks; otherwise use `bl archive <id>`. This prevents silently erasing months of history.
- **"Pause" is stop + start.** Multiple `time_entries` per task over time; resuming creates a new entry. No `paused_at` or accumulation column. The report sums all entries for a task.
- **Day bucketing uses `chrono::Local`.** "Today" is the user's local day, not UTC. DST transitions are handled by `chrono`.
- **Shortcut story cache TTL = 1h.** Re-fetch on next access if `fetched_at < now - 1h`.
- **Single Shortcut workspace assumption.** `external_id` is unique. If the user switches workspaces mid-life, documented workaround is to clear the `shortcut_stories` table; a startup check that pings `GET /member` to record the workspace slug is a deferred addition.

## Command Grammar (CLI)

`clap` v4 derive. Subcommand verbs are imperatives. Exit codes:
- `0` — success or "active" for `bl status`
- `1` — nothing to report (status with no active timer) or logical failure
- `2` — usage error (clap default)

```
bl add <title> [--sc <ID>] [--description <text>]
    Create a task. --sc accepts "SC-123", "sc-123", or "123"; fetches and
    caches story metadata.

bl list [--all | --archived | --completed]
    Default: only open (not completed, not archived).
    Output aligned columns: ID, SC-id, title, total, last-touched.

bl start <target>
    <target> is one of (in resolution order):
      1. numeric task id (matches tasks.id)
      2. "SC-NNN" or "NNN" (matches tasks.shortcut_story_id via shortcut_stories.external_id)
      3. free text → creates a new task with that title and starts it
    Stops any active timer atomically in the same transaction.

bl stop
bl pause            (alias for stop)
    Ends the active timer. No-op with exit 1 if idle.

bl status
    Prints the active timer, e.g.:
      "SC-123 Fix login flow — 01:23:45 (started 09:15)"
    Exit 0 if active, 1 if idle. Scriptable.

bl done <id>
    Sets completed_at = now.

bl archive <id>
    Sets archived_at = now.

bl delete <id>
    Hard delete. Fails with a message if the task has time entries,
    suggesting `bl archive` instead.

bl report [--today | --week | --month | --all | --range FROM..TO]
          [--by-task | --by-epic | --by-day]
          [--json]
    Default: --today --by-task. Tables rendered with Unicode block bars.
    --json dumps a structured object for scripting.

bl tui
    Opens the TUI fullscreen. `bl` with no args is a synonym.

bl tray
    Runs the tray-icon process until SIGINT. Alternative: `bl-tray` binary.

bl config get <key>
bl config set <key> <value>
bl config list
    Manipulates $XDG_CONFIG_HOME/buckland/config.toml. Known keys:
      shortcut.token, ui.icons, ui.accent_color, tray.poll_seconds.

bl shortcut <SC-ID>
    Forces re-fetch of a story into the cache. Useful for debugging.
```

## TUI

Four screens plus two overlays.

**Common chrome** — top header line with the active timer (or "idle"), bottom status bar with contextual shortcuts. Status bar text changes per screen.

### Screens

- **Tasks (home)** — default when `bl` opens. Lists open tasks with the running one highlighted and the daily total summarized at the bottom.
- **Agenda** — by day, with entries grouped under their day header. Navigate weeks with `h`/`l`. Drill into an entry (`enter`) opens the Edit overlay.
- **Report** — totals with Unicode-block bars. `Tab` cycles scope (today/week/month/all), `T` cycles grouping (task/epic/day). `c` copies a one-liner to the system clipboard via `wl-copy` (Wayland) or `xclip` (X11); `j` toggles a JSON dump.
- **Edit entry** — modal centered over a dimmed backdrop. Fields: task (read-only), started_at, ended_at, notes. `Tab` cycles fields, `^D` deletes with an inline "y/N" confirm.
- **Help** — overlay listing all shortcuts grouped by category. `?` to open, `esc`/`q` to close.

### Keymap

Vim-inspired but not puritan. Full map:

```
Navigation     j k         move down/up        h l     move left/right or prev/next
               gg G        top/bottom          Tab     cycle view / modal field
View switch    g           Tasks (home)
               a           Agenda
               r           Report
Task actions   n           New task (inline prompt)
               s           Start selected
               S           Stop active
               d           Done
               A           Archive
               D           Delete (with confirm)
               e           Edit selected entry (modal)
               /           Filter (inline)
               enter       Default action for context
Report actions Tab         Cycle scope
               T           Cycle group
               c           Copy summary to clipboard
               j           Toggle JSON dump
Global         ?           Help overlay
               q / esc     Quit or close overlay
```

- `a` is reserved exclusively for the Agenda view. Adding a new task is `n` ("new"), never `a`. This keeps letters single-purpose across screens.
- **Context of `e` and `D`:** these act on the currently selected *row*, and the row's kind depends on the screen. On Tasks, the row is a task, so `e` opens a task edit modal (title, description, SC-id) and `D` hard-deletes the task (fails if it has time entries, suggesting Archive instead). On Agenda, the row is a time entry, so `e` opens the entry edit modal and `D` deletes the entry. Report has no row selection; `e`/`D` are no-ops there.
- Confirmation prompts are inline at the bottom: `Delete entry #287 (1h 23m)? y/N`.

### Theme

- Colors come from `Color::Indexed` (the terminal's 16 ANSI + 8 bright). No hardcoded RGB.
- One overridable accent color (`ui.accent_color`, default `Color::Cyan`) used for: running state, focus ring, highlight bar.
- Selected row: `Modifier::REVERSED` on accent.
- Epic / metadata: `Modifier::DIM`.
- Totals: `Modifier::BOLD`.
- Running `▶` marker blinks at ~0.5Hz via `Modifier::SLOW_BLINK`.

### Icons

Config key `ui.icons = "unicode" | "ascii"`, default `unicode`. Nerd-font glyphs are not used.

```
unicode    ascii      meaning
▶          >          running
⏸          =          paused
✓          x          done
●          *          today
◯          o          pending
├─  └─  ─  |- `- -    tree joiners
```

### State flow

```rust
enum Screen {
    Tasks(TasksState),
    Agenda(AgendaState),
    Report(ReportState),
    Edit(EditState),
    Help,
}

enum Action {
    Transition(Screen),
    Mutate(Op),
    Async(Job),
    Quit,
}

struct App<R: Repo> {
    screen: Screen,
    active_timer: Option<ActiveTimerSnapshot>,  // always visible in header
    repo: R,
    events: Receiver<Event>,
    jobs: Receiver<JobResult>,
}
```

Main loop `select!`s on events and job results. Each iteration produces at most one `Action`; the loop applies it and re-renders. A 1Hz timer posts a `Tick` event that refreshes the header clock. Screens are pure in the ratatui sense — they receive `&State` or `&mut State` but never touch `rusqlite` directly; mutations go through the `Repo` trait.

**Optimistic updates.** Pressing `s` on a task changes the visual state immediately (row goes accent + `▶`), then the transaction runs. On failure (I/O, DB error), the visual rollback plus an error line at the bottom.

## Tray

`bl-tray` is a short-lived process the user starts at login via the `.desktop` autostart file (disabled by default, enabled by user).

- Opens the SQLite file read-only (`SQLITE_OPEN_READ_ONLY`).
- Polls `time_entries` for the active row every `tray.poll_seconds` (default 30).
- On state transition (active → idle, idle → active, or task change), swaps the icon and updates the tooltip.
- A `glib::timeout_add_local` at 1Hz recomputes the elapsed time locally from `started_at` (no DB access); only the tooltip label changes.
- Menu has one item: "Quit". Everything else goes through `bl` CLI/TUI.
- If the SQLite file is missing (user hasn't run `bl` yet), tray sits in idle with tooltip "Buckland: no database yet".
- If polling fails (permissions, lock contention), tray falls back to an "error" icon variant and the tooltip says "Buckland: cannot read database".

### GNOME caveat

Tray icons on GNOME Shell require the AppIndicator Support extension since GNOME 3.26. The README explicitly documents this as a prerequisite for GNOME users. KDE Plasma and XFCE work out of the box.

## Shortcut Integration

### Client

`shortcut::Client` — thin wrapper around `reqwest::blocking::Client`. Exposes:

```rust
pub fn fetch_story(&self, id: i64) -> Result<Story, ShortcutError>;
```

Where `ShortcutError` is a `thiserror` enum with variants:

- `Auth(String)` — 401, token missing or invalid.
- `NotFound` — 404.
- `RateLimited { retry_after: Option<Duration> }` — 429.
- `Transient { status: u16 }` — 5xx.
- `Timeout` — request exceeded 5s.
- `Network(#[from] reqwest::Error)` — other I/O errors.
- `MalformedResponse(String)` — JSON parse failure.

On success, returns a `Story { external_id, title, epic_name, state }`. The client validates input before building the URL: IDs are normalized to digits-only (strip leading `SC-` or `sc-`), and anything else returns a validation error before the request goes out.

### Cache policy

`shortcut::Fetcher` wraps the client and the `shortcut_stories` table.

```rust
pub fn get(&self, id: i64) -> Result<Story, FetcherError>;
```

- Query the cache. If `fetched_at > now - 1h`, return it.
- Otherwise call the client. On success, upsert by `external_id`, update `fetched_at`, return.
- On failure (any `ShortcutError`), if a cached row exists (even stale), return it with a warning logged; otherwise propagate.

### Token storage

Read from `$XDG_CONFIG_HOME/buckland/config.toml` under `shortcut.token`. Not in env vars, not in keyring (keyring is deferred). The config file is created with mode 0600. The token is added to any log formatter's filter list (we do not log full HTTP requests in release builds).

## Security Notes

Plan-level controls and their justifications:

- **No HTTP surface.** The data store is a file on disk; there is no local API for browser tabs or other processes to exploit. Local processes with access to the user's home directory can read/write the SQLite file — that is the correct trust boundary for a single-user tool.
- **HTTPS to Shortcut** via `rustls-tls` (no OpenSSL system dependency, reproducible builds).
- **Shortcut ID validation** at the `Client` boundary (digits only after prefix strip) — prevents path injection.
- **Token is filtered from logs** via a tagged secret wrapper.
- **`.env` is in `.gitignore`** from the initial commit (though we do not use dotenv; tokens live in `config.toml`). No shell-sourced secrets.

## Testing

```
Unit          ~70%   inline mod tests per module
Integration   ~25%   tests/ dir, in-memory SQLite, assert_cmd
HTTP          ~5%    wiremock server per test
TUI            —     ratatui TestBackend + insta snapshots
Tray           —     manual smoke test documented in CONTRIBUTING
```

### Disciplines

- `cargo test` runs everything in under 10 seconds on a laptop.
- Tests do no real I/O except in-memory SQLite.
- Every bug that reaches the author becomes a failing test before the fix.
- No mocking of things we own; only at HTTP and filesystem boundaries.

## CI & Release

### CI (`ci.yml`)

Runs on pull requests and pushes to `main`:

- `lint` — `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`.
- `test` — `cargo test --all-features` on `ubuntu-latest`.
- `msrv` — `cargo +<MSRV> test` to guarantee the declared minimum supported Rust version holds.
- `audit` — `cargo audit` for advisories.
- `build` — `cargo build --release` sanity check.

### Release (`release.yml`)

Triggers on git tag `v*`:

- `deb` — `cargo deb -p buckland` produces `buckland_<ver>_amd64.deb`.
- `tarball` — `cargo build --release` + `tar czf buckland-<ver>-linux-x86_64.tar.gz`.
- `github` — creates a GitHub Release attaching the artifacts.
- `crates` — `cargo publish` (manual in the first few releases, automated once the CLI surface stabilizes).

### Install surfaces

- `.deb` — for Debian / Ubuntu, primary path. Installs `/usr/bin/bl`, `/usr/bin/bl-tray`, `.desktop` file, icons, `changelog.Debian.gz`.
- `cargo install buckland` — for Arch / Fedora / BSD / wherever `.deb` doesn't land.
- `cargo install buckland --no-default-features` — for users who don't want tray deps.

### MSRV

Declared in `Cargo.toml` as `rust-version = "1.74"` initially. Bump only with a changelog entry and a test run on the new MSRV.

## OSS Readiness

Mirrors the sibling `martin` project:

- **License** — GPL-3.0-or-later. `LICENSE` at the repo root.
- **README** — English primary (pt/en bilingual is deferred to v1.1 if someone asks). Sections: tagline, screenshots (TUI + tray), features, requirements, install, quickstart, configuration, keyboard shortcuts, roadmap, contributing, license.
- **CHANGELOG.md** — Keep a Changelog format.
- **CONTRIBUTING.md** — development setup, test expectations, commit style (Conventional Commits), PR checklist.
- **CODE_OF_CONDUCT.md** — Contributor Covenant.
- **Issue templates** — bug report, feature request under `.github/ISSUE_TEMPLATE/`.
- **GitHub Actions** — `ci.yml`, `release.yml`.

## Deferred / Out-of-Scope for v1

- Shortcut write-back (post time entries as comments).
- Fuzzy search on task titles (`/` filters literally in v1; fuzzy is a polish item).
- Command palette (`:` in the TUI for arbitrary actions).
- Mouse support in the TUI.
- macOS / Windows tray.
- Multi-workspace support for Shortcut (see Data Model note).
- Backup / sync. The SQLite file is the user's responsibility; document `cp` / `rsync` workflows in the README.
- Theming beyond the accent color.
- Multi-split TUI panes.
- Pause/resume with gap-aware single entries (see Data Model: we use stop+start instead).

## Open Questions

None blocking the first implementation plan. Minor items to decide during execution:

- Exact MSRV (`1.74` is the proposal; pick the current stable at implementation time).
- Exact ratatui version (target the latest stable at implementation; `0.29` as of this spec).
- Clipboard library choice: use OS commands (`wl-copy` / `xclip`) or the `arboard` crate. Leaning toward OS commands for fewer deps.

## References

- The Rust Programming Language — https://doc.rust-lang.org/book/
- Rust API Guidelines — https://rust-lang.github.io/api-guidelines/
- ratatui — https://ratatui.rs/
- ksni — https://docs.rs/ksni/
- rusqlite — https://docs.rs/rusqlite/
- rusqlite_migration — https://docs.rs/rusqlite_migration/
- clap v4 derive — https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html
- wiremock — https://docs.rs/wiremock/
- insta — https://insta.rs/
- cargo-deb — https://github.com/kornelski/cargo-deb
- Shortcut API — https://developer.shortcut.com/api/rest/v3
- Keep a Changelog — https://keepachangelog.com/
- Contributor Covenant — https://www.contributor-covenant.org/
- StatusNotifierItem (KDE/GNOME) — https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/
- Sibling project `martin` (workspace reference for OSS shape) — `../martin/`
