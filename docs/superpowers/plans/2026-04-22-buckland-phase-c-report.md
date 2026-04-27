# Buckland Phase C — Reports Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `bl report` with scopes (`--today | --week | --month | --all | --range`), groupings (`--by-task | --by-epic | --by-day`), Unicode block bars, and a `--json` dump for scripting. Extends Phase B's Shortcut integration with epic-name resolution so `--by-epic` produces meaningful labels.

**Architecture:** Three new pieces. (1) A new domain module `src/domain/report.rs` owns `Scope`, `Grouping`, `ReportBuilder`, and `Report`/`ReportRow` value types — pure logic, no I/O beyond a `Repo` reference. (2) A new repo method `list_entries_in_range` returns time entries that overlap a UTC range. (3) The CLI grows a `Report` subcommand that parses scope/grouping flags, dispatches to `ReportBuilder::build`, and renders either a Unicode-block-bar table or JSON. Epic resolution is added one layer up: `Story` gains a transient `epic_id`, the client exposes `fetch_epic`, and `Fetcher::get`/`refresh` compose story + epic so `epic_name` reaches the `shortcut_stories` cache.

**Tech Stack:** No new runtime dependencies. `serde_json` (already a dep) renders the JSON output; `chrono::Local` resolves day boundaries; `serde` derive is added to the new report types for the JSON path.

---

## What this phase delivers

CLI surface added in this phase:

```
bl report [--today | --week | --month | --all | --range FROM..TO]
          [--by-task | --by-epic | --by-day]
          [--json]
    Default: --today --by-task. Renders a Unicode-block-bar table to stdout.
    --json prints a structured object for scripting (no bars, no headers).
    Scope flags are mutually exclusive; grouping flags are mutually exclusive.
    Range syntax: FROM..TO with FROM and TO as YYYY-MM-DD (both inclusive).
```

Behavioural changes layered on top of Phase B:

- `Story` (in-memory DTO) gains `epic_id: Option<i64>` so the fetcher can resolve epic names.
- `shortcut::Client` gains `fetch_epic(id) -> Result<Epic, ShortcutError>`.
- `Fetcher::get` and `Fetcher::refresh` now call `fetch_epic` when the story carries an `epic_id` and `epic_name` is empty. The persisted `shortcut_stories.epic_name` is populated. Existing pre-Phase-C cache rows are filled in on the next refresh (either after the 1h TTL expires or when the user runs `bl shortcut SC-N`).

## Required reading (load before executing the first task)

- **Spec:** `docs/superpowers/specs/2026-04-22-buckland-design.md` — especially §"Command Grammar (CLI)" (the `bl report` block), §"Data Model" (the "Durations computed in Rust, not SQL" decision), and §"Shortcut Integration" (Client/Fetcher contract).
- **Project guidelines:** `CLAUDE.md` — §"Rust Idioms First" (chrono boundaries, no ORM), §"Object Design" (size budgets), §"Testing Approach".
- **Prior phases:**
  - `docs/superpowers/plans/2026-04-22-buckland-phase-a-cli-core.md` — Repo trait, Context shape, format helpers.
  - `docs/superpowers/plans/2026-04-22-buckland-phase-b-shortcut.md` — Client / Fetcher / Story contract you are extending.

## Preconditions

Before starting Task 1:

- [ ] `git status` on `main` is clean.
- [ ] Phase B is marked `done` in `docs/superpowers/plans/README.md`.
- [ ] `cargo test` passes (run it once to confirm green baseline).
- [ ] `cargo clippy --all-targets -- -D warnings` is clean.

## Postconditions (how to verify Phase C is done)

After the final task:

- [ ] `cargo test` green, `cargo clippy --all-targets -- -D warnings` clean, `cargo fmt --all --check` clean.
- [ ] `bl report --help` shows scope flags, grouping flags, and `--json`. Mutually exclusive groups are rejected by clap.
- [ ] On an empty database, `bl report` exits 0 and prints a "no time tracked" message (not an error).
- [ ] Seeded with at least one closed entry inside today's local day, `bl report` prints a table with a non-empty bar; `bl report --json | jq` parses cleanly.
- [ ] `bl report --by-epic` lumps tasks with no linked story or no epic into a single "(no epic)" group.
- [ ] No real HTTP calls in tests — only mockito.
- [ ] `docs/superpowers/plans/README.md` has Phase C marked `done (<date>)` and Phase D promoted to `ready`.

## Architecture (in scope for this phase)

```
src/
├── cli/
│   ├── args.rs        # +Report subcommand with scope/grouping/json flags
│   ├── commands.rs    # +report() handler
│   ├── format.rs      # +bar(), +scope_header()
│   └── mod.rs         # +Commands::Report dispatch
├── domain/
│   ├── mod.rs         # +pub mod report; +pub use report::*
│   └── report.rs      # NEW: Scope, Grouping, Report, ReportRow, ReportBuilder
├── shortcut/
│   ├── client.rs      # +fetch_epic(), +Epic DTO, +epic_id capture in StoryPayload
│   ├── fetcher.rs     # +epic resolution in get/refresh
│   └── story.rs       # +epic_id field on Story
└── storage/
    └── repo.rs        # +list_entries_in_range()
```

```
tests/
├── cli_report.rs           # NEW: integration tests for table output + scopes
├── cli_report_json.rs      # NEW: integration tests for --json
└── cli_report_epic.rs      # NEW: integration tests for --by-epic with mockito
```

## Tech stack (this phase)

| Concern | Choice | Why |
|---------|--------|-----|
| Date math | `chrono::Local` for day boundaries; `chrono::Duration` everywhere else | Spec decision. UTC at storage, Local at display. |
| JSON serialization | `serde` derive + `serde_json::to_string_pretty` | Already deps. No new crates. |
| Bar rendering | Unicode block characters `█▏▎▍▌▋▊▉` | Spec mandates Unicode block bars. Width hard-coded at 30 for v1; `terminal_size`-aware sizing is a v1.x polish item (added to `docs/future-ideas.md` if not already implied). |
| Range parsing | `chrono::NaiveDate::parse_from_str("%Y-%m-%d")` | Standard. No regex, no third-party parser. |
| Test seed | Direct `rusqlite::Connection::open` against the test DB to inject entries with chosen timestamps | Avoids sleep/wall-clock fragility. The CLI is still the test target; only seed timestamps go through the SQL side door. |

## Decisions (lock in before coding)

- **Range overlap, not start-membership.** An entry contributes to `[from, to)` if its `[started_at, ended_at_or_now)` interval overlaps the scope. Contribution = `min(end_or_now, to) − max(start, from)`, clamped to zero. This makes a 14h timer that started yesterday correctly count partial credit toward today.
- **Local-day boundaries.** "Today" is `Local::today()` 00:00:00 to next-day 00:00:00, converted to UTC for the SQL filter. DST transitions are chrono's job.
- **Week = ISO week (Mon–Sun).** Resolved in Local. Matches spec's "this week" intuition for European/Brazilian users; deterministic.
- **Range = both endpoints inclusive whole days.** `--range 2026-04-01..2026-04-30` covers `[2026-04-01 00:00 Local, 2026-05-01 00:00 Local)`.
- **Empty scope is success, not failure.** `bl report` with zero contributions prints a "No time tracked in this scope." line and exits 0. Logic mirrors `bl list` returning 0 on an empty list.
- **`--by-epic` with no token configured still works.** It only reads the cached `shortcut_stories.epic_name`; tasks without a story or with `epic_name = None` group under "(no epic)".
- **Bar width.** Fixed 30 chars in v1. `bar(value, max, 30)` — max is the largest group total in the report. Empty bar = empty string (not `░░...`). The spec says "sized to the terminal" — that's a v1.x improvement; the spec does not block fixed-width.
- **Active timer in JSON output.** Reported as is; consumers see `duration_seconds` but no "is_active" flag in v1. JSON shape stays minimal.

## Next phase

Phase D — ratatui TUI (Tasks / Agenda / Report screens, Edit modal, Help overlay). The `Report` rendered by Phase C's CLI handler is reused inside the Report screen — render it into a `Paragraph` instead of stdout.

---

## Task 1: CLI args scaffolding for `bl report`

**Files:**
- Modify: `src/cli/args.rs`
- Modify: `src/cli/mod.rs`
- Modify: `src/cli/commands.rs`
- Modify: `tests/cli_smoke.rs`

- [ ] **Step 1: Write the failing smoke test for `bl report --help`**

Append to `tests/cli_smoke.rs`:

```rust
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
```

Also extend the existing `help_shows_subcommands` test by adding one more line before the final `;`:

```rust
        .stdout(contains("report"));
```

- [ ] **Step 2: Run the failing test**

Run: `cargo test --test cli_smoke report_help_lists_flags -- --exact`
Expected: FAIL — `report` is not a subcommand yet.

- [ ] **Step 3: Add the `Report` variant to `Commands`**

Edit `src/cli/args.rs`. After the `Shortcut { id }` variant (still inside the `Commands` enum), append:

```rust
    /// Report time totals over a scope, grouped by task / epic / day.
    Report {
        /// Time tracked today (default).
        #[arg(long, group = "scope")]
        today: bool,
        /// Time tracked in the current ISO week (Monday–Sunday, local).
        #[arg(long, group = "scope")]
        week: bool,
        /// Time tracked in the current calendar month (local).
        #[arg(long, group = "scope")]
        month: bool,
        /// Time tracked across the entire database.
        #[arg(long, group = "scope")]
        all: bool,
        /// Custom range FROM..TO with both endpoints as YYYY-MM-DD (inclusive).
        #[arg(long, group = "scope", value_name = "FROM..TO")]
        range: Option<String>,
        /// Group rows by task (default).
        #[arg(long, group = "grouping")]
        by_task: bool,
        /// Group rows by Shortcut epic (uses cached epic_name).
        #[arg(long, group = "grouping")]
        by_epic: bool,
        /// Group rows by local calendar day.
        #[arg(long, group = "grouping")]
        by_day: bool,
        /// Emit a JSON object instead of a table.
        #[arg(long)]
        json: bool,
    },
```

clap's `group = "scope"` and `group = "grouping"` make the flags mutually exclusive automatically.

- [ ] **Step 4: Wire the dispatch in `src/cli/mod.rs`**

Edit the `match cli.command` block. After the `Commands::Delete` arm, before the closing brace, add:

```rust
        Commands::Report {
            today,
            week,
            month,
            all,
            range,
            by_task,
            by_epic,
            by_day,
            json,
        } => commands::report(
            &mut ctx,
            commands::ReportArgs {
                today,
                week,
                month,
                all,
                range,
                by_task,
                by_epic,
                by_day,
                json,
            },
        ),
```

- [ ] **Step 5: Add a stub handler in `src/cli/commands.rs`**

Append to `src/cli/commands.rs`:

```rust
pub struct ReportArgs {
    pub today: bool,
    pub week: bool,
    pub month: bool,
    pub all: bool,
    pub range: Option<String>,
    pub by_task: bool,
    pub by_epic: bool,
    pub by_day: bool,
    pub json: bool,
}

pub fn report(_ctx: &mut Context, _args: ReportArgs) -> anyhow::Result<i32> {
    // Stub — fully implemented in Tasks 4–11.
    println!("report: not yet implemented");
    Ok(0)
}
```

- [ ] **Step 6: Run the smoke test**

Run: `cargo test --test cli_smoke report_help_lists_flags -- --exact`
Expected: PASS.

Run: `cargo test --test cli_smoke help_shows_subcommands -- --exact`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/cli/args.rs src/cli/mod.rs src/cli/commands.rs tests/cli_smoke.rs
git commit -m "feat(cli): scaffold bl report subcommand with scope/grouping flags"
```

---

## Task 2: Domain — `Scope`, `Grouping`, range parsing

**Files:**
- Create: `src/domain/report.rs`
- Modify: `src/domain/mod.rs`

- [ ] **Step 1: Wire the new module into the domain**

Edit `src/domain/mod.rs`. Replace its contents with:

```rust
//! Domain types and operations.

pub mod report;
pub mod shortcut_story;
pub mod task;
pub mod time_entry;
pub mod timer_ops;

pub use report::{Grouping, Report, ReportBuilder, ReportRow, Scope, ScopeKind};
pub use shortcut_story::ShortcutStory;
pub use task::Task;
pub use time_entry::TimeEntry;
pub use timer_ops::TimerOps;
```

- [ ] **Step 2: Write the failing tests for `Scope`**

Create `src/domain/report.rs`:

```rust
//! Report aggregation: scope resolution, grouping, and totals.
//!
//! `Scope` resolves CLI flags into a UTC `[from, to)` window plus a kind
//! tag for display. `ReportBuilder` walks the entries the repo returns and
//! distributes their durations into groups.

use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeKind {
    Today,
    Week,
    Month,
    All,
    Range,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Scope {
    pub kind: ScopeKind,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Grouping {
    Task,
    Epic,
    Day,
}

#[derive(Debug, thiserror::Error)]
pub enum ScopeError {
    #[error("range must be FROM..TO with two YYYY-MM-DD dates")]
    RangeShape,
    #[error("range FROM ({from}) must be on or before TO ({to})")]
    RangeOrder { from: NaiveDate, to: NaiveDate },
    #[error("range date {0} is not YYYY-MM-DD")]
    RangeDate(String),
}

impl Scope {
    /// Today in local time, expressed as a UTC half-open interval.
    pub fn today(now: DateTime<Utc>) -> Self {
        let local_today = now.with_timezone(&chrono::Local).date_naive();
        let from = local_midnight_utc(local_today);
        let to = local_midnight_utc(local_today.succ_opt().expect("date does not overflow"));
        Self {
            kind: ScopeKind::Today,
            from,
            to,
        }
    }

    /// Current ISO week (Monday–Sunday) in local time.
    pub fn week(now: DateTime<Utc>) -> Self {
        let local_today = now.with_timezone(&chrono::Local).date_naive();
        let weekday = local_today.weekday();
        let days_since_monday = weekday.num_days_from_monday() as i64;
        let monday = local_today - Duration::days(days_since_monday);
        let next_monday = monday + Duration::days(7);
        Self {
            kind: ScopeKind::Week,
            from: local_midnight_utc(monday),
            to: local_midnight_utc(next_monday),
        }
    }

    /// Current calendar month in local time.
    pub fn month(now: DateTime<Utc>) -> Self {
        let local_today = now.with_timezone(&chrono::Local).date_naive();
        let first = NaiveDate::from_ymd_opt(local_today.year(), local_today.month(), 1)
            .expect("first day of month exists");
        let next_first = next_month_first(first);
        Self {
            kind: ScopeKind::Month,
            from: local_midnight_utc(first),
            to: local_midnight_utc(next_first),
        }
    }

    /// Everything ever recorded, plus a generous tail so currently-active
    /// timers count.
    pub fn all(now: DateTime<Utc>) -> Self {
        let from = Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap();
        let to = now + Duration::days(1);
        Self {
            kind: ScopeKind::All,
            from,
            to,
        }
    }

    /// Parse `FROM..TO` with both endpoints as `YYYY-MM-DD` (inclusive whole days).
    pub fn range(input: &str) -> Result<Self, ScopeError> {
        let (from_str, to_str) = input.split_once("..").ok_or(ScopeError::RangeShape)?;
        if from_str.is_empty() || to_str.is_empty() {
            return Err(ScopeError::RangeShape);
        }
        let from_date = NaiveDate::parse_from_str(from_str, "%Y-%m-%d")
            .map_err(|_| ScopeError::RangeDate(from_str.to_string()))?;
        let to_date = NaiveDate::parse_from_str(to_str, "%Y-%m-%d")
            .map_err(|_| ScopeError::RangeDate(to_str.to_string()))?;
        if from_date > to_date {
            return Err(ScopeError::RangeOrder {
                from: from_date,
                to: to_date,
            });
        }
        let next_after_to = to_date.succ_opt().expect("date does not overflow");
        Ok(Self {
            kind: ScopeKind::Range,
            from: local_midnight_utc(from_date),
            to: local_midnight_utc(next_after_to),
        })
    }
}

fn local_midnight_utc(date: NaiveDate) -> DateTime<Utc> {
    let naive = date.and_hms_opt(0, 0, 0).expect("midnight always exists");
    chrono::Local
        .from_local_datetime(&naive)
        .single()
        .or_else(|| chrono::Local.from_local_datetime(&naive).earliest())
        .expect("local midnight resolvable (ambiguous DST handled by .earliest())")
        .with_timezone(&Utc)
}

fn next_month_first(first_of_month: NaiveDate) -> NaiveDate {
    let (y, m) = (first_of_month.year(), first_of_month.month());
    if m == 12 {
        NaiveDate::from_ymd_opt(y + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(y, m + 1, 1).unwrap()
    }
}

// ReportBuilder, Report, ReportRow live here too — added in later tasks.

#[cfg(test)]
mod tests {
    use super::*;

    fn at(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, 0, 0).unwrap()
    }

    #[test]
    fn today_spans_local_midnight_to_next_midnight() {
        let now = at(2026, 4, 27, 13);
        let s = Scope::today(now);
        assert_eq!(s.kind, ScopeKind::Today);
        // The window must contain `now`.
        assert!(s.from <= now && now < s.to);
        // Its width must be at most 25 hours (DST tolerance) and at least 23.
        let width = (s.to - s.from).num_hours();
        assert!((23..=25).contains(&width), "width was {width}h");
    }

    #[test]
    fn week_starts_on_monday_and_is_seven_days() {
        let monday = at(2026, 4, 27, 13); // Mon 2026-04-27 13:00 UTC
        let s = Scope::week(monday);
        assert_eq!(s.kind, ScopeKind::Week);
        let width = (s.to - s.from).num_days();
        assert!((6..=7).contains(&width), "width was {width}d");
    }

    #[test]
    fn month_uses_calendar_first_to_first() {
        let mid_april = at(2026, 4, 15, 12);
        let s = Scope::month(mid_april);
        let width = (s.to - s.from).num_days();
        // April has 30 days; allow 29..=31 for any DST or month-length edges.
        assert!((28..=31).contains(&width), "width was {width}d");
    }

    #[test]
    fn all_covers_a_huge_range_ending_in_the_future() {
        let now = at(2026, 4, 27, 13);
        let s = Scope::all(now);
        assert!(s.to > now);
        assert!(s.from < at(2000, 1, 1, 0));
    }

    #[test]
    fn range_parses_inclusive_dates() {
        let s = Scope::range("2026-04-01..2026-04-30").unwrap();
        assert_eq!(s.kind, ScopeKind::Range);
        let width = (s.to - s.from).num_days();
        assert!((29..=31).contains(&width));
    }

    #[test]
    fn range_rejects_bad_shape() {
        assert!(matches!(Scope::range(""), Err(ScopeError::RangeShape)));
        assert!(matches!(Scope::range("..2026-04-30"), Err(ScopeError::RangeShape)));
        assert!(matches!(Scope::range("2026-04-01.."), Err(ScopeError::RangeShape)));
        assert!(matches!(Scope::range("2026-04-01"), Err(ScopeError::RangeShape)));
    }

    #[test]
    fn range_rejects_bad_dates() {
        assert!(matches!(
            Scope::range("2026-13-01..2026-04-30"),
            Err(ScopeError::RangeDate(_))
        ));
    }

    #[test]
    fn range_rejects_inverted_endpoints() {
        assert!(matches!(
            Scope::range("2026-04-30..2026-04-01"),
            Err(ScopeError::RangeOrder { .. })
        ));
    }
}
```

- [ ] **Step 3: Run the tests**

Run: `cargo test --lib domain::report`
Expected: all 8 tests PASS.

- [ ] **Step 4: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: clean.

Run: `cargo fmt --all`
Expected: no diff (or apply and re-run).

- [ ] **Step 5: Commit**

```bash
git add src/domain/mod.rs src/domain/report.rs
git commit -m "feat(domain): add Scope and Grouping types with range parsing"
```

---

## Task 3: Repo — `list_entries_in_range`

**Files:**
- Modify: `src/storage/repo.rs`

- [ ] **Step 1: Write the failing tests**

In `src/storage/repo.rs`, append inside `mod tests` (right before its closing `}`):

```rust
    #[test]
    fn list_entries_in_range_includes_overlapping_entries() {
        use chrono::TimeZone;
        let mut r = repo();
        let t = r.create_task("t", None).unwrap();
        // Closed entry 09:00–09:30
        let a = r
            .create_time_entry(t.id, Utc.with_ymd_and_hms(2026, 4, 22, 9, 0, 0).unwrap())
            .unwrap();
        r.end_time_entry(a.id, Utc.with_ymd_and_hms(2026, 4, 22, 9, 30, 0).unwrap())
            .unwrap();
        // Closed entry 10:00–11:00, fully inside the day
        let b = r
            .create_time_entry(t.id, Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap())
            .unwrap();
        r.end_time_entry(b.id, Utc.with_ymd_and_hms(2026, 4, 22, 11, 0, 0).unwrap())
            .unwrap();
        // Closed entry on the next day, must not appear in the 22nd window
        let c = r
            .create_time_entry(t.id, Utc.with_ymd_and_hms(2026, 4, 23, 9, 0, 0).unwrap())
            .unwrap();
        r.end_time_entry(c.id, Utc.with_ymd_and_hms(2026, 4, 23, 9, 15, 0).unwrap())
            .unwrap();

        let from = Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2026, 4, 23, 0, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 4, 23, 12, 0, 0).unwrap();
        let entries = r.list_entries_in_range(from, to, now).unwrap();
        let ids: Vec<i64> = entries.iter().map(|e| e.id).collect();
        assert_eq!(ids, vec![a.id, b.id]);
    }

    #[test]
    fn list_entries_in_range_includes_active_entry_started_before_range_end() {
        use chrono::TimeZone;
        let mut r = repo();
        let t = r.create_task("t", None).unwrap();
        // Active entry started inside the window.
        let active = r
            .create_time_entry(t.id, Utc.with_ymd_and_hms(2026, 4, 22, 14, 0, 0).unwrap())
            .unwrap();

        let from = Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2026, 4, 23, 0, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 4, 22, 16, 0, 0).unwrap();
        let entries = r.list_entries_in_range(from, to, now).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, active.id);
        assert!(entries[0].is_active());
    }

    #[test]
    fn list_entries_in_range_excludes_entries_ending_before_range() {
        use chrono::TimeZone;
        let mut r = repo();
        let t = r.create_task("t", None).unwrap();
        let earlier = r
            .create_time_entry(t.id, Utc.with_ymd_and_hms(2026, 4, 21, 9, 0, 0).unwrap())
            .unwrap();
        r.end_time_entry(earlier.id, Utc.with_ymd_and_hms(2026, 4, 21, 10, 0, 0).unwrap())
            .unwrap();
        let from = Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2026, 4, 23, 0, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 4, 22, 12, 0, 0).unwrap();
        let entries = r.list_entries_in_range(from, to, now).unwrap();
        assert!(entries.is_empty());
    }
```

- [ ] **Step 2: Add the trait method declaration**

Edit `src/storage/repo.rs`. Find the `pub trait Repo {` block. After the existing `delete_time_entry(...)` method declaration (search for `fn delete_time_entry`), add:

```rust
    /// All time entries whose `[started_at, ended_at_or_now)` interval
    /// overlaps the half-open UTC range `[from, to)`.
    /// Active entries (where `ended_at IS NULL`) are evaluated against `now`.
    /// Ordered by `started_at ASC`.
    fn list_entries_in_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> RepoResult<Vec<TimeEntry>>;
```

- [ ] **Step 3: Run the tests to confirm they fail**

Run: `cargo test --lib storage::repo::tests::list_entries_in_range`
Expected: FAIL — compile error or "not implemented" because the trait method has no `impl`.

- [ ] **Step 4: Implement on `SqliteRepo`**

Edit `src/storage/repo.rs`. In `impl Repo for SqliteRepo`, after the `delete_time_entry` impl (search for `fn delete_time_entry`), add:

```rust
    fn list_entries_in_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> RepoResult<Vec<TimeEntry>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {TIME_ENTRY_COLS} FROM time_entries \
             WHERE started_at < ?1 \
               AND COALESCE(ended_at, ?2) > ?3 \
             ORDER BY started_at ASC"
        ))?;
        let rows = stmt.query_map(params![to, now, from], |row| TimeEntry::try_from(row))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
```

- [ ] **Step 5: Run the tests**

Run: `cargo test --lib storage::repo::tests::list_entries_in_range`
Expected: 3 tests PASS.

- [ ] **Step 6: Lint**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add src/storage/repo.rs
git commit -m "feat(storage): add Repo::list_entries_in_range with overlap semantics"
```

---

## Task 4: Domain — `ReportBuilder` for `--by-task`

**Files:**
- Modify: `src/domain/report.rs`

- [ ] **Step 1: Write the failing tests**

In `src/domain/report.rs`, replace the placeholder line `// ReportBuilder, Report, ReportRow live here too — added in later tasks.` with a `Report` and `ReportRow` definition (we will keep tests at the bottom, as already structured), then add the `mod tests` for the builder.

Add the following types just below the `next_month_first` helper, before the `const _` placeholder. Delete the placeholder `const _` block.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub scope: Scope,
    pub grouping: Grouping,
    pub rows: Vec<ReportRow>,
    pub total_seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReportRow {
    pub label: String,
    pub duration_seconds: i64,
    /// `Some` only when [`Grouping::Task`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<i64>,
    /// `Some` only when [`Grouping::Task`] and the task links to a Shortcut story.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shortcut_external_id: Option<i64>,
    /// `Some` only when [`Grouping::Day`]. Format: YYYY-MM-DD (Local).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

pub struct ReportBuilder<'a, R: crate::storage::Repo> {
    repo: &'a R,
}

impl<'a, R: crate::storage::Repo> ReportBuilder<'a, R> {
    pub fn new(repo: &'a R) -> Self {
        Self { repo }
    }

    pub fn build(
        &self,
        scope: Scope,
        grouping: Grouping,
        now: DateTime<Utc>,
    ) -> Result<Report, crate::storage::RepoError> {
        match grouping {
            Grouping::Task => self.build_by_task(scope, now),
            Grouping::Epic => Err(crate::storage::RepoError::Sqlite(
                rusqlite::Error::InvalidQuery,
            )), // filled in by Task 10
            Grouping::Day => Err(crate::storage::RepoError::Sqlite(
                rusqlite::Error::InvalidQuery,
            )), // filled in by Task 8
        }
    }

    fn build_by_task(
        &self,
        scope: Scope,
        now: DateTime<Utc>,
    ) -> Result<Report, crate::storage::RepoError> {
        let entries = self.repo.list_entries_in_range(scope.from, scope.to, now)?;

        // task_id -> accumulated seconds
        let mut totals: std::collections::BTreeMap<i64, i64> = std::collections::BTreeMap::new();
        for e in &entries {
            let contribution = contribution_seconds(e, scope.from, scope.to, now);
            *totals.entry(e.task_id).or_default() += contribution;
        }

        let mut rows = Vec::with_capacity(totals.len());
        for (task_id, secs) in totals.iter() {
            let task = self
                .repo
                .find_task(*task_id)?
                .ok_or(crate::storage::RepoError::TaskNotFound(*task_id))?;
            let shortcut_external_id = match task.shortcut_story_id {
                Some(row_id) => self
                    .repo
                    .find_shortcut_story_by_row_id(row_id)?
                    .map(|s| s.external_id),
                None => None,
            };
            let label = match shortcut_external_id {
                Some(ext) => format!("SC-{ext} {}", task.title),
                None => task.title.clone(),
            };
            rows.push(ReportRow {
                label,
                duration_seconds: *secs,
                task_id: Some(task.id),
                shortcut_external_id,
                date: None,
            });
        }
        rows.sort_by(|a, b| b.duration_seconds.cmp(&a.duration_seconds));
        let total_seconds = rows.iter().map(|r| r.duration_seconds).sum();
        Ok(Report {
            scope,
            grouping: Grouping::Task,
            rows,
            total_seconds,
        })
    }
}

fn contribution_seconds(
    entry: &crate::domain::TimeEntry,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    now: DateTime<Utc>,
) -> i64 {
    let entry_end = entry.ended_at.unwrap_or(now);
    let start = entry.started_at.max(from);
    let end = entry_end.min(to);
    (end - start).max(Duration::zero()).num_seconds()
}
```

Then append, **inside the existing `mod tests` block** (above its closing brace), the following tests. They use a real in-memory repo to avoid mocking what we own:

```rust
    use crate::storage::SqliteRepo;

    fn seed_closed_entry(
        r: &mut SqliteRepo,
        task_id: i64,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) {
        use crate::storage::Repo;
        let e = r.create_time_entry(task_id, start).unwrap();
        r.end_time_entry(e.id, end).unwrap();
    }

    #[test]
    fn build_by_task_aggregates_per_task_within_scope() {
        use crate::storage::Repo;
        let mut r = SqliteRepo::in_memory();
        let t1 = r.create_task("alpha", None).unwrap();
        let t2 = r.create_task("beta", None).unwrap();

        let from = at(2026, 4, 22, 0);
        let to = at(2026, 4, 23, 0);
        let now = at(2026, 4, 22, 18);

        // Two closed entries on t1, totalling 90 minutes.
        seed_closed_entry(&mut r, t1.id, at(2026, 4, 22, 9), at(2026, 4, 22, 10));
        seed_closed_entry(&mut r, t1.id, at(2026, 4, 22, 11), at(2026, 4, 22, 11) + Duration::minutes(30));
        // One closed entry on t2, 30 minutes.
        seed_closed_entry(&mut r, t2.id, at(2026, 4, 22, 13), at(2026, 4, 22, 13) + Duration::minutes(30));

        let scope = Scope {
            kind: ScopeKind::Today,
            from,
            to,
        };
        let report = ReportBuilder::new(&r).build(scope, Grouping::Task, now).unwrap();
        assert_eq!(report.rows.len(), 2);
        // Sorted descending by duration: t1 first.
        assert_eq!(report.rows[0].task_id, Some(t1.id));
        assert_eq!(report.rows[0].duration_seconds, 90 * 60);
        assert_eq!(report.rows[1].task_id, Some(t2.id));
        assert_eq!(report.rows[1].duration_seconds, 30 * 60);
        assert_eq!(report.total_seconds, 120 * 60);
        assert_eq!(report.grouping, Grouping::Task);
    }

    #[test]
    fn build_by_task_clamps_entries_to_scope_window() {
        use crate::storage::Repo;
        let mut r = SqliteRepo::in_memory();
        let t = r.create_task("over the edge", None).unwrap();

        let from = at(2026, 4, 22, 0);
        let to = at(2026, 4, 23, 0);
        let now = at(2026, 4, 22, 18);

        // Entry started yesterday at 23:00, ended today at 02:00 — only the 2h on the
        // today side should count.
        seed_closed_entry(&mut r, t.id, at(2026, 4, 21, 23), at(2026, 4, 22, 2));

        let scope = Scope {
            kind: ScopeKind::Today,
            from,
            to,
        };
        let report = ReportBuilder::new(&r).build(scope, Grouping::Task, now).unwrap();
        assert_eq!(report.rows.len(), 1);
        assert_eq!(report.rows[0].duration_seconds, 2 * 3600);
    }

    #[test]
    fn build_by_task_counts_active_entry_up_to_now() {
        use crate::storage::Repo;
        let mut r = SqliteRepo::in_memory();
        let t = r.create_task("active", None).unwrap();
        let from = at(2026, 4, 22, 0);
        let to = at(2026, 4, 23, 0);
        let now = at(2026, 4, 22, 14);

        // Active entry started at 13:00, no end. Now is 14:00 → 1h should count.
        r.create_time_entry(t.id, at(2026, 4, 22, 13)).unwrap();

        let scope = Scope {
            kind: ScopeKind::Today,
            from,
            to,
        };
        let report = ReportBuilder::new(&r).build(scope, Grouping::Task, now).unwrap();
        assert_eq!(report.rows.len(), 1);
        assert_eq!(report.rows[0].duration_seconds, 3600);
    }

    #[test]
    fn build_by_task_includes_shortcut_external_id_when_linked() {
        use crate::storage::Repo;
        use crate::shortcut::Story;
        let mut r = SqliteRepo::in_memory();
        // NOTE: `Story` does not yet have `epic_id` in this task — Task 9 adds
        // it and updates this fixture to set `epic_id: None`. Do not include
        // the field here; the test would not compile until Task 9 runs.
        let row = r
            .upsert_shortcut_story(
                &Story {
                    external_id: 555,
                    title: Some("from sc".into()),
                    epic_name: None,
                    state: None,
                },
                Utc::now(),
            )
            .unwrap();
        let t = r.create_task("linked", None).unwrap();
        r.link_task_to_story(t.id, row.id, Utc::now()).unwrap();
        seed_closed_entry(&mut r, t.id, at(2026, 4, 22, 10), at(2026, 4, 22, 11));

        let scope = Scope {
            kind: ScopeKind::Today,
            from: at(2026, 4, 22, 0),
            to: at(2026, 4, 23, 0),
        };
        let now = at(2026, 4, 22, 12);
        let report = ReportBuilder::new(&r).build(scope, Grouping::Task, now).unwrap();
        assert_eq!(report.rows[0].shortcut_external_id, Some(555));
        assert!(report.rows[0].label.starts_with("SC-555 "));
    }
```

> Task 9 introduces `Story.epic_id`. When that task runs, the inline NOTE in this fixture is removed and `epic_id: None,` is added — Task 9 already lists this file in its "Update existing `Story { ... }` literals" step.

- [ ] **Step 2: Run the tests**

Run: `cargo test --lib domain::report`
Expected: previous 8 + new 4 PASS = 12 tests.

- [ ] **Step 3: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: clean.

Run: `cargo fmt --all`
Expected: no diff.

- [ ] **Step 4: Commit**

```bash
git add src/domain/report.rs
git commit -m "feat(domain): ReportBuilder aggregates by task with scope clamping"
```

---

## Task 5: Format — bar renderer

**Files:**
- Modify: `src/cli/format.rs`

- [ ] **Step 1: Write the failing tests**

Append to the `mod tests` block in `src/cli/format.rs` (just before its closing brace):

```rust
    #[test]
    fn bar_full_width_when_value_equals_max() {
        // 30 full blocks expected.
        let s = bar(100, 100, 30);
        assert_eq!(s.chars().count(), 30);
        assert!(s.chars().all(|c| c == '█'));
    }

    #[test]
    fn bar_empty_when_value_zero() {
        let s = bar(0, 100, 30);
        assert_eq!(s, "");
    }

    #[test]
    fn bar_empty_when_max_zero() {
        let s = bar(0, 0, 30);
        assert_eq!(s, "");
    }

    #[test]
    fn bar_half_width_when_value_is_half_of_max() {
        let s = bar(50, 100, 30);
        let chars: Vec<char> = s.chars().collect();
        // 15 full blocks
        assert_eq!(chars.iter().take_while(|c| **c == '█').count(), 15);
    }

    #[test]
    fn bar_uses_partial_block_when_fractional() {
        // 10/100 of 30 = 3.0 → exactly 3 full blocks, no trailing partial.
        let s = bar(10, 100, 30);
        assert_eq!(s.chars().count(), 3);
        // 1/100 of 30 = 0.3 → 0 full blocks + one partial.
        let s = bar(1, 100, 30);
        assert_eq!(s.chars().count(), 1);
        let c = s.chars().next().unwrap();
        assert!("▏▎▍▌▋▊▉".contains(c), "got {c:?}");
    }
```

- [ ] **Step 2: Run the failing tests**

Run: `cargo test --lib cli::format::tests::bar`
Expected: FAIL — `bar` not defined.

- [ ] **Step 3: Implement `bar`**

In `src/cli/format.rs`, append at the bottom of the file (before `#[cfg(test)]`):

```rust
/// Render a Unicode-block bar of `width` cells representing `value` against
/// `max`. Empty when `value` or `max` is zero. Uses partial blocks for
/// fractional widths so a 0.3-cell value still renders.
pub fn bar(value: i64, max: i64, width: usize) -> String {
    if value <= 0 || max <= 0 || width == 0 {
        return String::new();
    }
    let proportion = (value as f64 / max as f64).min(1.0);
    let cells = proportion * width as f64;
    let full = cells.floor() as usize;
    let remainder = cells - full as f64;

    // 8 partial-block glyphs from 1/8 to 7/8 of a cell.
    const PARTIALS: [char; 8] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];
    let partial_idx = (remainder * 8.0).round() as usize;

    let mut out = String::with_capacity(width * 4);
    for _ in 0..full {
        out.push('█');
    }
    if full < width && partial_idx > 0 {
        out.push(PARTIALS[partial_idx.min(7)]);
    }
    out
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib cli::format`
Expected: all bar tests + the existing duration tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/cli/format.rs
git commit -m "feat(cli): add Unicode-block bar renderer"
```

---

## Task 6: CLI — wire `bl report` end-to-end (default scope/grouping)

**Files:**
- Modify: `src/cli/commands.rs`
- Create: `tests/cli_report.rs`

- [ ] **Step 1: Write the failing integration tests**

Create `tests/cli_report.rs`:

```rust
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
```

(`TimeZone` is imported at the top because Task 7 adds tests that use `Utc.with_ymd_and_hms(...)`.)

- [ ] **Step 2: Run the failing tests**

Run: `cargo test --test cli_report`
Expected: FAIL — the stub `report` handler still prints "not yet implemented".

- [ ] **Step 3: Implement the real `report` handler**

Replace the stub `report` function in `src/cli/commands.rs` with:

```rust
pub fn report(ctx: &mut Context, args: ReportArgs) -> anyhow::Result<i32> {
    use crate::domain::{Grouping, ReportBuilder, Scope};

    let now = chrono::Utc::now();
    let scope = match resolve_scope(&args, now) {
        Ok(s) => s,
        Err(msg) => {
            println!("{msg}");
            return Ok(1);
        }
    };
    let grouping = resolve_grouping(&args);

    let report = ReportBuilder::new(&ctx.repo).build(scope, grouping, now)?;

    if args.json {
        let body = serde_json::to_string_pretty(&report)?;
        println!("{body}");
        return Ok(0);
    }

    if report.rows.is_empty() {
        println!("No time tracked in this scope.");
        return Ok(0);
    }

    print_table(&report);
    Ok(0)
}

fn resolve_scope(
    args: &ReportArgs,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<crate::domain::Scope, String> {
    use crate::domain::Scope;
    if args.week {
        Ok(Scope::week(now))
    } else if args.month {
        Ok(Scope::month(now))
    } else if args.all {
        Ok(Scope::all(now))
    } else if let Some(raw) = args.range.as_deref() {
        Scope::range(raw).map_err(|e| e.to_string())
    } else {
        // Default and explicit --today both land here.
        Ok(Scope::today(now))
    }
}

fn resolve_grouping(args: &ReportArgs) -> crate::domain::Grouping {
    use crate::domain::Grouping;
    if args.by_epic {
        Grouping::Epic
    } else if args.by_day {
        Grouping::Day
    } else {
        Grouping::Task
    }
}

fn print_table(report: &crate::domain::Report) {
    use crate::cli::format::{bar, duration_compact};
    use crate::domain::ScopeKind;
    let header = match report.scope.kind {
        ScopeKind::Today => "Today".to_string(),
        ScopeKind::Week => "This week".to_string(),
        ScopeKind::Month => "This month".to_string(),
        ScopeKind::All => "All time".to_string(),
        ScopeKind::Range => format!(
            "{} to {}",
            report
                .scope
                .from
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d"),
            (report.scope.to - chrono::Duration::days(1))
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d"),
        ),
    };
    println!("{header} ({} rows)", report.rows.len());

    let max = report
        .rows
        .iter()
        .map(|r| r.duration_seconds)
        .max()
        .unwrap_or(0);
    for row in &report.rows {
        let total = chrono::Duration::seconds(row.duration_seconds);
        println!(
            "{:<48}  {:>8}  {}",
            truncate(&row.label, 48),
            duration_compact(total),
            bar(row.duration_seconds, max, 30),
        );
    }
    let total = chrono::Duration::seconds(report.total_seconds);
    println!("{:<48}  {:>8}", "Total", duration_compact(total));
}
```

- [ ] **Step 4: Run the integration tests**

Run: `cargo test --test cli_report`
Expected: 2 tests PASS.

- [ ] **Step 5: Run all tests**

Run: `cargo test`
Expected: green.

- [ ] **Step 6: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings`
Run: `cargo fmt --all`
Expected: clean and no diff.

- [ ] **Step 7: Commit**

```bash
git add src/cli/commands.rs tests/cli_report.rs
git commit -m "feat(cli): bl report (default scope and grouping) end-to-end"
```

---

## Task 7: Other scopes — `--week`, `--month`, `--all`, `--range`

**Files:**
- Modify: `tests/cli_report.rs`

The handler already calls `resolve_scope`, which delegates to `Scope::week / month / all / range`. This task is purely test coverage at the integration level.

- [ ] **Step 1: Write the failing tests**

Append to `tests/cli_report.rs`:

```rust
#[test]
fn week_scope_includes_an_entry_from_yesterday() {
    let home = TempDir::new().unwrap();
    bl(&home).args(["add", "yesterday work"]).assert().success();

    let yesterday_9am = local_today_at(9, 0) - Duration::days(1);
    seed_closed_entry(&home, 1, yesterday_9am, yesterday_9am + Duration::minutes(45));

    bl(&home)
        .args(["report", "--week"])
        .assert()
        .success()
        .stdout(contains("yesterday work"))
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
        .stdout(predicates::prelude::PredicateBooleanExt::not(contains("out window")));
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
```

- [ ] **Step 2: Run the tests**

Run: `cargo test --test cli_report`
Expected: all PASS.

- [ ] **Step 3: Lint**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add tests/cli_report.rs
git commit -m "test(cli): bl report scope flags (week, month, all, range)"
```

---

## Task 8: Domain — `ReportBuilder` for `--by-day`

**Files:**
- Modify: `src/domain/report.rs`
- Modify: `tests/cli_report.rs`

- [ ] **Step 1: Write the failing domain tests**

Append inside the `mod tests` block of `src/domain/report.rs`, just before its closing brace:

```rust
    #[test]
    fn build_by_day_buckets_entries_by_local_date() {
        use crate::storage::Repo;
        let mut r = SqliteRepo::in_memory();
        let t = r.create_task("t", None).unwrap();

        // Two entries on the 22nd, one on the 23rd. All in UTC; the test
        // assumes the runner's local zone is the same as the chosen UTC days
        // for these specific hours (mid-day → not on the boundary).
        seed_closed_entry(&mut r, t.id, at(2026, 4, 22, 12), at(2026, 4, 22, 13));
        seed_closed_entry(&mut r, t.id, at(2026, 4, 22, 14), at(2026, 4, 22, 14) + Duration::minutes(30));
        seed_closed_entry(&mut r, t.id, at(2026, 4, 23, 12), at(2026, 4, 23, 13));

        let scope = Scope {
            kind: ScopeKind::Range,
            from: at(2026, 4, 22, 0),
            to: at(2026, 4, 24, 0),
        };
        let now = at(2026, 4, 24, 0);
        let report = ReportBuilder::new(&r).build(scope, Grouping::Day, now).unwrap();

        // Two distinct dates expected.
        assert_eq!(report.rows.len(), 2);
        // Sorted descending by duration: the 22nd has 90m, the 23rd has 60m.
        let labels: Vec<&str> = report.rows.iter().map(|r| r.label.as_str()).collect();
        assert!(labels[0].contains("2026-04-22"));
        assert!(labels[1].contains("2026-04-23"));
        assert_eq!(report.rows[0].duration_seconds, 90 * 60);
        assert_eq!(report.rows[1].duration_seconds, 60 * 60);
        assert_eq!(report.rows[0].date.as_deref(), Some("2026-04-22"));
    }
```

> Note: this test pins UTC hours far from midnight so the local date matches the UTC date in any timezone the CI runner uses. Do not weaken this guarantee.

- [ ] **Step 2: Run the failing test**

Run: `cargo test --lib domain::report::tests::build_by_day`
Expected: FAIL — `build` returns `InvalidQuery` for `Grouping::Day`.

- [ ] **Step 3: Implement `build_by_day`**

In `src/domain/report.rs`, modify the `build` match arm for `Grouping::Day` and add the helper. Replace the entire `build` body with:

```rust
    pub fn build(
        &self,
        scope: Scope,
        grouping: Grouping,
        now: DateTime<Utc>,
    ) -> Result<Report, crate::storage::RepoError> {
        match grouping {
            Grouping::Task => self.build_by_task(scope, now),
            Grouping::Day => self.build_by_day(scope, now),
            Grouping::Epic => Err(crate::storage::RepoError::Sqlite(
                rusqlite::Error::InvalidQuery,
            )), // filled in by Task 10
        }
    }
```

Append a new method on `ReportBuilder` next to `build_by_task`:

```rust
    fn build_by_day(
        &self,
        scope: Scope,
        now: DateTime<Utc>,
    ) -> Result<Report, crate::storage::RepoError> {
        let entries = self.repo.list_entries_in_range(scope.from, scope.to, now)?;

        // Day key (Local YYYY-MM-DD) → seconds.
        let mut totals: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
        for e in &entries {
            let entry_end = e.ended_at.unwrap_or(now);
            let mut cursor = e.started_at.max(scope.from);
            let cap = entry_end.min(scope.to);
            while cursor < cap {
                let local = cursor.with_timezone(&chrono::Local);
                let day = local.date_naive();
                let next_local = (day.succ_opt().unwrap()).and_hms_opt(0, 0, 0).unwrap();
                let next_local_utc = chrono::Local
                    .from_local_datetime(&next_local)
                    .single()
                    .unwrap_or_else(|| chrono::Local.from_local_datetime(&next_local).earliest().unwrap())
                    .with_timezone(&Utc);
                let segment_end = next_local_utc.min(cap);
                let secs = (segment_end - cursor).num_seconds().max(0);
                *totals.entry(day.format("%Y-%m-%d").to_string()).or_default() += secs;
                cursor = segment_end;
            }
        }

        let mut rows: Vec<ReportRow> = totals
            .into_iter()
            .map(|(label, seconds)| ReportRow {
                label: label.clone(),
                duration_seconds: seconds,
                task_id: None,
                shortcut_external_id: None,
                date: Some(label),
            })
            .collect();
        rows.sort_by(|a, b| b.duration_seconds.cmp(&a.duration_seconds));
        let total_seconds = rows.iter().map(|r| r.duration_seconds).sum();
        Ok(Report {
            scope,
            grouping: Grouping::Day,
            rows,
            total_seconds,
        })
    }
```

- [ ] **Step 4: Run the domain test**

Run: `cargo test --lib domain::report::tests::build_by_day`
Expected: PASS.

- [ ] **Step 5: Add a CLI integration test for `--by-day`**

Append to `tests/cli_report.rs`:

```rust
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
```

- [ ] **Step 6: Run the integration test**

Run: `cargo test --test cli_report by_day_groups_under_local_date_labels`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/domain/report.rs tests/cli_report.rs
git commit -m "feat(domain): build report by day with local-date bucketing"
```

---

## Task 9: Shortcut — epic resolution in client and fetcher

**Files:**
- Modify: `src/shortcut/story.rs`
- Modify: `src/shortcut/client.rs`
- Modify: `src/shortcut/fetcher.rs`
- Modify: `src/shortcut/mod.rs`
- Modify: `src/storage/repo.rs` (test fixtures only — `Story` literals add `epic_id`)

- [ ] **Step 1: Add `epic_id` to `Story`**

Edit `src/shortcut/story.rs`. Replace the `Story` struct:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Story {
    pub external_id: i64,
    pub title: Option<String>,
    /// Shortcut epic id, when the story is attached to one. Transient — not
    /// persisted in `shortcut_stories` (we resolve `epic_name` at fetch time).
    pub epic_id: Option<i64>,
    pub epic_name: Option<String>,
    pub state: Option<String>,
}
```

- [ ] **Step 2: Update existing `Story { ... }` literals**

`grep -rn "Story {" src/ tests/` finds the call sites. Add `epic_id: None,` to every literal of `Story`. Locations to update:

- `src/storage/repo.rs` (multiple test fixtures inside `mod tests`).
- `src/shortcut/client.rs` (the `into_story` impl — set `epic_id: self.epic_id`).
- `src/shortcut/fetcher.rs` (test fixtures inside `mod tests`).
- The Task 4 tests in `src/domain/report.rs` already include `epic_id: None,` per the note in Task 4 — confirm by re-reading that test.

- [ ] **Step 3: Capture `epic_id` in the client payload**

Edit `src/shortcut/client.rs`. In the `StoryPayload` struct, add a field:

```rust
    #[serde(default)]
    epic_id: Option<i64>,
```

In `into_story`:

```rust
impl StoryPayload {
    fn into_story(self) -> Story {
        Story {
            external_id: self.id,
            title: self.name,
            epic_id: self.epic_id,
            epic_name: None, // resolved by Fetcher
            state: self.workflow_state_id.map(|id| id.to_string()),
        }
    }
}
```

Also remove the now-misleading comment block above `into_story` ("In v1 we do not resolve epics; Phase C adds epic_id fetching when needed.") since this is the place that now captures it. Replace with a one-liner: `// epic_name is filled in by Fetcher::resolve_epic.`

- [ ] **Step 4: Add `Epic` DTO and `Client::fetch_epic`**

In `src/shortcut/client.rs`, after the `fetch_story` impl block, add:

```rust
impl Client {
    pub fn fetch_epic(&self, id: i64) -> Result<Epic, ShortcutError> {
        let url = format!(
            "{}/api/v3/epics/{}",
            self.base_url.trim_end_matches('/'),
            id
        );
        let resp = self
            .http
            .get(&url)
            .header("Shortcut-Token", &self.token)
            .header("Accept", "application/json")
            .send()
            .map_err(map_transport_error)?;

        let status = resp.status();
        if status.is_success() {
            let payload: EpicPayload = resp
                .json()
                .map_err(|e| ShortcutError::MalformedResponse(e.to_string()))?;
            return Ok(Epic {
                id: payload.id,
                name: payload.name,
            });
        }

        match status.as_u16() {
            401 | 403 => Err(ShortcutError::Auth(format!(
                "token rejected (status {})",
                status.as_u16()
            ))),
            404 => Err(ShortcutError::NotFound),
            429 => Err(ShortcutError::RateLimited {
                retry_after: resp
                    .headers()
                    .get("Retry-After")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(Duration::from_secs),
            }),
            code => Err(ShortcutError::Transient { status: code }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Epic {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct EpicPayload {
    id: i64,
    name: String,
}
```

> Note: this introduces a small amount of duplication with `fetch_story`'s status-mapping. We accept the duplication for v1 — extracting a helper is fine but optional. If you extract it, do so in this same task and adjust both call sites.

- [ ] **Step 5: Re-export `Epic` from `mod.rs`**

Edit `src/shortcut/mod.rs`. Replace the existing `pub use client::...` line with:

```rust
pub use client::{Client, Epic, ShortcutError, DEFAULT_BASE_URL};
```

- [ ] **Step 6: Add a client-level test for `fetch_epic`**

Append to the `mod tests` block in `src/shortcut/client.rs`:

```rust
    #[test]
    fn fetch_epic_success() {
        let mut server = Server::new();
        let _m = server
            .mock("GET", "/api/v3/epics/9")
            .match_header("Shortcut-Token", "abc")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":9,"name":"Auth refactor"}"#)
            .create();
        let client = mocked(&server, "abc");
        let epic = client.fetch_epic(9).unwrap();
        assert_eq!(epic.id, 9);
        assert_eq!(epic.name, "Auth refactor");
    }

    #[test]
    fn fetch_epic_404_maps_to_not_found() {
        let mut server = Server::new();
        let _m = server
            .mock("GET", "/api/v3/epics/404")
            .with_status(404)
            .create();
        let client = mocked(&server, "abc");
        assert!(matches!(
            client.fetch_epic(404).unwrap_err(),
            ShortcutError::NotFound
        ));
    }
```

- [ ] **Step 7: Resolve epic in the fetcher**

Edit `src/shortcut/fetcher.rs`. Add a private helper inside `impl Fetcher`:

```rust
    fn fetch_and_resolve(&self, external_id: i64) -> Result<crate::shortcut::Story, FetcherError> {
        let mut story = self.client.fetch_story(external_id)?;
        if let (Some(epic_id), None) = (story.epic_id, story.epic_name.as_ref()) {
            // epic_id present, epic_name not yet resolved → fetch.
            let epic = self.client.fetch_epic(epic_id)?;
            story.epic_name = Some(epic.name);
        }
        Ok(story)
    }
```

Then replace the two `self.client.fetch_story(external_id)?` call sites in `get` and `refresh` with `self.fetch_and_resolve(external_id)?`. Concretely:

In `get`, the stale-cache branch becomes:

```rust
            match self.fetch_and_resolve(external_id) {
                Ok(fresh) => {
                    let saved = repo.upsert_shortcut_story(&fresh, now)?;
                    Ok(Cached {
                        story: saved,
                        is_stale: false,
                    })
                }
                Err(FetcherError::Shortcut(ShortcutError::NotFound)) => {
                    Err(ShortcutError::NotFound.into())
                }
                Err(_) => Ok(Cached {
                    story: row,
                    is_stale: true,
                }),
            }
```

And the no-cache branch:

```rust
            let fresh = self.fetch_and_resolve(external_id)?;
            let saved = repo.upsert_shortcut_story(&fresh, now)?;
            Ok(Cached {
                story: saved,
                is_stale: false,
            })
```

In `refresh`:

```rust
    pub fn refresh<R: Repo>(
        &self,
        repo: &mut R,
        external_id: i64,
        now: DateTime<Utc>,
    ) -> Result<ShortcutStory, FetcherError> {
        let fresh = self.fetch_and_resolve(external_id)?;
        Ok(repo.upsert_shortcut_story(&fresh, now)?)
    }
```

- [ ] **Step 8: Add a fetcher test that exercises epic resolution**

Append to the `mod tests` block in `src/shortcut/fetcher.rs`:

```rust
    #[test]
    fn get_resolves_epic_name_on_fresh_fetch() {
        let mut server = Server::new();
        let _story = server
            .mock("GET", "/api/v3/stories/77")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":77,"name":"With epic","workflow_state_id":1,"epic_id":9}"#)
            .create();
        let _epic = server
            .mock("GET", "/api/v3/epics/9")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":9,"name":"Auth refactor"}"#)
            .create();
        let client = Client::new(server.url(), "tok");
        let fetcher = Fetcher::new(client);
        let mut repo = SqliteRepo::in_memory();
        let now = Utc::now();
        let cached = fetcher.get(&mut repo, 77, now).unwrap();
        assert_eq!(cached.story.epic_name.as_deref(), Some("Auth refactor"));
    }
```

- [ ] **Step 9: Run all shortcut tests**

Run: `cargo test --lib shortcut`
Expected: green, including the two new tests in `client.rs` and the new test in `fetcher.rs`.

- [ ] **Step 10: Run the whole suite**

Run: `cargo test`
Expected: green. The `Story` literal updates from Step 2 should silence any compiler error about a missing field.

- [ ] **Step 11: Lint and format**

Run: `cargo clippy --all-targets -- -D warnings`
Run: `cargo fmt --all`
Expected: clean and no diff.

- [ ] **Step 12: Commit**

```bash
git add src/shortcut/story.rs src/shortcut/client.rs src/shortcut/fetcher.rs src/shortcut/mod.rs src/storage/repo.rs src/domain/report.rs
git commit -m "feat(shortcut): resolve epic_name during fetch, persist into cache"
```

---

## Task 10: Domain — `ReportBuilder` for `--by-epic`

**Files:**
- Modify: `src/domain/report.rs`
- Create: `tests/cli_report_epic.rs`

- [ ] **Step 1: Write the failing domain test**

Append inside the `mod tests` block of `src/domain/report.rs`:

```rust
    #[test]
    fn build_by_epic_groups_tasks_by_epic_name() {
        use crate::storage::Repo;
        use crate::shortcut::Story;

        let mut r = SqliteRepo::in_memory();
        let alpha_row = r
            .upsert_shortcut_story(
                &Story {
                    external_id: 1,
                    title: Some("alpha".into()),
                    epic_id: Some(9),
                    epic_name: Some("Auth".into()),
                    state: None,
                },
                Utc::now(),
            )
            .unwrap();
        let beta_row = r
            .upsert_shortcut_story(
                &Story {
                    external_id: 2,
                    title: Some("beta".into()),
                    epic_id: Some(9),
                    epic_name: Some("Auth".into()),
                    state: None,
                },
                Utc::now(),
            )
            .unwrap();
        let gamma_row = r
            .upsert_shortcut_story(
                &Story {
                    external_id: 3,
                    title: Some("gamma".into()),
                    epic_id: None,
                    epic_name: None,
                    state: None,
                },
                Utc::now(),
            )
            .unwrap();

        let t_alpha = r.create_task("alpha task", None).unwrap();
        let t_beta = r.create_task("beta task", None).unwrap();
        let t_gamma = r.create_task("gamma task", None).unwrap();
        let t_loose = r.create_task("loose task", None).unwrap();
        r.link_task_to_story(t_alpha.id, alpha_row.id, Utc::now()).unwrap();
        r.link_task_to_story(t_beta.id, beta_row.id, Utc::now()).unwrap();
        r.link_task_to_story(t_gamma.id, gamma_row.id, Utc::now()).unwrap();

        seed_closed_entry(&mut r, t_alpha.id, at(2026, 4, 22, 9), at(2026, 4, 22, 10));
        seed_closed_entry(&mut r, t_beta.id, at(2026, 4, 22, 10), at(2026, 4, 22, 10) + Duration::minutes(30));
        seed_closed_entry(&mut r, t_gamma.id, at(2026, 4, 22, 11), at(2026, 4, 22, 11) + Duration::minutes(15));
        seed_closed_entry(&mut r, t_loose.id, at(2026, 4, 22, 12), at(2026, 4, 22, 12) + Duration::minutes(20));

        let scope = Scope {
            kind: ScopeKind::Today,
            from: at(2026, 4, 22, 0),
            to: at(2026, 4, 23, 0),
        };
        let report = ReportBuilder::new(&r)
            .build(scope, Grouping::Epic, at(2026, 4, 22, 18))
            .unwrap();

        // "Auth" gets t_alpha (60m) + t_beta (30m) = 90m. "(no epic)" gets gamma (15m) + loose (20m) = 35m.
        assert_eq!(report.rows.len(), 2);
        assert_eq!(report.rows[0].label, "Auth");
        assert_eq!(report.rows[0].duration_seconds, 90 * 60);
        assert_eq!(report.rows[1].label, "(no epic)");
        assert_eq!(report.rows[1].duration_seconds, 35 * 60);
    }
```

- [ ] **Step 2: Run the failing test**

Run: `cargo test --lib domain::report::tests::build_by_epic`
Expected: FAIL — the `Grouping::Epic` arm still returns `InvalidQuery`.

- [ ] **Step 3: Implement `build_by_epic`**

Edit `src/domain/report.rs`. Update the `build` match:

```rust
            Grouping::Epic => self.build_by_epic(scope, now),
```

Append a new method on `ReportBuilder`:

```rust
    fn build_by_epic(
        &self,
        scope: Scope,
        now: DateTime<Utc>,
    ) -> Result<Report, crate::storage::RepoError> {
        let entries = self.repo.list_entries_in_range(scope.from, scope.to, now)?;

        // task_id -> seconds (intermediate)
        let mut per_task: std::collections::BTreeMap<i64, i64> = std::collections::BTreeMap::new();
        for e in &entries {
            *per_task.entry(e.task_id).or_default() +=
                contribution_seconds(e, scope.from, scope.to, now);
        }

        // epic_label -> seconds (final)
        let mut per_epic: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
        for (task_id, secs) in per_task {
            let task = self
                .repo
                .find_task(task_id)?
                .ok_or(crate::storage::RepoError::TaskNotFound(task_id))?;
            let label = match task.shortcut_story_id {
                Some(row_id) => match self.repo.find_shortcut_story_by_row_id(row_id)? {
                    Some(s) => s.epic_name.unwrap_or_else(|| "(no epic)".to_string()),
                    None => "(no epic)".to_string(),
                },
                None => "(no epic)".to_string(),
            };
            *per_epic.entry(label).or_default() += secs;
        }

        let mut rows: Vec<ReportRow> = per_epic
            .into_iter()
            .map(|(label, seconds)| ReportRow {
                label,
                duration_seconds: seconds,
                task_id: None,
                shortcut_external_id: None,
                date: None,
            })
            .collect();
        rows.sort_by(|a, b| b.duration_seconds.cmp(&a.duration_seconds));
        let total_seconds = rows.iter().map(|r| r.duration_seconds).sum();
        Ok(Report {
            scope,
            grouping: Grouping::Epic,
            rows,
            total_seconds,
        })
    }
```

- [ ] **Step 4: Run the domain test**

Run: `cargo test --lib domain::report::tests::build_by_epic`
Expected: PASS.

- [ ] **Step 5: Add an integration test for `--by-epic`**

Create `tests/cli_report_epic.rs`:

```rust
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
    chrono::Local
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
```

- [ ] **Step 6: Run the integration tests**

Run: `cargo test --test cli_report_epic`
Expected: 2 PASS.

- [ ] **Step 7: Commit**

```bash
git add src/domain/report.rs tests/cli_report_epic.rs
git commit -m "feat(domain): bl report --by-epic groups via cached epic_name"
```

---

## Task 11: CLI — `--json` output

**Files:**
- Create: `tests/cli_report_json.rs`

The handler in Task 6 already routes `args.json` through `serde_json::to_string_pretty(&report)`. This task verifies the contract.

- [ ] **Step 1: Write the failing tests**

Create `tests/cli_report_json.rs`:

```rust
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
    chrono::Local
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
```

- [ ] **Step 2: Run the tests**

Run: `cargo test --test cli_report_json`
Expected: 2 PASS (the handler already supports `--json`).

- [ ] **Step 3: Run the whole suite**

Run: `cargo test`
Expected: green.

- [ ] **Step 4: Lint**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add tests/cli_report_json.rs
git commit -m "test(cli): bl report --json contract coverage"
```

---

## Task 12: Self-review, smoke flow, and phase handoff

**Files:**
- Modify: `docs/superpowers/plans/README.md`
- Create: `docs/smoke-tests/phase-c-report.md`

- [ ] **Step 1: Run the self-review checklist**

Top-to-bottom:

1. **Spec coverage:**
   - `bl report --today` (default) → `tests/cli_report.rs::default_report_groups_by_task_today` ✅
   - `bl report --week` → `tests/cli_report.rs::week_scope_includes_an_entry_from_yesterday` ✅
   - `bl report --month` → `tests/cli_report.rs::month_scope_works` ✅
   - `bl report --all` → `tests/cli_report.rs::all_scope_includes_very_old_entries` ✅
   - `bl report --range FROM..TO` → `tests/cli_report.rs::range_scope_filters_to_the_window` ✅
   - `bl report --by-task` (default) → covered by default test ✅
   - `bl report --by-day` → `tests/cli_report.rs::by_day_groups_under_local_date_labels` ✅
   - `bl report --by-epic` → `tests/cli_report_epic.rs::by_epic_groups_two_tasks_under_their_shared_epic` ✅
   - `bl report --json` → `tests/cli_report_json.rs::json_emits_a_valid_object_with_required_keys` ✅
   - Mutually exclusive scope flags → `tests/cli_report.rs::scope_flags_are_mutually_exclusive` ✅
   - Empty scope → `tests/cli_report.rs::empty_report_prints_message_and_exits_zero` ✅
2. **No `todo!()`, `unimplemented!()`, or `dbg!()` in `src/`.** Run `grep -rn 'todo!\|unimplemented!\|dbg!' src/` — expected empty.
3. **`cargo test` green; `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --all --check` clean.**
4. **`Cargo.lock` is committed and unchanged in this phase (no new deps).**

- [ ] **Step 2: Write the manual smoke test document**

Create `docs/smoke-tests/phase-c-report.md`:

````markdown
# Phase C — Manual smoke tests for `bl report`

Run these against a real shell after the phase merges. Each block is independent;
each clears `BUCKLAND_HOME` and seeds fresh data.

## 1. Empty report

```bash
export BUCKLAND_HOME=/tmp/bl-phase-c-smoke
rm -rf "$BUCKLAND_HOME"
cargo run -- add "noop"
cargo run -- report
# Expected: "No time tracked in this scope." Exit 0.
```

## 2. Default report (today, by task)

```bash
rm -rf "$BUCKLAND_HOME"
cargo run -- add "alpha"
cargo run -- start 1
sleep 3
cargo run -- stop
cargo run -- report
# Expected: a single row "alpha", non-empty bar, "Total" line.
```

## 3. JSON dump

```bash
cargo run -- report --json | jq .
# Expected: a JSON object with scope, grouping, rows, total_seconds.
```

## 4. By-day across two days

```bash
# Pre-requisite: have entries in the database from yesterday and today.
cargo run -- report --week --by-day
# Expected: one row per local day with entries.
```

## 5. By-epic with token configured

```bash
# Pre-requisite: ~/.config/buckland/config.toml has shortcut.token set
# and you have at least one task linked to a Shortcut story whose
# story has an epic. Force a fresh fetch:
cargo run -- shortcut SC-<ID>
cargo run -- report --by-epic
# Expected: rows with the epic's name; "(no epic)" lumps tasks
# without a story or without an epic.
```

## 6. Range parsing failures

```bash
cargo run -- report --range "garbage"
# Expected: exit 1 with a message about range shape.
cargo run -- report --range "2026-04-30..2026-04-01"
# Expected: exit 1 with a message about range order.
```
````

- [ ] **Step 3: Update the phase index**

Edit `docs/superpowers/plans/README.md`. In the Phase index table:

- Change the Phase C row from `ready` to `done (<today's date>)`.
- Change the Phase D row from `draft after C merges` to `ready` and set its file path to `2026-04-22-buckland-phase-d-tui.md` (the file does not yet exist — it will be drafted before execution).

- [ ] **Step 4: Commit the docs**

```bash
git add docs/superpowers/plans/README.md docs/smoke-tests/phase-c-report.md
git commit -m "docs(plans): mark Phase C done, promote Phase D to ready"
```

---

## Phase C complete

At this point `bl report` is a fully working scriptable surface for time totals. Smoke flow:

```bash
cargo run -- report                      # today, by task, table
cargo run -- report --week --by-day      # this week, one row per day
cargo run -- report --range 2026-04-01..2026-04-30 --by-epic
cargo run -- report --month --json | jq  # machine-readable
```

## Self-review checklist for the executing engineer

Before handing the phase off, run this top-to-bottom:

1. **Every CLI flag combination listed in the spec has at least one happy-path integration test.**
2. **Exit codes.** Bad range / mutually exclusive flags return non-zero.
3. **`cargo test` green, `cargo clippy --all-targets -- -D warnings` clean, `cargo fmt --all --check` clean.**
4. **No `todo!()`, `unimplemented!()`, or `dbg!()` in `src/`.**
5. **`Cargo.lock` committed and unchanged (no new deps in this phase).**
6. **No real network calls in tests.** `grep -rn "api.app.shortcut.com" tests/` should return only the manual-smoke document, never test code.
7. **DST tolerance.** All time-arithmetic tests allow ±1h fudge or use mid-day timestamps.

## What's next

Phase D — ratatui TUI. The Tasks, Agenda, and Report screens consume the same `Repo` and the same `ReportBuilder` Phase C just shipped. Phase D's Report screen renders into a `Paragraph` widget instead of stdout; the JSON path remains exclusive to the CLI.
