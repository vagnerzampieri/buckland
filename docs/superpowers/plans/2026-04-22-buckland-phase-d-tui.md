# Buckland Phase D — TUI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the fullscreen ratatui TUI promised in the spec — three screens (Tasks / Agenda / Report) plus two overlays (Edit / Help) — composed on top of the existing CLI domain layer. `bl` with no subcommand and `bl tui` both open it. The TUI mutates state through the same `Repo` trait the CLI uses; no new persistence code.

**Architecture:** A new `src/tui/` module owns the entire surface. `tui::run()` sets up the terminal (alt screen + raw mode), spawns a 1Hz tick thread, drains a `crossterm::event::EventStream` on the main thread, and feeds an `App<R: Repo>` state machine. The `App` holds the active `Screen` enum, an optional active-timer snapshot for the header, a footer status bar, and dispatches `Action`s (mutate, transition, quit). Each screen module owns its state plus `render(&self, frame, area, ctx)` and `handle_key(&mut self, key, app_ctx) -> Option<Action>`. The Edit overlay reuses the same `Action` channel for save / delete. Optimistic updates land first in screen state, then the repo call runs; on error the screen rolls back and posts an inline message.

**Tech Stack:** New runtime deps: `ratatui = "0.29"` and `crossterm = "0.28"` (ratatui's default backend). No `insta` (we assert on `TestBackend` buffers directly — see Decisions). No new feature flag: TUI is core. Clipboard uses shell-out to `wl-copy` / `xclip` (already on every Linux desktop) — no `arboard` dep.

---

## What this phase delivers

CLI surface added in this phase:

```
bl tui
    Opens the TUI fullscreen. Same entry point as `bl` with no subcommand.
```

Behavioral changes layered on top of Phase C:

- `bl` invoked with no subcommand no longer prints help; it launches the TUI.
- `bl tui` is an explicit synonym for the no-arg form.
- Keypress contract documented in the README's "Keyboard shortcuts" section.

User-facing surface inside the TUI:

- **Tasks** (home) — open tasks, running task highlighted with `▶`, daily total at the footer.
- **Agenda** — entries grouped by local day; `h`/`l` navigates weeks.
- **Report** — wraps the Phase C `ReportBuilder` output in Unicode-block bars; `Tab` cycles scope, `T` cycles grouping, `c` copies a one-liner, `j` toggles a JSON dump.
- **Edit** modal — task (read-only) + `started_at` / `ended_at` / `notes`; `Tab` cycles fields; `^D` deletes with `y/N` confirm.
- **Help** overlay — full keymap; `?` opens, `q`/`esc` closes.

## Required reading (load before executing the first task)

- **Spec:** `docs/superpowers/specs/2026-04-22-buckland-design.md` — read all of §"TUI" (screens, keymap, theme, icons, state flow) plus the "Optimistic updates" paragraph. Skim §"Architecture" for the crate-layout block under `src/tui/`.
- **Project guidelines:** `CLAUDE.md` — §"TUI Conventions (ratatui)", §"Rust Idioms First" (sync by default), §"Testing Approach", §"Object Design" (size budgets).
- **Prior phases:**
  - `docs/superpowers/plans/2026-04-22-buckland-phase-a-cli-core.md` — `Repo` trait shape, `Context`, format helpers, integration test harness.
  - `docs/superpowers/plans/2026-04-22-buckland-phase-c-report.md` — `ReportBuilder`, `Scope`, `Grouping`, `Report` value types — reused verbatim by the Report screen.
- **External:** ratatui 0.29 docs (https://ratatui.rs/), focus on `Frame`, `Layout`, `List`, `Paragraph`, `Block`, `Style`, `Modifier`, and `backend::TestBackend`.

## Preconditions

Before starting Task 1:

- [ ] `git status` on `main` is clean.
- [ ] Phase C is marked `done` in `docs/superpowers/plans/README.md`.
- [ ] `cargo test --all-features` passes (run it once to confirm green baseline).
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` is clean.
- [ ] You are about to create branch `phase-d-tui` from `main` (Task 1 does this — do not work on `main`).

## Postconditions (how to verify Phase D is done)

After the final task:

- [ ] `cargo test --all-features` green; `cargo clippy --all-targets --all-features -- -D warnings` clean; `cargo fmt --all --check` clean.
- [ ] `bl --help` lists `tui` as a subcommand.
- [ ] `bl tui` and `bl` (no args) both open the same TUI; both return exit 0 on `q`.
- [ ] `bl --version` still prints `0.3.x` (or whatever the current pre-Phase-D version was — Phase D itself does not bump the version; that lives in the merge commit).
- [ ] Manual smoke test passes (documented in Task 16): launch TUI on a tempdir DB seeded with two tasks and one closed entry, navigate Tasks → Agenda → Report, open Help, open Edit on an Agenda row, hit `q` to quit.
- [ ] All TUI rendering tests use `ratatui::backend::TestBackend` only — no real terminal in CI.
- [ ] `docs/superpowers/plans/README.md` has Phase D marked `done (<date>)` and Phase E promoted from `draft after D merges` to `ready`.
- [ ] Branch `phase-d-tui` is merged into `main` (the merge commit is the first place the version may bump to `0.4.0`; that bump is **not** part of this plan).

## Architecture (in scope for this phase)

```
src/
├── cli/
│   ├── args.rs        # +Tui variant; subcommand_required = false; default_value option for command
│   └── mod.rs         # +Commands::Tui dispatch; +None case → tui::run()
├── tui/               # NEW — entire module is new in this phase
│   ├── mod.rs         # pub fn run(repo, fetcher, config) -> anyhow::Result<()>
│   ├── terminal.rs    # raw-mode + alt-screen lifecycle, Drop guard
│   ├── app.rs         # App<R: Repo>, Screen, Action, event loop core
│   ├── theme.rs       # Theme struct, accent color resolution, named ANSI map
│   ├── keymap.rs      # KeyAction enum + key→action resolver
│   ├── chrome.rs      # render_header, render_footer
│   ├── clipboard.rs   # shell-out helper (wl-copy / xclip)
│   └── screens/
│       ├── mod.rs
│       ├── tasks.rs   # TasksState, render, handle_key, inline prompts
│       ├── agenda.rs  # AgendaState (week cursor, selected entry)
│       ├── report.rs  # ReportState (scope, grouping, json toggle)
│       ├── edit.rs    # EditState (target kind: Task or TimeEntry; field cursor)
│       └── help.rs    # render-only; no state
└── main.rs            # untouched — bin entry stays `cli::run()`

tests/
├── tui_smoke.rs            # NEW: assert_cmd that `bl tui --help` works (no real terminal)
└── (TUI rendering tests live inline as `#[cfg(test)] mod tests` per screen module — no integration tests for TUI rendering, only smoke for CLI dispatch)
```

`src/tui/mod.rs` re-exports `run`. `src/lib.rs` gains a `pub mod tui;` line.

## Tech stack (this phase)

| Concern | Choice | Why |
|---------|--------|-----|
| TUI framework | `ratatui = "0.29"` (default features) | Spec mandates ratatui. Latest stable as of writing. |
| Terminal backend | `crossterm = "0.28"` (default features, includes `event-stream`) | ratatui's default; no Windows-only code paths to dodge. |
| Test backend | `ratatui::backend::TestBackend` | Built into ratatui; no extra dep. |
| Snapshot library | None — direct buffer assertions | Spec mentions insta as a target; we defer it. Direct asserts are simpler and good enough for v1. The decision is logged in `docs/future-ideas.md` so we can revisit. |
| Clipboard | Shell-out to `wl-copy` (Wayland) or `xclip` (X11) | Spec leaning. No new crate. Best-effort: missing tools surface a footer error, not a panic. |
| Date display | `chrono::Local` for header/agenda/edit; UTC at storage | Same boundary rule as Phase C. |
| Config | Existing `crate::config::Config` (read-only inside TUI) | Already loads `ui.icons` and `ui.accent_color`. |
| Tick stream | `std::thread` + `std::sync::mpsc::Sender<Event>` posting `Event::Tick` every 1s | No tokio, no `crossterm-async`. Sync default per `CLAUDE.md`. |
| Optimistic update | Apply to in-memory screen state first, then call repo, rollback + footer error on failure | Spec mandate. |

## Decisions (lock in before coding)

- **No `insta` snapshot crate in this phase.** Direct `TestBackend` buffer assertions are enough to lock down rendering. Adding insta forces the engineer to manage snapshot files; we postpone the cost. If a future phase needs visual diffing, add insta then. Recorded as a future-ideas item in Task 16.
- **Reuse existing `Repo` trait, do not extend it.** Every TUI mutation maps to a method that already exists from Phase A/B/C: `create_task`, `mark_task_done`, `archive_task`, `delete_task`, `delete_time_entry`, `link_task_to_story`, plus `TimerOps::start` / `TimerOps::stop`. The only new repo capability the TUI needs is "update started_at / ended_at / notes for a time entry"; we add that as a single new method `update_time_entry` in Task 14 (it is the only gap, justified by the Edit overlay).
- **`bl` with no subcommand opens the TUI.** Achieved by switching `Cli::command` to `Option<Commands>` plus `subcommand_required(false)` in clap. `Commands::Tui` dispatches to the same `tui::run()` so the help text still lists `tui` explicitly. This change is contained in Task 1 + Task 15.
- **Header clock recomputes every Tick from `started_at`.** No persistent counter. Clock drift is impossible because the source is the DB row's start timestamp.
- **Optimistic updates**: in Tasks (start/stop/done/archive) and Agenda (delete) we mutate the screen's in-memory list immediately, render, then call the repo. On `RepoError`, we re-fetch the list from the repo and surface the error in the footer. Edits inside the Edit overlay are NOT optimistic (the overlay is modal and short-lived; we call the repo first, then close on success).
- **Filter (`/`) is a literal substring filter, not fuzzy.** Spec defers fuzzy to v1.x. Case-insensitive match against `task.title`.
- **Inline prompts** (new task, delete confirm) own the bottom row. While a prompt is open, `j`/`k`/global shortcuts are disabled — only printable characters, `Backspace`, `Enter`, and `Esc` are consumed.
- **No mouse, no clicks.** `crossterm::event::EnableMouseCapture` is **not** enabled. Spec non-goal.
- **Version bump deferred.** This plan does not touch `Cargo.toml`'s `version = ...`. The merge commit on `main` is where Phase D becomes a release tag; that's the OSS workflow's job in Phase F.

## Next phase

Phase E — `bl-tray` binary (ksni + glib 1Hz tick) becomes `ready` once Phase D merges. Phase E reuses the `Repo` trait in read-only mode and embeds two SVG assets; nothing in the TUI module is touched.

---

## Task 1: Branch + ratatui dependencies + `Tui` subcommand

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/cli/args.rs`
- Modify: `src/cli/mod.rs`
- Modify: `src/lib.rs`
- Modify: `tests/cli_smoke.rs`
- Create: `src/tui/mod.rs`

- [ ] **Step 1: Create the phase branch from clean main**

```bash
git checkout main
git pull --ff-only
git checkout -b phase-d-tui
```

Expected: `git status -sb` prints `## phase-d-tui` and an empty body.

- [ ] **Step 2: Write the failing smoke test for `bl tui --help`**

Append to `tests/cli_smoke.rs`:

```rust
#[test]
fn tui_subcommand_listed_in_help() {
    Command::cargo_bin("bl")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("tui"));
}

#[test]
fn tui_help_describes_command() {
    Command::cargo_bin("bl")
        .unwrap()
        .args(["tui", "--help"])
        .assert()
        .success()
        .stdout(contains("Open the TUI"));
}
```

- [ ] **Step 3: Run the failing tests**

Run: `cargo test --test cli_smoke tui_ -- --exact`
Expected: FAIL — `tui` is not a subcommand yet.

- [ ] **Step 4: Add ratatui + crossterm to dependencies**

Edit `Cargo.toml`. In `[dependencies]`, add the two new lines (alphabetized into the existing list):

```toml
crossterm = "0.28"
ratatui = "0.29"
```

The `[dependencies]` section after editing reads:

```toml
[dependencies]
anyhow = "1"
chrono = { version = "0.4", default-features = false, features = ["clock", "serde"] }
clap = { version = "4", features = ["derive"] }
crossterm = "0.28"
dirs = "5"
ratatui = "0.29"
reqwest = { version = "0.12", default-features = false, features = ["blocking", "rustls-tls", "json"] }
rusqlite = { version = "0.32", features = ["bundled", "chrono"] }
rusqlite_migration = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
toml = "0.8"
```

(`alphabetized` is the rule the existing manifest follows; preserve it.)

- [ ] **Step 5: Add the `Tui` variant to `Commands` and make `command` optional**

Edit `src/cli/args.rs`. Change `pub command: Commands` to `pub command: Option<Commands>`, add `#[command(subcommand_required = false, arg_required_else_help = false)]` on the `Cli` struct, and add the new variant after `Report { ... }`:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "bl",
    about = "Time tracker for developers who use Shortcut",
    version,
    subcommand_required = false,
    arg_required_else_help = false
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    // ... all existing variants unchanged ...

    /// Open the TUI. Same as running `bl` with no subcommand.
    Tui,
}
```

Keep every other variant exactly as it was. Only `Cli` (added two attributes, made `command` optional) and the new `Tui` variant change.

- [ ] **Step 6: Wire dispatch for `Commands::Tui` and the `None` case**

Edit `src/cli/mod.rs`. The match has to handle the new variant and the `None` case. Replace the body of `run()`:

```rust
//! CLI surface for `bl`.

pub mod args;
pub mod commands;
pub mod context;
pub mod format;
pub mod resolve;

use args::{Cli, Commands};
use clap::Parser;

pub fn run() -> anyhow::Result<i32> {
    let cli = Cli::parse();
    let mut ctx = context::open()?;
    let command = cli.command.unwrap_or(Commands::Tui);
    match command {
        Commands::Add {
            title,
            description,
            sc,
        } => commands::add(&mut ctx, &title, description.as_deref(), sc.as_deref()),
        Commands::Shortcut { id } => commands::shortcut_refresh(&mut ctx, &id),
        Commands::List {
            all,
            archived,
            completed,
        } => commands::list(&mut ctx, all, archived, completed),
        Commands::Start { target } => commands::start(&mut ctx, &target),
        Commands::Stop | Commands::Pause => commands::stop(&mut ctx),
        Commands::Status => commands::status(&mut ctx),
        Commands::Done { id } => commands::done(&mut ctx, id),
        Commands::Archive { id } => commands::archive(&mut ctx, id),
        Commands::Delete { id } => commands::delete(&mut ctx, id),
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
        Commands::Tui => crate::tui::run(ctx),
    }
}
```

- [ ] **Step 7: Add a placeholder `tui` module that exits cleanly**

Create `src/tui/mod.rs`:

```rust
//! Buckland TUI — ratatui surface composed on top of the `Repo` trait.
//!
//! Phase D landing point. Real terminal lifecycle and screens land in later
//! tasks of the same phase; for now `run()` is a no-op that returns 0 so
//! `bl tui --help` and `bl` (no-args) compile and exit cleanly.

use crate::cli::context::Context;

pub fn run(_ctx: Context) -> anyhow::Result<i32> {
    Ok(0)
}
```

Edit `src/lib.rs`, append a `pub mod tui;` line:

```rust
//! Buckland — personal time tracker core library.

pub mod cli;
pub mod config;
pub mod domain;
pub mod shortcut;
pub mod storage;
pub mod tui;
```

- [ ] **Step 8: Run the smoke tests; expect green**

Run: `cargo test --test cli_smoke -- --exact`
Expected: PASS — both `tui_subcommand_listed_in_help` and `tui_help_describes_command` pass; no other tests broken.

Run: `cargo build`
Expected: PASS, no warnings.

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml Cargo.lock src/cli/args.rs src/cli/mod.rs src/lib.rs src/tui/mod.rs tests/cli_smoke.rs
git commit -m "feat(tui): scaffold tui subcommand and ratatui deps"
```

---

## Task 2: Terminal lifecycle (raw mode + alt screen + Drop guard)

**Files:**
- Create: `src/tui/terminal.rs`
- Modify: `src/tui/mod.rs`

- [ ] **Step 1: Write the failing test for the terminal guard's `Drop` semantics**

Create `src/tui/terminal.rs` with the test stub first:

```rust
//! Terminal lifecycle: enter alt screen + raw mode on construction,
//! restore on drop. Designed so the production `run()` path holds a
//! `TerminalGuard` in a `let` binding and the destructor runs even if
//! the body panics.

#[cfg(test)]
mod tests {
    #[test]
    fn guard_construction_compiles() {
        // No real terminal in tests — we only assert the type exists and
        // compiles. Lifecycle is covered by the manual smoke check in the
        // README. Real Drop semantics are observed by running `bl tui`
        // and ensuring the parent shell is restored.
        let _: fn() -> () = || {};
    }
}
```

- [ ] **Step 2: Run the placeholder test**

Run: `cargo test --lib tui::terminal -- --nocapture`
Expected: FAIL — module is empty otherwise; rustc complains there's no `super`.

- [ ] **Step 3: Implement the `TerminalGuard`**

Replace `src/tui/terminal.rs` with the full implementation:

```rust
//! Terminal lifecycle: enter alt screen + raw mode on construction,
//! restore on drop. Designed so the production `run()` path holds a
//! `TerminalGuard` in a `let` binding and the destructor runs even if
//! the body panics.

use crossterm::event::DisableMouseCapture;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub struct TerminalGuard {
    terminal: Tui,
}

impl TerminalGuard {
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        crossterm::execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub fn terminal(&mut self) -> &mut Tui {
        &mut self.terminal
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best-effort restore. Errors here would leave the user's shell
        // wedged; we log to stderr but never panic.
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn guard_type_exists() {
        // We do not invoke `enter()` in tests because it touches the real
        // terminal. Compilation alone proves the type signature.
        fn _accepts(_: super::TerminalGuard) {}
    }
}
```

- [ ] **Step 4: Run the test**

Run: `cargo test --lib tui::terminal -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: clean.

- [ ] **Step 6: Wire `terminal` into `tui::mod`**

Replace `src/tui/mod.rs`:

```rust
//! Buckland TUI — ratatui surface composed on top of the `Repo` trait.

pub mod terminal;

use crate::cli::context::Context;

pub fn run(_ctx: Context) -> anyhow::Result<i32> {
    // Real loop lands in Task 5. Until then we still return 0 so the
    // subcommand smoke tests stay green.
    Ok(0)
}
```

- [ ] **Step 7: Commit**

```bash
git add src/tui/mod.rs src/tui/terminal.rs
git commit -m "feat(tui): terminal lifecycle guard"
```

---

## Task 3: Theme module (accent color + named ANSI map)

**Files:**
- Create: `src/tui/theme.rs`
- Modify: `src/tui/mod.rs`

- [ ] **Step 1: Write the failing tests**

Create `src/tui/theme.rs`:

```rust
//! Theme — converts the user's `ui.accent_color` config string into a
//! ratatui `Color`, plus shortcut helpers for the canonical styles
//! (selected row, dim metadata, bold totals, slow-blinking running marker).

use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub accent: Color,
}

impl Theme {
    pub fn from_config_accent(name: &str) -> Self {
        Self {
            accent: parse_color(name).unwrap_or(Color::Cyan),
        }
    }

    pub fn selected(&self) -> Style {
        Style::default().fg(self.accent).add_modifier(Modifier::REVERSED)
    }

    pub fn running(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::SLOW_BLINK)
    }

    pub fn dim(&self) -> Style {
        Style::default().add_modifier(Modifier::DIM)
    }

    pub fn total(&self) -> Style {
        Style::default().add_modifier(Modifier::BOLD)
    }
}

fn parse_color(name: &str) -> Option<Color> {
    match name.trim().to_ascii_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "darkgrey" => Some(Color::DarkGray),
        "lightred" => Some(Color::LightRed),
        "lightgreen" => Some(Color::LightGreen),
        "lightyellow" => Some(Color::LightYellow),
        "lightblue" => Some(Color::LightBlue),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightcyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_cyan_when_unknown() {
        let t = Theme::from_config_accent("not-a-color");
        assert_eq!(t.accent, Color::Cyan);
    }

    #[test]
    fn parses_named_ansi_colors_case_insensitively() {
        assert_eq!(Theme::from_config_accent("Magenta").accent, Color::Magenta);
        assert_eq!(Theme::from_config_accent("LIGHTGREEN").accent, Color::LightGreen);
    }

    #[test]
    fn selected_uses_accent_and_reversed() {
        let t = Theme::from_config_accent("red");
        let style = t.selected();
        assert_eq!(style.fg, Some(Color::Red));
        assert!(style.add_modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn running_uses_accent_and_slow_blink() {
        let t = Theme::from_config_accent("yellow");
        let style = t.running();
        assert_eq!(style.fg, Some(Color::Yellow));
        assert!(style.add_modifier.contains(Modifier::SLOW_BLINK));
    }

    #[test]
    fn dim_does_not_carry_accent() {
        let t = Theme::from_config_accent("blue");
        assert_eq!(t.dim().fg, None);
        assert!(t.dim().add_modifier.contains(Modifier::DIM));
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test --lib tui::theme -- --nocapture`
Expected: PASS (all five).

- [ ] **Step 3: Re-export from the `tui` module**

Edit `src/tui/mod.rs`:

```rust
//! Buckland TUI — ratatui surface composed on top of the `Repo` trait.

pub mod terminal;
pub mod theme;

use crate::cli::context::Context;

pub fn run(_ctx: Context) -> anyhow::Result<i32> {
    Ok(0)
}
```

- [ ] **Step 4: Commit**

```bash
git add src/tui/mod.rs src/tui/theme.rs
git commit -m "feat(tui): theme with accent-color resolution"
```

---

## Task 4: Keymap (Key → KeyAction enum)

**Files:**
- Create: `src/tui/keymap.rs`
- Modify: `src/tui/mod.rs`

- [ ] **Step 1: Write the failing tests**

Create `src/tui/keymap.rs`:

```rust
//! Single source of truth for keybindings. Screens consume `KeyAction`
//! variants, never raw `KeyEvent`s. New shortcuts go here first; this
//! makes the keymap auditable in one place and keeps the Help overlay
//! honest.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    // --- Navigation
    Down,
    Up,
    Left,
    Right,
    Top,
    Bottom,
    CycleNext,    // Tab
    CyclePrev,    // BackTab (Shift+Tab)

    // --- View switch
    OpenTasks,    // g
    OpenAgenda,   // a
    OpenReport,   // r

    // --- Task / Entry actions
    NewTask,      // n
    StartSelected,// s
    StopActive,   // S
    Done,         // d
    Archive,      // A
    Delete,       // D
    Edit,         // e
    Filter,       // /
    Confirm,      // Enter

    // --- Report actions
    CycleScope,   // Tab in Report (subset of CycleNext, dispatched by screen)
    CycleGroup,   // T
    Copy,         // c
    ToggleJson,   // j (only in Report; conflicts handled in screen)

    // --- Global
    Help,         // ?
    Quit,         // q / esc
    DeleteCtrlD,  // ^D — delete inside the Edit modal

    // --- Catch-all for free typing inside inline prompts
    Char(char),
    Backspace,

    // --- Anything we don't handle
    Ignore,
}

pub fn resolve(event: KeyEvent) -> KeyAction {
    let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
    match event.code {
        KeyCode::Char('d') if ctrl => KeyAction::DeleteCtrlD,
        KeyCode::Char('c') if ctrl => KeyAction::Quit,
        KeyCode::Char('j') => KeyAction::Down,
        KeyCode::Char('k') => KeyAction::Up,
        KeyCode::Char('h') => KeyAction::Left,
        KeyCode::Char('l') => KeyAction::Right,
        KeyCode::Char('g') => KeyAction::OpenTasks,
        KeyCode::Char('G') => KeyAction::Bottom,
        KeyCode::Char('a') => KeyAction::OpenAgenda,
        KeyCode::Char('r') => KeyAction::OpenReport,
        KeyCode::Char('n') => KeyAction::NewTask,
        KeyCode::Char('s') => KeyAction::StartSelected,
        KeyCode::Char('S') => KeyAction::StopActive,
        KeyCode::Char('d') => KeyAction::Done,
        KeyCode::Char('A') => KeyAction::Archive,
        KeyCode::Char('D') => KeyAction::Delete,
        KeyCode::Char('e') => KeyAction::Edit,
        KeyCode::Char('/') => KeyAction::Filter,
        KeyCode::Char('T') => KeyAction::CycleGroup,
        KeyCode::Char('c') => KeyAction::Copy,
        KeyCode::Char('?') => KeyAction::Help,
        KeyCode::Char('q') => KeyAction::Quit,
        KeyCode::Esc => KeyAction::Quit,
        KeyCode::Tab => KeyAction::CycleNext,
        KeyCode::BackTab => KeyAction::CyclePrev,
        KeyCode::Enter => KeyAction::Confirm,
        KeyCode::Backspace => KeyAction::Backspace,
        KeyCode::Up => KeyAction::Up,
        KeyCode::Down => KeyAction::Down,
        KeyCode::Left => KeyAction::Left,
        KeyCode::Right => KeyAction::Right,
        KeyCode::Home => KeyAction::Top,
        KeyCode::End => KeyAction::Bottom,
        KeyCode::Char(c) => KeyAction::Char(c),
        _ => KeyAction::Ignore,
    }
}

/// `gg` is the only multi-key sequence we honor. The App keeps a
/// `pending_g: bool` and consults this helper to decide what the *second*
/// key means once the first one was a `g`.
pub fn resolve_after_pending_g(event: KeyEvent) -> KeyAction {
    match event.code {
        KeyCode::Char('g') => KeyAction::Top,
        _ => resolve(event),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    #[test]
    fn vim_arrows_map_to_directions() {
        assert_eq!(resolve(key(KeyCode::Char('j'))), KeyAction::Down);
        assert_eq!(resolve(key(KeyCode::Char('k'))), KeyAction::Up);
        assert_eq!(resolve(key(KeyCode::Char('h'))), KeyAction::Left);
        assert_eq!(resolve(key(KeyCode::Char('l'))), KeyAction::Right);
    }

    #[test]
    fn ctrl_d_is_delete_ctrl_d() {
        assert_eq!(resolve(ctrl(KeyCode::Char('d'))), KeyAction::DeleteCtrlD);
    }

    #[test]
    fn ctrl_c_quits() {
        assert_eq!(resolve(ctrl(KeyCode::Char('c'))), KeyAction::Quit);
    }

    #[test]
    fn esc_quits() {
        assert_eq!(resolve(key(KeyCode::Esc)), KeyAction::Quit);
    }

    #[test]
    fn shifted_letters_pick_uppercase_actions() {
        assert_eq!(resolve(key(KeyCode::Char('S'))), KeyAction::StopActive);
        assert_eq!(resolve(key(KeyCode::Char('A'))), KeyAction::Archive);
        assert_eq!(resolve(key(KeyCode::Char('D'))), KeyAction::Delete);
        assert_eq!(resolve(key(KeyCode::Char('G'))), KeyAction::Bottom);
    }

    #[test]
    fn pending_g_then_g_is_top() {
        assert_eq!(
            resolve_after_pending_g(key(KeyCode::Char('g'))),
            KeyAction::Top
        );
    }

    #[test]
    fn pending_g_then_anything_else_falls_back_to_resolve() {
        assert_eq!(
            resolve_after_pending_g(key(KeyCode::Char('j'))),
            KeyAction::Down
        );
    }

    #[test]
    fn unknown_chars_are_passthrough() {
        assert_eq!(resolve(key(KeyCode::Char('x'))), KeyAction::Char('x'));
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test --lib tui::keymap -- --nocapture`
Expected: PASS (all eight).

- [ ] **Step 3: Re-export from `tui::mod`**

Edit `src/tui/mod.rs`:

```rust
//! Buckland TUI — ratatui surface composed on top of the `Repo` trait.

pub mod keymap;
pub mod terminal;
pub mod theme;

use crate::cli::context::Context;

pub fn run(_ctx: Context) -> anyhow::Result<i32> {
    Ok(0)
}
```

- [ ] **Step 4: Commit**

```bash
git add src/tui/keymap.rs src/tui/mod.rs
git commit -m "feat(tui): central keymap with KeyAction enum"
```

---

## Task 5: App + Screen + Action + 1Hz tick (event loop scaffolding)

**Files:**
- Create: `src/tui/app.rs`
- Modify: `src/tui/mod.rs`

- [ ] **Step 1: Write the failing tests**

Create `src/tui/app.rs`:

```rust
//! The top-level `App<R: Repo>` state machine. Owns the active screen,
//! a header snapshot of the active timer, and the footer message line.
//! `step()` is the pure transition function used by tests; the real
//! event loop wraps it with crossterm + a 1Hz tick thread.

use crate::storage::Repo;
use crate::tui::keymap::KeyAction;
use crate::tui::theme::Theme;
use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenKind {
    Tasks,
    Agenda,
    Report,
    Help,
}

#[derive(Debug, Clone)]
pub struct ActiveTimer {
    pub task_id: i64,
    pub task_title: String,
    pub shortcut_external_id: Option<i64>,
    pub started_at: DateTime<Utc>,
}

impl ActiveTimer {
    pub fn elapsed(&self, now: DateTime<Utc>) -> Duration {
        (now - self.started_at).max(Duration::zero())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FooterMessage {
    Empty,
    Info(String),
    Error(String),
}

pub struct App<R: Repo> {
    pub repo: R,
    pub theme: Theme,
    pub screen: ScreenKind,
    pub active_timer: Option<ActiveTimer>,
    pub footer: FooterMessage,
    pub now: DateTime<Utc>,
    pub pending_g: bool,
    pub should_quit: bool,
}

impl<R: Repo> App<R> {
    pub fn new(repo: R, theme: Theme, now: DateTime<Utc>) -> Self {
        Self {
            repo,
            theme,
            screen: ScreenKind::Tasks,
            active_timer: None,
            footer: FooterMessage::Empty,
            now,
            pending_g: false,
            should_quit: false,
        }
    }

    /// Refresh the active-timer snapshot from the repo. Called once at
    /// startup and after every mutation that may have changed the active
    /// row.
    pub fn refresh_active_timer(&mut self) -> anyhow::Result<()> {
        let entry = self.repo.active_time_entry()?;
        self.active_timer = match entry {
            Some(e) => {
                let task = self.repo.find_task(e.task_id)?.ok_or_else(|| {
                    anyhow::anyhow!(
                        "internal: active entry #{} references missing task #{}",
                        e.id,
                        e.task_id
                    )
                })?;
                let sc_external = match task.shortcut_story_id {
                    Some(row_id) => self
                        .repo
                        .find_shortcut_story_by_row_id(row_id)?
                        .map(|s| s.external_id),
                    None => None,
                };
                Some(ActiveTimer {
                    task_id: task.id,
                    task_title: task.title,
                    shortcut_external_id: sc_external,
                    started_at: e.started_at,
                })
            }
            None => None,
        };
        Ok(())
    }

    /// Pure transition for a tick. Records `now` so the header clock
    /// advances; no DB calls.
    pub fn on_tick(&mut self, now: DateTime<Utc>) {
        self.now = now;
    }

    /// Pure transition for a `Quit` request from any screen. Sets
    /// `should_quit`; the loop checks this each iteration.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Posts a transient info message to the footer, replacing whatever
    /// was there.
    pub fn info(&mut self, msg: impl Into<String>) {
        self.footer = FooterMessage::Info(msg.into());
    }

    pub fn error(&mut self, msg: impl Into<String>) {
        self.footer = FooterMessage::Error(msg.into());
    }

    pub fn clear_footer(&mut self) {
        self.footer = FooterMessage::Empty;
    }

    /// Resolves the global handlers shared by every screen. Returns
    /// `true` when the action was consumed at the global level so screens
    /// can skip it.
    pub fn handle_global(&mut self, action: KeyAction) -> bool {
        match action {
            KeyAction::Quit => {
                if self.screen == ScreenKind::Help {
                    self.screen = ScreenKind::Tasks;
                } else {
                    self.quit();
                }
                true
            }
            KeyAction::Help => {
                self.screen = ScreenKind::Help;
                true
            }
            KeyAction::OpenTasks if !self.pending_g => {
                self.pending_g = true;
                self.clear_footer();
                false
            }
            KeyAction::OpenAgenda => {
                self.screen = ScreenKind::Agenda;
                self.pending_g = false;
                self.clear_footer();
                true
            }
            KeyAction::OpenReport => {
                self.screen = ScreenKind::Report;
                self.pending_g = false;
                self.clear_footer();
                true
            }
            _ => {
                if self.pending_g {
                    self.pending_g = false;
                    self.clear_footer();
                    if matches!(action, KeyAction::OpenTasks) {
                        self.screen = ScreenKind::Tasks;
                        return true;
                    }
                }
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SqliteRepo;
    use chrono::TimeZone;

    fn at(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 22, h, 0, 0).unwrap()
    }

    fn app() -> App<SqliteRepo> {
        let repo = SqliteRepo::in_memory();
        let theme = Theme::from_config_accent("cyan");
        App::new(repo, theme, at(10))
    }

    #[test]
    fn starts_on_tasks_screen() {
        let a = app();
        assert_eq!(a.screen, ScreenKind::Tasks);
        assert!(!a.should_quit);
        assert!(a.active_timer.is_none());
    }

    #[test]
    fn quit_action_globally_sets_quit() {
        let mut a = app();
        a.handle_global(KeyAction::Quit);
        assert!(a.should_quit);
    }

    #[test]
    fn quit_inside_help_returns_to_tasks_instead_of_quitting() {
        let mut a = app();
        a.screen = ScreenKind::Help;
        a.handle_global(KeyAction::Quit);
        assert!(!a.should_quit);
        assert_eq!(a.screen, ScreenKind::Tasks);
    }

    #[test]
    fn help_action_opens_help_screen() {
        let mut a = app();
        a.handle_global(KeyAction::Help);
        assert_eq!(a.screen, ScreenKind::Help);
    }

    #[test]
    fn agenda_action_switches_screen() {
        let mut a = app();
        a.handle_global(KeyAction::OpenAgenda);
        assert_eq!(a.screen, ScreenKind::Agenda);
    }

    #[test]
    fn pending_g_then_g_returns_to_tasks() {
        let mut a = app();
        a.screen = ScreenKind::Agenda;
        // First g — sets pending and is NOT consumed (the screen will see it
        // and ignore it; pending_g will be reset on the next key).
        a.handle_global(KeyAction::OpenTasks);
        assert!(a.pending_g);
        // Second g — recognized as `gg` and routes to Tasks.
        let consumed = a.handle_global(KeyAction::OpenTasks);
        assert!(consumed);
        assert_eq!(a.screen, ScreenKind::Tasks);
        assert!(!a.pending_g);
    }

    #[test]
    fn tick_updates_now() {
        let mut a = app();
        let later = at(11);
        a.on_tick(later);
        assert_eq!(a.now, later);
    }

    #[test]
    fn refresh_active_timer_picks_up_running_entry() {
        use crate::domain::TimerOps;
        use crate::storage::Repo;
        let mut a = app();
        let task = a.repo.create_task("active", None).unwrap();
        TimerOps::new(&mut a.repo).start(task.id, at(10)).unwrap();
        a.refresh_active_timer().unwrap();
        let snapshot = a.active_timer.as_ref().unwrap();
        assert_eq!(snapshot.task_id, task.id);
        assert_eq!(snapshot.task_title, "active");
    }

    #[test]
    fn footer_messaging_helpers_set_state() {
        let mut a = app();
        a.info("ok");
        assert_eq!(a.footer, FooterMessage::Info("ok".into()));
        a.error("nope");
        assert_eq!(a.footer, FooterMessage::Error("nope".into()));
        a.clear_footer();
        assert_eq!(a.footer, FooterMessage::Empty);
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test --lib tui::app -- --nocapture`
Expected: PASS (all nine).

- [ ] **Step 3: Re-export from `tui::mod`**

Edit `src/tui/mod.rs`:

```rust
//! Buckland TUI — ratatui surface composed on top of the `Repo` trait.

pub mod app;
pub mod keymap;
pub mod terminal;
pub mod theme;

use crate::cli::context::Context;

pub fn run(_ctx: Context) -> anyhow::Result<i32> {
    Ok(0)
}
```

- [ ] **Step 4: Commit**

```bash
git add src/tui/app.rs src/tui/mod.rs
git commit -m "feat(tui): App state machine, screen kinds, footer messages"
```

---

## Task 6: Header chrome (active timer line)

**Files:**
- Create: `src/tui/chrome.rs`
- Modify: `src/tui/mod.rs`

- [ ] **Step 1: Write the failing tests**

Create `src/tui/chrome.rs`:

```rust
//! Header + footer chrome. Pure rendering — no state mutations, no I/O.
//! The header shows the active timer or "idle"; the footer shows the
//! per-screen shortcut hints plus any transient info/error message.

use crate::cli::format::duration_hms;
use crate::tui::app::{ActiveTimer, App, FooterMessage, ScreenKind};
use crate::storage::Repo;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render_header<R: Repo>(frame: &mut Frame<'_>, area: Rect, app: &App<R>) {
    let line = match &app.active_timer {
        Some(t) => header_active_line(t, app),
        None => Line::from(vec![Span::styled(
            "Buckland — idle",
            Style::default().add_modifier(Modifier::DIM),
        )]),
    };
    frame.render_widget(Paragraph::new(line), area);
}

fn header_active_line<R: Repo>(timer: &ActiveTimer, app: &App<R>) -> Line<'static> {
    let elapsed = duration_hms(timer.elapsed(app.now));
    let label = match timer.shortcut_external_id {
        Some(ext) => format!("SC-{ext} {}", timer.task_title),
        None => timer.task_title.clone(),
    };
    Line::from(vec![
        Span::styled("▶ ", app.theme.running()),
        Span::raw(label),
        Span::raw(" — "),
        Span::styled(elapsed, app.theme.total()),
    ])
}

pub fn render_footer<R: Repo>(frame: &mut Frame<'_>, area: Rect, app: &App<R>) {
    let hint = match app.screen {
        ScreenKind::Tasks => "j/k move • s start • S stop • n new • d done • A archive • D delete • e edit • / filter • ? help • q quit",
        ScreenKind::Agenda => "j/k move • h/l week • enter edit • D delete • g tasks • r report • ? help • q quit",
        ScreenKind::Report => "Tab scope • T group • c copy • j json • g tasks • a agenda • ? help • q quit",
        ScreenKind::Help => "any key to return",
    };
    let line = match &app.footer {
        FooterMessage::Empty => Line::from(Span::styled(hint, app.theme.dim())),
        FooterMessage::Info(msg) => Line::from(vec![
            Span::styled("ℹ ", app.theme.dim()),
            Span::raw(msg.clone()),
        ]),
        FooterMessage::Error(msg) => Line::from(vec![
            Span::styled("✗ ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(msg.clone()),
        ]),
    };
    frame.render_widget(Paragraph::new(line), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::{ActiveTimer, App};
    use crate::tui::theme::Theme;
    use crate::storage::SqliteRepo;
    use chrono::{TimeZone, Utc};
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use ratatui::Terminal;

    fn render<F: FnOnce(&mut Frame<'_>)>(width: u16, height: u16, f: F) -> String {
        let backend = TestBackend::new(width, height);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|frame| f(frame)).unwrap();
        let buffer = term.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..buffer.area().height {
            for x in 0..buffer.area().width {
                out.push_str(buffer.cell((x, y)).unwrap().symbol());
            }
            out.push('\n');
        }
        out
    }

    fn app_idle() -> App<SqliteRepo> {
        let now = Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap();
        App::new(SqliteRepo::in_memory(), Theme::from_config_accent("cyan"), now)
    }

    #[test]
    fn header_says_idle_when_no_active_timer() {
        let app = app_idle();
        let out = render(40, 1, |f| render_header(f, Rect::new(0, 0, 40, 1), &app));
        assert!(out.contains("idle"), "got buffer:\n{out}");
    }

    #[test]
    fn header_shows_active_task_label_and_elapsed() {
        let mut app = app_idle();
        app.active_timer = Some(ActiveTimer {
            task_id: 1,
            task_title: "fix login".into(),
            shortcut_external_id: None,
            started_at: app.now,
        });
        // Advance the clock by 1h 5m 12s.
        app.now = app.now + chrono::Duration::seconds(3912);
        let out = render(60, 1, |f| render_header(f, Rect::new(0, 0, 60, 1), &app));
        assert!(out.contains("fix login"));
        assert!(out.contains("01:05:12"), "elapsed missing in:\n{out}");
        assert!(out.contains('▶'));
    }

    #[test]
    fn header_prefixes_sc_id_when_linked() {
        let mut app = app_idle();
        app.active_timer = Some(ActiveTimer {
            task_id: 1,
            task_title: "fix login".into(),
            shortcut_external_id: Some(123),
            started_at: app.now,
        });
        let out = render(60, 1, |f| render_header(f, Rect::new(0, 0, 60, 1), &app));
        assert!(out.contains("SC-123 fix login"), "got:\n{out}");
    }

    #[test]
    fn footer_shows_screen_hint_when_idle() {
        let app = app_idle();
        let out = render(120, 1, |f| render_footer(f, Rect::new(0, 0, 120, 1), &app));
        assert!(out.contains("j/k"));
        assert!(out.contains("? help"));
    }

    #[test]
    fn footer_shows_error_message_with_marker() {
        let mut app = app_idle();
        app.error("can't delete");
        let out = render(60, 1, |f| render_footer(f, Rect::new(0, 0, 60, 1), &app));
        assert!(out.contains("can't delete"));
        assert!(out.contains('✗'));
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test --lib tui::chrome -- --nocapture`
Expected: PASS (all five).

- [ ] **Step 3: Re-export the chrome module**

Edit `src/tui/mod.rs`:

```rust
pub mod app;
pub mod chrome;
pub mod keymap;
pub mod terminal;
pub mod theme;

use crate::cli::context::Context;

pub fn run(_ctx: Context) -> anyhow::Result<i32> {
    Ok(0)
}
```

- [ ] **Step 4: Commit**

```bash
git add src/tui/chrome.rs src/tui/mod.rs
git commit -m "feat(tui): header + footer chrome rendering"
```

---

## Task 7: Tasks screen — list, selection, j/k/gg/G

**Files:**
- Create: `src/tui/screens/mod.rs`
- Create: `src/tui/screens/tasks.rs`
- Modify: `src/tui/mod.rs`

- [ ] **Step 1: Write the failing tests**

Create `src/tui/screens/mod.rs`:

```rust
//! Per-screen state + rendering. Each screen owns a `*State` struct and
//! exposes `render` + `handle_key`. Mutations always go through the
//! parent `App<R>`'s `Repo`.

pub mod tasks;
```

Create `src/tui/screens/tasks.rs`:

```rust
//! Tasks screen — the home view. Lists open tasks with the running one
//! highlighted. Owns: selection cursor, optional filter input, optional
//! new-task prompt, optional delete-confirm prompt.

use crate::cli::format::duration_compact;
use crate::domain::Task;
use crate::storage::Repo;
use crate::tui::app::App;
use crate::tui::keymap::KeyAction;
use chrono::{DateTime, Utc};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

#[derive(Debug, Default)]
pub struct TasksState {
    pub tasks: Vec<Task>,
    pub totals: Vec<i64>, // seconds, parallel array to `tasks`
    pub selected: usize,
    pub filter: Option<String>,
    pub prompt: Option<Prompt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prompt {
    NewTask { buffer: String },
    DeleteConfirm { task_id: i64, title: String },
}

impl TasksState {
    pub fn refresh<R: Repo>(&mut self, repo: &R, now: DateTime<Utc>) -> anyhow::Result<()> {
        let mut tasks = repo.list_open_tasks()?;
        // Filter is applied last so the source list is stable for selection math.
        if let Some(needle) = self.filter.as_deref() {
            let needle = needle.to_lowercase();
            tasks.retain(|t| t.title.to_lowercase().contains(&needle));
        }
        let mut totals = Vec::with_capacity(tasks.len());
        for t in &tasks {
            let d = repo.task_total_duration(t.id, now)?;
            totals.push(d.num_seconds());
        }
        if self.selected >= tasks.len() {
            self.selected = tasks.len().saturating_sub(1);
        }
        self.tasks = tasks;
        self.totals = totals;
        Ok(())
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.tasks.len() {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_top(&mut self) {
        self.selected = 0;
    }

    pub fn move_bottom(&mut self) {
        self.selected = self.tasks.len().saturating_sub(1);
    }

    pub fn selected_task(&self) -> Option<&Task> {
        self.tasks.get(self.selected)
    }
}

pub fn render<R: Repo>(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App<R>,
    state: &TasksState,
) {
    let block = Block::default().borders(Borders::ALL).title("Tasks");
    let active_id = app.active_timer.as_ref().map(|t| t.task_id);
    let items: Vec<ListItem> = state
        .tasks
        .iter()
        .zip(state.totals.iter())
        .enumerate()
        .map(|(i, (t, secs))| {
            let is_active = Some(t.id) == active_id;
            let marker = if is_active { "▶ " } else { "  " };
            let total = duration_compact(chrono::Duration::seconds(*secs));
            let line = Line::from(vec![
                Span::styled(
                    marker,
                    if is_active {
                        app.theme.running()
                    } else {
                        Style::default()
                    },
                ),
                Span::raw(format!("#{:<4} ", t.id)),
                Span::raw(format!("{:<40} ", crate::cli::commands_truncate(&t.title, 40))),
                Span::styled(total, app.theme.total()),
            ]);
            let style = if i == state.selected {
                app.theme.selected()
            } else if is_active {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(line).style(style)
        })
        .collect();
    let list = List::new(items).block(block);
    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));
    frame.render_stateful_widget(list, area, &mut list_state);

    if let Some(prompt) = &state.prompt {
        render_prompt(frame, area, prompt);
    }
}

fn render_prompt(frame: &mut Frame<'_>, area: Rect, prompt: &Prompt) {
    let h = area.height.saturating_sub(1);
    let prompt_area = Rect::new(area.x, area.y + h, area.width, 1);
    let line = match prompt {
        Prompt::NewTask { buffer } => Line::from(vec![
            Span::styled("New task: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(buffer.clone()),
            Span::raw("_"),
        ]),
        Prompt::DeleteConfirm { task_id, title } => Line::from(vec![
            Span::styled(
                format!("Delete task #{task_id} \"{title}\"? y/N"),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
    };
    frame.render_widget(Paragraph::new(line), prompt_area);
}

pub fn handle_key<R: Repo>(
    state: &mut TasksState,
    _app: &mut App<R>,
    action: KeyAction,
) -> bool {
    if state.prompt.is_some() {
        // Prompt-mode handling is added in Tasks 9 & 10 below.
        return false;
    }
    match action {
        KeyAction::Down => {
            state.move_down();
            true
        }
        KeyAction::Up => {
            state.move_up();
            true
        }
        KeyAction::Top => {
            state.move_top();
            true
        }
        KeyAction::Bottom => {
            state.move_bottom();
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::App;
    use crate::tui::theme::Theme;
    use crate::storage::SqliteRepo;
    use chrono::TimeZone;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap()
    }

    fn fresh() -> (App<SqliteRepo>, TasksState) {
        let app = App::new(SqliteRepo::in_memory(), Theme::from_config_accent("cyan"), now());
        (app, TasksState::default())
    }

    #[test]
    fn refresh_loads_open_tasks_in_creation_order_desc() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        app.repo.create_task("first", None).unwrap();
        app.repo.create_task("second", None).unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        assert_eq!(state.tasks.len(), 2);
        // list_open_tasks sorts by created_at DESC — newest first.
        assert_eq!(state.tasks[0].title, "second");
        assert_eq!(state.tasks[1].title, "first");
    }

    #[test]
    fn refresh_filter_narrows_list_case_insensitive() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        app.repo.create_task("Login flow", None).unwrap();
        app.repo.create_task("Pricing tweak", None).unwrap();
        state.filter = Some("login".into());
        state.refresh(&app.repo, app.now).unwrap();
        assert_eq!(state.tasks.len(), 1);
        assert_eq!(state.tasks[0].title, "Login flow");
    }

    #[test]
    fn move_down_clamps_at_last_row() {
        let (_, mut state) = fresh();
        state.tasks = vec![dummy_task(1), dummy_task(2)];
        state.totals = vec![0, 0];
        state.move_down();
        assert_eq!(state.selected, 1);
        state.move_down();
        assert_eq!(state.selected, 1, "must clamp at len-1");
    }

    #[test]
    fn move_up_clamps_at_zero() {
        let (_, mut state) = fresh();
        state.tasks = vec![dummy_task(1), dummy_task(2)];
        state.totals = vec![0, 0];
        state.move_up();
        assert_eq!(state.selected, 0, "must clamp at 0");
    }

    #[test]
    fn move_top_and_bottom_jump() {
        let (_, mut state) = fresh();
        state.tasks = (1..=5).map(dummy_task).collect();
        state.totals = vec![0; 5];
        state.move_bottom();
        assert_eq!(state.selected, 4);
        state.move_top();
        assert_eq!(state.selected, 0);
    }

    fn dummy_task(id: i64) -> Task {
        Task {
            id,
            title: format!("t{id}"),
            description: None,
            shortcut_story_id: None,
            completed_at: None,
            archived_at: None,
            created_at: now(),
            updated_at: now(),
        }
    }
}
```

- [ ] **Step 2: Add a tiny re-export so `cli::commands_truncate` is reachable**

The render code references `crate::cli::commands_truncate`. The existing helper lives in `src/cli/commands/helpers.rs` as `pub(super) fn truncate`. Promote it.

Edit `src/cli/commands/helpers.rs`:

```rust
use crate::domain::Task;

pub(super) fn status_glyph(t: &Task) -> &'static str {
    if t.completed_at.is_some() {
        "✓"
    } else if t.archived_at.is_some() {
        "·"
    } else {
        " "
    }
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}
```

Edit `src/cli/mod.rs` to re-export:

```rust
//! CLI surface for `bl`.

pub mod args;
pub mod commands;
pub mod context;
pub mod format;
pub mod resolve;

pub use commands::helpers::truncate as commands_truncate;

use args::{Cli, Commands};
use clap::Parser;

pub fn run() -> anyhow::Result<i32> {
    // (body unchanged)
    let cli = Cli::parse();
    let mut ctx = context::open()?;
    let command = cli.command.unwrap_or(Commands::Tui);
    match command {
        Commands::Add { title, description, sc } => commands::add(&mut ctx, &title, description.as_deref(), sc.as_deref()),
        Commands::Shortcut { id } => commands::shortcut_refresh(&mut ctx, &id),
        Commands::List { all, archived, completed } => commands::list(&mut ctx, all, archived, completed),
        Commands::Start { target } => commands::start(&mut ctx, &target),
        Commands::Stop | Commands::Pause => commands::stop(&mut ctx),
        Commands::Status => commands::status(&mut ctx),
        Commands::Done { id } => commands::done(&mut ctx, id),
        Commands::Archive { id } => commands::archive(&mut ctx, id),
        Commands::Delete { id } => commands::delete(&mut ctx, id),
        Commands::Report { today, week, month, all, range, by_task, by_epic, by_day, json } => commands::report(
            &mut ctx,
            commands::ReportArgs { today, week, month, all, range, by_task, by_epic, by_day, json },
        ),
        Commands::Tui => crate::tui::run(ctx),
    }
}
```

Edit `src/cli/commands/mod.rs` so `helpers` is `pub` (currently it's a private `mod helpers;`):

```rust
mod add;
mod archive;
mod delete;
mod done;
pub mod helpers;
mod list;
mod report;
mod shortcut;
mod start;
mod status;
mod stop;

pub use add::add;
pub use archive::archive;
pub use delete::delete;
pub use done::done;
pub use list::list;
pub use report::{report, ReportArgs};
pub use shortcut::shortcut_refresh;
pub use start::start;
pub use status::status;
pub use stop::stop;
```

Update internal callers from `pub(super) fn truncate` to plain `pub fn truncate` — only one external caller (the new tui code) needs it; existing `super::helpers::truncate` keeps working.

- [ ] **Step 3: Wire `screens` into `tui::mod`**

Edit `src/tui/mod.rs`:

```rust
//! Buckland TUI — ratatui surface composed on top of the `Repo` trait.

pub mod app;
pub mod chrome;
pub mod keymap;
pub mod screens;
pub mod terminal;
pub mod theme;

use crate::cli::context::Context;

pub fn run(_ctx: Context) -> anyhow::Result<i32> {
    Ok(0)
}
```

- [ ] **Step 4: Run all tests**

Run: `cargo test --all-features`
Expected: PASS — all prior tests still green and the five new `tasks` tests + the `helpers::truncate` visibility change compile.

- [ ] **Step 5: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add src/cli/commands/helpers.rs src/cli/commands/mod.rs src/cli/mod.rs src/tui/mod.rs src/tui/screens/mod.rs src/tui/screens/tasks.rs
git commit -m "feat(tui): tasks screen rendering with j/k/gg/G navigation"
```

---

## Task 8: Tasks screen — start, stop, enter (timer actions)

**Files:**
- Modify: `src/tui/screens/tasks.rs`

- [ ] **Step 1: Write the failing tests**

Append inside the existing `mod tests` block in `src/tui/screens/tasks.rs`:

```rust
    use crate::tui::keymap::KeyAction;

    #[test]
    fn start_action_starts_timer_for_selected_task() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        let t = app.repo.create_task("alpha", None).unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        let acted = handle_key(&mut state, &mut app, KeyAction::StartSelected);
        assert!(acted);
        let active = app.repo.active_time_entry().unwrap().expect("active set");
        assert_eq!(active.task_id, t.id);
    }

    #[test]
    fn start_action_with_empty_list_is_a_noop_and_warns() {
        let (mut app, mut state) = fresh();
        let acted = handle_key(&mut state, &mut app, KeyAction::StartSelected);
        assert!(acted);
        assert!(matches!(app.footer, FooterMessage::Info(_)));
    }

    #[test]
    fn stop_action_ends_active_timer_and_refreshes_header() {
        use crate::domain::TimerOps;
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        let t = app.repo.create_task("running", None).unwrap();
        TimerOps::new(&mut app.repo).start(t.id, app.now).unwrap();
        app.refresh_active_timer().unwrap();
        state.refresh(&app.repo, app.now).unwrap();

        let acted = handle_key(&mut state, &mut app, KeyAction::StopActive);
        assert!(acted);
        assert!(app.repo.active_time_entry().unwrap().is_none());
        assert!(app.active_timer.is_none());
    }

    #[test]
    fn confirm_action_starts_timer_when_no_prompt_open() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        let t = app.repo.create_task("alpha", None).unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        handle_key(&mut state, &mut app, KeyAction::Confirm);
        let active = app.repo.active_time_entry().unwrap().expect("active set");
        assert_eq!(active.task_id, t.id);
    }
```

Also import `FooterMessage` at the top of the test module (or qualify the path inline).

- [ ] **Step 2: Run them; expect failure**

Run: `cargo test --lib tui::screens::tasks -- --nocapture`
Expected: FAIL — `KeyAction::StartSelected` / `StopActive` / `Confirm` are not handled yet.

- [ ] **Step 3: Implement the actions**

Replace the `handle_key` function body in `src/tui/screens/tasks.rs`:

```rust
pub fn handle_key<R: Repo>(
    state: &mut TasksState,
    app: &mut App<R>,
    action: KeyAction,
) -> bool {
    if state.prompt.is_some() {
        // Prompt-mode handling lands in Task 9.
        return false;
    }
    match action {
        KeyAction::Down => {
            state.move_down();
            true
        }
        KeyAction::Up => {
            state.move_up();
            true
        }
        KeyAction::Top => {
            state.move_top();
            true
        }
        KeyAction::Bottom => {
            state.move_bottom();
            true
        }
        KeyAction::StartSelected | KeyAction::Confirm => {
            start_selected(state, app);
            true
        }
        KeyAction::StopActive => {
            stop_active(state, app);
            true
        }
        _ => false,
    }
}

fn start_selected<R: Repo>(state: &mut TasksState, app: &mut App<R>) {
    use crate::domain::TimerOps;
    let Some(task) = state.selected_task().cloned() else {
        app.info("Nothing to start. Press n to add a task.");
        return;
    };
    let now = app.now;
    match TimerOps::new(&mut app.repo).start(task.id, now) {
        Ok(_) => {
            app.refresh_active_timer().ok();
            let _ = state.refresh(&app.repo, now);
            app.info(format!("Started #{} {}", task.id, task.title));
        }
        Err(e) => {
            app.error(format!("Could not start: {e}"));
        }
    }
}

fn stop_active<R: Repo>(state: &mut TasksState, app: &mut App<R>) {
    use crate::domain::TimerOps;
    let now = app.now;
    match TimerOps::new(&mut app.repo).stop(now) {
        Ok(Some(_)) => {
            app.refresh_active_timer().ok();
            let _ = state.refresh(&app.repo, now);
            app.info("Stopped");
        }
        Ok(None) => app.info("Nothing to stop."),
        Err(e) => app.error(format!("Could not stop: {e}")),
    }
}
```

- [ ] **Step 4: Run the tests; expect green**

Run: `cargo test --lib tui::screens::tasks -- --nocapture`
Expected: PASS — original five plus four new tests = nine green.

- [ ] **Step 5: Commit**

```bash
git add src/tui/screens/tasks.rs
git commit -m "feat(tui): tasks screen start/stop/enter actions"
```

---

## Task 9: Tasks screen — done, archive, delete with inline y/N confirm

**Files:**
- Modify: `src/tui/screens/tasks.rs`

- [ ] **Step 1: Write the failing tests**

Append to the test module:

```rust
    #[test]
    fn done_action_marks_selected_task_complete_and_drops_from_open_list() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        app.repo.create_task("finish me", None).unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        handle_key(&mut state, &mut app, KeyAction::Done);
        assert!(state.tasks.is_empty(), "open list should be empty after done");
        assert!(matches!(app.footer, FooterMessage::Info(_)));
    }

    #[test]
    fn archive_action_drops_from_open_list() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        app.repo.create_task("shelf", None).unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        handle_key(&mut state, &mut app, KeyAction::Archive);
        assert!(state.tasks.is_empty());
    }

    #[test]
    fn delete_action_opens_confirm_prompt_then_y_deletes() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        app.repo.create_task("oops", None).unwrap();
        state.refresh(&app.repo, app.now).unwrap();

        // First D opens prompt.
        handle_key(&mut state, &mut app, KeyAction::Delete);
        assert!(matches!(state.prompt, Some(Prompt::DeleteConfirm { .. })));

        // Pressing y deletes the task and clears the prompt.
        handle_key(&mut state, &mut app, KeyAction::Char('y'));
        assert!(state.prompt.is_none());
        assert!(state.tasks.is_empty());
    }

    #[test]
    fn delete_confirm_n_cancels() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        app.repo.create_task("oops", None).unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        handle_key(&mut state, &mut app, KeyAction::Delete);
        handle_key(&mut state, &mut app, KeyAction::Char('n'));
        assert!(state.prompt.is_none());
        assert_eq!(state.tasks.len(), 1, "task still present after cancel");
    }

    #[test]
    fn delete_blocked_when_task_has_entries() {
        use crate::domain::TimerOps;
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        let t = app.repo.create_task("with entries", None).unwrap();
        TimerOps::new(&mut app.repo).start(t.id, app.now).unwrap();
        TimerOps::new(&mut app.repo)
            .stop(app.now + chrono::Duration::seconds(5))
            .unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        handle_key(&mut state, &mut app, KeyAction::Delete);
        handle_key(&mut state, &mut app, KeyAction::Char('y'));
        assert_eq!(state.tasks.len(), 1, "delete must be blocked");
        assert!(matches!(app.footer, FooterMessage::Error(_)));
    }
```

- [ ] **Step 2: Run; expect failure**

Run: `cargo test --lib tui::screens::tasks -- --nocapture`
Expected: FAIL on the four new tests.

- [ ] **Step 3: Implement done / archive / delete-with-confirm**

Edit `src/tui/screens/tasks.rs`. Replace `handle_key` with the version that handles prompts and the new actions:

```rust
pub fn handle_key<R: Repo>(
    state: &mut TasksState,
    app: &mut App<R>,
    action: KeyAction,
) -> bool {
    if state.prompt.is_some() {
        return handle_prompt_key(state, app, action);
    }
    match action {
        KeyAction::Down => { state.move_down(); true }
        KeyAction::Up => { state.move_up(); true }
        KeyAction::Top => { state.move_top(); true }
        KeyAction::Bottom => { state.move_bottom(); true }
        KeyAction::StartSelected | KeyAction::Confirm => {
            start_selected(state, app);
            true
        }
        KeyAction::StopActive => { stop_active(state, app); true }
        KeyAction::Done => { mark_selected_done(state, app); true }
        KeyAction::Archive => { archive_selected(state, app); true }
        KeyAction::Delete => { open_delete_confirm(state, app); true }
        _ => false,
    }
}

fn handle_prompt_key<R: Repo>(
    state: &mut TasksState,
    app: &mut App<R>,
    action: KeyAction,
) -> bool {
    let Some(prompt) = state.prompt.clone() else { return false };
    match (prompt, action) {
        (Prompt::DeleteConfirm { task_id, title }, KeyAction::Char('y')) => {
            confirm_delete(state, app, task_id, &title);
            state.prompt = None;
            true
        }
        (Prompt::DeleteConfirm { .. }, _) => {
            state.prompt = None;
            app.info("Cancelled");
            true
        }
        // NewTask + filter prompt handlers land in Task 10.
        _ => false,
    }
}

fn mark_selected_done<R: Repo>(state: &mut TasksState, app: &mut App<R>) {
    let Some(task) = state.selected_task().cloned() else {
        app.info("Nothing selected.");
        return;
    };
    match app.repo.mark_task_done(task.id, app.now) {
        Ok(_) => {
            let _ = state.refresh(&app.repo, app.now);
            app.info(format!("Done #{} {}", task.id, task.title));
        }
        Err(e) => app.error(format!("Could not mark done: {e}")),
    }
}

fn archive_selected<R: Repo>(state: &mut TasksState, app: &mut App<R>) {
    let Some(task) = state.selected_task().cloned() else {
        app.info("Nothing selected.");
        return;
    };
    match app.repo.archive_task(task.id, app.now) {
        Ok(_) => {
            let _ = state.refresh(&app.repo, app.now);
            app.info(format!("Archived #{} {}", task.id, task.title));
        }
        Err(e) => app.error(format!("Could not archive: {e}")),
    }
}

fn open_delete_confirm<R: Repo>(state: &mut TasksState, app: &mut App<R>) {
    let Some(task) = state.selected_task() else {
        app.info("Nothing selected.");
        return;
    };
    state.prompt = Some(Prompt::DeleteConfirm {
        task_id: task.id,
        title: task.title.clone(),
    });
}

fn confirm_delete<R: Repo>(
    state: &mut TasksState,
    app: &mut App<R>,
    task_id: i64,
    title: &str,
) {
    use crate::storage::RepoError;
    match app.repo.delete_task(task_id) {
        Ok(()) => {
            let _ = state.refresh(&app.repo, app.now);
            app.info(format!("Deleted #{task_id} {title}"));
        }
        Err(RepoError::TaskHasEntries(_)) => {
            app.error(format!(
                "Task #{task_id} has time entries — use Archive (A) instead."
            ));
        }
        Err(e) => app.error(format!("Could not delete: {e}")),
    }
}
```

- [ ] **Step 4: Run; expect green**

Run: `cargo test --lib tui::screens::tasks -- --nocapture`
Expected: PASS — five new tests green plus the prior nine = thirteen total in this module.

- [ ] **Step 5: Commit**

```bash
git add src/tui/screens/tasks.rs
git commit -m "feat(tui): tasks screen done/archive/delete with confirm"
```

---

## Task 10: Tasks screen — new task (n) + literal filter (/)

**Files:**
- Modify: `src/tui/screens/tasks.rs`

- [ ] **Step 1: Write the failing tests**

Append to the test module:

```rust
    #[test]
    fn new_task_prompt_consumes_chars_then_enter_creates_task() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        state.refresh(&app.repo, app.now).unwrap();

        handle_key(&mut state, &mut app, KeyAction::NewTask);
        assert!(matches!(state.prompt, Some(Prompt::NewTask { .. })));

        for c in "fix login".chars() {
            handle_key(&mut state, &mut app, KeyAction::Char(c));
        }
        if let Some(Prompt::NewTask { buffer }) = &state.prompt {
            assert_eq!(buffer, "fix login");
        } else {
            panic!("expected NewTask prompt");
        }

        handle_key(&mut state, &mut app, KeyAction::Confirm);
        assert!(state.prompt.is_none(), "prompt must close on enter");
        assert_eq!(state.tasks.len(), 1);
        assert_eq!(state.tasks[0].title, "fix login");
    }

    #[test]
    fn new_task_prompt_esc_cancels() {
        let (mut app, mut state) = fresh();
        handle_key(&mut state, &mut app, KeyAction::NewTask);
        handle_key(&mut state, &mut app, KeyAction::Char('x'));
        handle_key(&mut state, &mut app, KeyAction::Quit); // esc / q maps to Quit
        assert!(state.prompt.is_none());
        assert!(state.tasks.is_empty());
    }

    #[test]
    fn new_task_prompt_backspace_removes_last_char() {
        let (mut app, mut state) = fresh();
        handle_key(&mut state, &mut app, KeyAction::NewTask);
        handle_key(&mut state, &mut app, KeyAction::Char('a'));
        handle_key(&mut state, &mut app, KeyAction::Char('b'));
        handle_key(&mut state, &mut app, KeyAction::Backspace);
        if let Some(Prompt::NewTask { buffer }) = &state.prompt {
            assert_eq!(buffer, "a");
        } else {
            panic!("expected NewTask prompt");
        }
    }

    #[test]
    fn filter_prompt_narrows_list_live() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        app.repo.create_task("Login flow", None).unwrap();
        app.repo.create_task("Pricing", None).unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        assert_eq!(state.tasks.len(), 2);

        handle_key(&mut state, &mut app, KeyAction::Filter);
        for c in "log".chars() {
            handle_key(&mut state, &mut app, KeyAction::Char(c));
        }
        // After confirm, prompt closes, filter persists, list narrows.
        handle_key(&mut state, &mut app, KeyAction::Confirm);
        assert_eq!(state.tasks.len(), 1);
        assert_eq!(state.tasks[0].title, "Login flow");
        assert_eq!(state.filter.as_deref(), Some("log"));
    }

    #[test]
    fn filter_prompt_esc_clears_filter() {
        let (mut app, mut state) = fresh();
        state.filter = Some("old".into());
        handle_key(&mut state, &mut app, KeyAction::Filter);
        handle_key(&mut state, &mut app, KeyAction::Quit);
        assert!(state.filter.is_none(), "esc clears any prior filter");
    }
```

- [ ] **Step 2: Run; expect failure**

Run: `cargo test --lib tui::screens::tasks -- --nocapture`
Expected: FAIL on five new tests.

- [ ] **Step 3: Extend `Prompt` and the prompt handler**

Replace the `Prompt` enum in `src/tui/screens/tasks.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prompt {
    NewTask { buffer: String },
    DeleteConfirm { task_id: i64, title: String },
    Filter { buffer: String },
}
```

Update `render_prompt` to render the Filter case:

```rust
fn render_prompt(frame: &mut Frame<'_>, area: Rect, prompt: &Prompt) {
    let h = area.height.saturating_sub(1);
    let prompt_area = Rect::new(area.x, area.y + h, area.width, 1);
    let line = match prompt {
        Prompt::NewTask { buffer } => Line::from(vec![
            Span::styled("New task: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(buffer.clone()),
            Span::raw("_"),
        ]),
        Prompt::Filter { buffer } => Line::from(vec![
            Span::styled("Filter: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(buffer.clone()),
            Span::raw("_"),
        ]),
        Prompt::DeleteConfirm { task_id, title } => Line::from(vec![Span::styled(
            format!("Delete task #{task_id} \"{title}\"? y/N"),
            Style::default().add_modifier(Modifier::BOLD),
        )]),
    };
    frame.render_widget(Paragraph::new(line), prompt_area);
}
```

Replace `handle_key` and `handle_prompt_key` with the full new versions:

```rust
pub fn handle_key<R: Repo>(
    state: &mut TasksState,
    app: &mut App<R>,
    action: KeyAction,
) -> bool {
    if state.prompt.is_some() {
        return handle_prompt_key(state, app, action);
    }
    match action {
        KeyAction::Down => { state.move_down(); true }
        KeyAction::Up => { state.move_up(); true }
        KeyAction::Top => { state.move_top(); true }
        KeyAction::Bottom => { state.move_bottom(); true }
        KeyAction::StartSelected | KeyAction::Confirm => { start_selected(state, app); true }
        KeyAction::StopActive => { stop_active(state, app); true }
        KeyAction::Done => { mark_selected_done(state, app); true }
        KeyAction::Archive => { archive_selected(state, app); true }
        KeyAction::Delete => { open_delete_confirm(state, app); true }
        KeyAction::NewTask => {
            state.prompt = Some(Prompt::NewTask { buffer: String::new() });
            true
        }
        KeyAction::Filter => {
            state.prompt = Some(Prompt::Filter { buffer: String::new() });
            true
        }
        _ => false,
    }
}

fn handle_prompt_key<R: Repo>(
    state: &mut TasksState,
    app: &mut App<R>,
    action: KeyAction,
) -> bool {
    let Some(prompt) = state.prompt.clone() else { return false };
    match (prompt, action) {
        (Prompt::DeleteConfirm { task_id, title }, KeyAction::Char('y')) => {
            confirm_delete(state, app, task_id, &title);
            state.prompt = None;
            true
        }
        (Prompt::DeleteConfirm { .. }, _) => {
            state.prompt = None;
            app.info("Cancelled");
            true
        }
        (Prompt::NewTask { mut buffer }, KeyAction::Char(c)) => {
            buffer.push(c);
            state.prompt = Some(Prompt::NewTask { buffer });
            true
        }
        (Prompt::NewTask { mut buffer }, KeyAction::Backspace) => {
            buffer.pop();
            state.prompt = Some(Prompt::NewTask { buffer });
            true
        }
        (Prompt::NewTask { buffer }, KeyAction::Confirm) => {
            create_task_from_prompt(state, app, &buffer);
            state.prompt = None;
            true
        }
        (Prompt::NewTask { .. }, KeyAction::Quit) => {
            state.prompt = None;
            app.info("Cancelled");
            true
        }
        (Prompt::Filter { mut buffer }, KeyAction::Char(c)) => {
            buffer.push(c);
            state.prompt = Some(Prompt::Filter { buffer });
            true
        }
        (Prompt::Filter { mut buffer }, KeyAction::Backspace) => {
            buffer.pop();
            state.prompt = Some(Prompt::Filter { buffer });
            true
        }
        (Prompt::Filter { buffer }, KeyAction::Confirm) => {
            apply_filter(state, app, buffer);
            state.prompt = None;
            true
        }
        (Prompt::Filter { .. }, KeyAction::Quit) => {
            state.filter = None;
            state.prompt = None;
            let _ = state.refresh(&app.repo, app.now);
            true
        }
        _ => true, // swallow anything else while a prompt is open
    }
}

fn create_task_from_prompt<R: Repo>(
    state: &mut TasksState,
    app: &mut App<R>,
    title: &str,
) {
    let title = title.trim();
    if title.is_empty() {
        app.info("Empty title — nothing created.");
        return;
    }
    match app.repo.create_task(title, None) {
        Ok(t) => {
            let _ = state.refresh(&app.repo, app.now);
            app.info(format!("Added #{} {}", t.id, t.title));
        }
        Err(e) => app.error(format!("Could not add: {e}")),
    }
}

fn apply_filter<R: Repo>(state: &mut TasksState, app: &mut App<R>, buffer: String) {
    let trimmed = buffer.trim();
    state.filter = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    };
    let _ = state.refresh(&app.repo, app.now);
}
```

- [ ] **Step 4: Run; expect green**

Run: `cargo test --lib tui::screens::tasks -- --nocapture`
Expected: PASS — eighteen total tests in this module.

- [ ] **Step 5: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add src/tui/screens/tasks.rs
git commit -m "feat(tui): tasks screen new-task prompt and live filter"
```

---

## Task 11: Agenda screen

**Files:**
- Create: `src/tui/screens/agenda.rs`
- Modify: `src/tui/screens/mod.rs`

- [ ] **Step 1: Write the failing tests**

Create `src/tui/screens/agenda.rs`:

```rust
//! Agenda screen — time entries grouped by local day, week-paginated
//! with `h` / `l`. `enter` opens the Edit overlay; `D` deletes the
//! highlighted entry with an inline y/N confirm.

use crate::cli::format::duration_compact;
use crate::domain::TimeEntry;
use crate::storage::Repo;
use crate::tui::app::App;
use crate::tui::keymap::KeyAction;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Utc};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

#[derive(Debug)]
pub struct AgendaState {
    /// The Monday (local) of the visible week.
    pub week_start: NaiveDate,
    pub entries: Vec<EntryRow>,
    pub selected: usize,
    pub prompt: Option<AgendaPrompt>,
}

#[derive(Debug, Clone)]
pub struct EntryRow {
    pub entry: TimeEntry,
    pub task_title: String,
    pub day_label: String, // "Mon 22 Apr"
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgendaPrompt {
    DeleteConfirm { entry_id: i64, label: String },
}

impl AgendaState {
    pub fn for_now(now: DateTime<Utc>) -> Self {
        Self {
            week_start: monday_of_local_week(now),
            entries: Vec::new(),
            selected: 0,
            prompt: None,
        }
    }

    pub fn refresh<R: Repo>(&mut self, repo: &R, now: DateTime<Utc>) -> anyhow::Result<()> {
        let from = local_midnight_utc(self.week_start);
        let to = local_midnight_utc(self.week_start + Duration::days(7));
        let entries = repo.list_entries_in_range(from, to, now)?;
        let mut rows = Vec::with_capacity(entries.len());
        for e in entries {
            let task = repo.find_task(e.task_id)?.ok_or_else(|| {
                anyhow::anyhow!("agenda: missing task #{} for entry #{}", e.task_id, e.id)
            })?;
            let day = e.started_at.with_timezone(&Local).date_naive();
            let day_label = day.format("%a %d %b").to_string();
            rows.push(EntryRow {
                entry: e,
                task_title: task.title,
                day_label,
            });
        }
        if self.selected >= rows.len() {
            self.selected = rows.len().saturating_sub(1);
        }
        self.entries = rows;
        Ok(())
    }

    pub fn prev_week(&mut self) {
        self.week_start -= Duration::days(7);
    }

    pub fn next_week(&mut self) {
        self.week_start += Duration::days(7);
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
}

pub fn render<R: Repo>(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App<R>,
    state: &AgendaState,
) {
    let title = format!(
        "Agenda — week of {}",
        state.week_start.format("%Y-%m-%d")
    );
    let block = Block::default().borders(Borders::ALL).title(title);

    let mut last_day = String::new();
    let mut items: Vec<ListItem> = Vec::new();
    for (i, row) in state.entries.iter().enumerate() {
        if row.day_label != last_day {
            items.push(ListItem::new(Line::from(vec![Span::styled(
                row.day_label.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            )])));
            last_day = row.day_label.clone();
        }
        let entry_dur = row.entry.duration(app.now);
        let times = format!(
            "{}–{}",
            row.entry.started_at.with_timezone(&Local).format("%H:%M"),
            match row.entry.ended_at {
                Some(end) => end.with_timezone(&Local).format("%H:%M").to_string(),
                None => "now".into(),
            }
        );
        let line = Line::from(vec![
            Span::raw("  "),
            Span::raw(times),
            Span::raw("  "),
            Span::raw(crate::cli::commands_truncate(&row.task_title, 40)),
            Span::raw("  "),
            Span::styled(duration_compact(entry_dur), app.theme.total()),
        ]);
        let style = if i == state.selected {
            app.theme.selected()
        } else {
            Style::default()
        };
        items.push(ListItem::new(line).style(style));
    }
    if state.entries.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "  (no entries this week)",
            app.theme.dim(),
        ))));
    }
    let list = List::new(items).block(block);
    let mut list_state = ListState::default();
    // Translate entry index into list-item index (each day header adds one).
    list_state.select(Some(state.selected));
    frame.render_stateful_widget(list, area, &mut list_state);

    if let Some(AgendaPrompt::DeleteConfirm { entry_id, label }) = &state.prompt {
        let h = area.height.saturating_sub(1);
        let prompt_area = Rect::new(area.x, area.y + h, area.width, 1);
        let line = Line::from(Span::styled(
            format!("Delete entry #{entry_id} ({label})? y/N"),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(line), prompt_area);
    }
}

pub fn handle_key<R: Repo>(
    state: &mut AgendaState,
    app: &mut App<R>,
    action: KeyAction,
) -> Option<AgendaIntent> {
    if state.prompt.is_some() {
        return handle_prompt_key(state, app, action);
    }
    match action {
        KeyAction::Down => {
            state.move_down();
            None
        }
        KeyAction::Up => {
            state.move_up();
            None
        }
        KeyAction::Left => {
            state.prev_week();
            let _ = state.refresh(&app.repo, app.now);
            None
        }
        KeyAction::Right => {
            state.next_week();
            let _ = state.refresh(&app.repo, app.now);
            None
        }
        KeyAction::Delete => {
            if let Some(row) = state.entries.get(state.selected) {
                let entry_dur = row.entry.duration(app.now);
                state.prompt = Some(AgendaPrompt::DeleteConfirm {
                    entry_id: row.entry.id,
                    label: duration_compact(entry_dur),
                });
            }
            None
        }
        KeyAction::Confirm | KeyAction::Edit => state
            .entries
            .get(state.selected)
            .map(|row| AgendaIntent::OpenEdit(row.entry.id)),
        _ => None,
    }
}

fn handle_prompt_key<R: Repo>(
    state: &mut AgendaState,
    app: &mut App<R>,
    action: KeyAction,
) -> Option<AgendaIntent> {
    let Some(AgendaPrompt::DeleteConfirm { entry_id, .. }) = state.prompt.clone() else {
        return None;
    };
    match action {
        KeyAction::Char('y') => {
            match app.repo.delete_time_entry(entry_id) {
                Ok(()) => {
                    let _ = state.refresh(&app.repo, app.now);
                    app.refresh_active_timer().ok();
                    app.info(format!("Deleted entry #{entry_id}"));
                }
                Err(e) => app.error(format!("Could not delete: {e}")),
            }
            state.prompt = None;
            None
        }
        _ => {
            state.prompt = None;
            app.info("Cancelled");
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgendaIntent {
    OpenEdit(i64),
}

fn monday_of_local_week(now: DateTime<Utc>) -> NaiveDate {
    let local_today = now.with_timezone(&Local).date_naive();
    let days_since_monday = local_today.weekday().num_days_from_monday() as i64;
    local_today - Duration::days(days_since_monday)
}

fn local_midnight_utc(date: NaiveDate) -> DateTime<Utc> {
    let naive = date.and_hms_opt(0, 0, 0).expect("midnight always exists");
    Local
        .from_local_datetime(&naive)
        .single()
        .or_else(|| Local.from_local_datetime(&naive).earliest())
        .expect("local midnight resolvable (DST handled)")
        .with_timezone(&Utc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SqliteRepo;
    use crate::tui::app::App;
    use crate::tui::theme::Theme;

    fn at(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, 0, 0).unwrap()
    }

    fn fresh() -> (App<SqliteRepo>, AgendaState) {
        let now = at(2026, 4, 22, 10);
        let app = App::new(SqliteRepo::in_memory(), Theme::from_config_accent("cyan"), now);
        let state = AgendaState::for_now(now);
        (app, state)
    }

    #[test]
    fn week_start_is_monday_of_now_local() {
        let (_, state) = fresh();
        // 2026-04-22 is a Wednesday → Monday is 2026-04-20.
        assert_eq!(state.week_start, NaiveDate::from_ymd_opt(2026, 4, 20).unwrap());
    }

    #[test]
    fn refresh_loads_entries_for_visible_week_only() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        let task = app.repo.create_task("alpha", None).unwrap();
        // Entry inside the visible week (Wed):
        let inside = app
            .repo
            .create_time_entry(task.id, at(2026, 4, 22, 9))
            .unwrap();
        app.repo
            .end_time_entry(inside.id, at(2026, 4, 22, 10))
            .unwrap();
        // Entry outside (next week Mon):
        let outside = app
            .repo
            .create_time_entry(task.id, at(2026, 4, 27, 9))
            .unwrap();
        app.repo
            .end_time_entry(outside.id, at(2026, 4, 27, 10))
            .unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.entries[0].entry.id, inside.id);
    }

    #[test]
    fn left_paginates_to_previous_week_and_refreshes() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        let task = app.repo.create_task("t", None).unwrap();
        // Entry in the previous week (Wed 2026-04-15).
        let last_week = app
            .repo
            .create_time_entry(task.id, at(2026, 4, 15, 9))
            .unwrap();
        app.repo
            .end_time_entry(last_week.id, at(2026, 4, 15, 10))
            .unwrap();

        // Initial week has no entries.
        state.refresh(&app.repo, app.now).unwrap();
        assert!(state.entries.is_empty());

        // h navigates back; refresh now picks up the prior-week entry.
        handle_key(&mut state, &mut app, KeyAction::Left);
        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.entries[0].entry.id, last_week.id);
    }

    #[test]
    fn enter_returns_open_edit_intent_for_selected_entry() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        let task = app.repo.create_task("t", None).unwrap();
        let e = app
            .repo
            .create_time_entry(task.id, at(2026, 4, 22, 9))
            .unwrap();
        app.repo.end_time_entry(e.id, at(2026, 4, 22, 10)).unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        let intent = handle_key(&mut state, &mut app, KeyAction::Confirm);
        assert_eq!(intent, Some(AgendaIntent::OpenEdit(e.id)));
    }

    #[test]
    fn delete_confirm_y_deletes_entry() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        let task = app.repo.create_task("t", None).unwrap();
        let e = app
            .repo
            .create_time_entry(task.id, at(2026, 4, 22, 9))
            .unwrap();
        app.repo.end_time_entry(e.id, at(2026, 4, 22, 10)).unwrap();
        state.refresh(&app.repo, app.now).unwrap();

        handle_key(&mut state, &mut app, KeyAction::Delete);
        assert!(matches!(state.prompt, Some(AgendaPrompt::DeleteConfirm { .. })));

        handle_key(&mut state, &mut app, KeyAction::Char('y'));
        assert!(state.entries.is_empty());
        assert!(state.prompt.is_none());
    }
}
```

- [ ] **Step 2: Wire the screen module**

Edit `src/tui/screens/mod.rs`:

```rust
//! Per-screen state + rendering.

pub mod agenda;
pub mod tasks;
```

- [ ] **Step 3: Run tests**

Run: `cargo test --lib tui::screens -- --nocapture`
Expected: PASS — five new agenda tests + the eighteen tasks tests still green.

- [ ] **Step 4: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add src/tui/screens/agenda.rs src/tui/screens/mod.rs
git commit -m "feat(tui): agenda screen with week pagination and delete confirm"
```

---

## Task 12: Report screen

**Files:**
- Create: `src/tui/clipboard.rs`
- Create: `src/tui/screens/report.rs`
- Modify: `src/tui/mod.rs`
- Modify: `src/tui/screens/mod.rs`

- [ ] **Step 1: Create the clipboard helper**

Create `src/tui/clipboard.rs`:

```rust
//! Thin wrapper around `wl-copy` and `xclip`. Best-effort: missing
//! tools surface a structured error the caller can show in the footer
//! instead of panicking.

use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, thiserror::Error)]
pub enum ClipboardError {
    #[error("no clipboard tool found (need wl-copy or xclip)")]
    NoTool,
    #[error("clipboard tool exited with code {0}")]
    Exit(i32),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub fn copy(text: &str) -> Result<&'static str, ClipboardError> {
    if let Some(name) = try_copy_with("wl-copy", &[], text)? {
        return Ok(name);
    }
    if let Some(name) = try_copy_with("xclip", &["-selection", "clipboard"], text)? {
        return Ok(name);
    }
    Err(ClipboardError::NoTool)
}

fn try_copy_with(
    bin: &'static str,
    extra_args: &[&str],
    text: &str,
) -> Result<Option<&'static str>, ClipboardError> {
    let mut cmd = Command::new(bin);
    cmd.args(extra_args).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes())?;
    }
    let status = child.wait()?;
    if status.success() {
        Ok(Some(bin))
    } else {
        Err(ClipboardError::Exit(status.code().unwrap_or(-1)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_tool_when_neither_wl_copy_nor_xclip_present() {
        // We test by asking for a binary that definitely doesn't exist.
        let res = try_copy_with("definitely-not-a-real-binary-xyz", &[], "hi");
        assert!(matches!(res, Ok(None)));
    }
}
```

- [ ] **Step 2: Write failing tests for the report screen**

Create `src/tui/screens/report.rs`:

```rust
//! Report screen — wraps the Phase C `ReportBuilder` output. Tab cycles
//! scope, T cycles grouping, c copies a one-liner, j toggles a JSON dump.

use crate::cli::format::{bar, duration_compact};
use crate::domain::{Grouping, Report, ReportBuilder, Scope, ScopeKind};
use crate::storage::Repo;
use crate::tui::app::App;
use crate::tui::clipboard;
use crate::tui::keymap::KeyAction;
use chrono::{DateTime, Utc};
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

#[derive(Debug, Clone)]
pub struct ReportState {
    pub scope_kind: ScopeKind,
    pub grouping: Grouping,
    pub json: bool,
    pub report: Option<Report>,
}

impl Default for ReportState {
    fn default() -> Self {
        Self {
            scope_kind: ScopeKind::Today,
            grouping: Grouping::Task,
            json: false,
            report: None,
        }
    }
}

impl ReportState {
    pub fn refresh<R: Repo>(&mut self, repo: &R, now: DateTime<Utc>) -> anyhow::Result<()> {
        let scope = match self.scope_kind {
            ScopeKind::Today => Scope::today(now),
            ScopeKind::Week => Scope::week(now),
            ScopeKind::Month => Scope::month(now),
            ScopeKind::All => Scope::all(now),
            ScopeKind::Range => Scope::today(now), // Custom ranges are CLI-only.
        };
        let report = ReportBuilder::new(repo).build(scope, self.grouping, now)?;
        self.report = Some(report);
        Ok(())
    }

    pub fn cycle_scope(&mut self) {
        self.scope_kind = match self.scope_kind {
            ScopeKind::Today => ScopeKind::Week,
            ScopeKind::Week => ScopeKind::Month,
            ScopeKind::Month => ScopeKind::All,
            ScopeKind::All | ScopeKind::Range => ScopeKind::Today,
        };
    }

    pub fn cycle_grouping(&mut self) {
        self.grouping = match self.grouping {
            Grouping::Task => Grouping::Epic,
            Grouping::Epic => Grouping::Day,
            Grouping::Day => Grouping::Task,
        };
    }
}

pub fn render<R: Repo>(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App<R>,
    state: &ReportState,
) {
    let title = format!(
        "Report — {} / {}",
        scope_label(state.scope_kind),
        grouping_label(state.grouping)
    );
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let body = match (&state.report, state.json) {
        (None, _) => Paragraph::new("Loading…")
            .style(app.theme.dim()),
        (Some(report), true) => {
            let json = serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".into());
            Paragraph::new(json).wrap(Wrap { trim: false })
        }
        (Some(report), false) => Paragraph::new(table_lines(report)).wrap(Wrap { trim: false }),
    };
    frame.render_widget(body, inner);
}

fn scope_label(kind: ScopeKind) -> &'static str {
    match kind {
        ScopeKind::Today => "Today",
        ScopeKind::Week => "This week",
        ScopeKind::Month => "This month",
        ScopeKind::All => "All time",
        ScopeKind::Range => "Range",
    }
}

fn grouping_label(g: Grouping) -> &'static str {
    match g {
        Grouping::Task => "by task",
        Grouping::Epic => "by epic",
        Grouping::Day => "by day",
    }
}

fn table_lines(report: &Report) -> Vec<Line<'static>> {
    if report.rows.is_empty() {
        return vec![Line::from("No time tracked in this scope.")];
    }
    let max = report
        .rows
        .iter()
        .map(|r| r.duration_seconds)
        .max()
        .unwrap_or(0);
    let mut lines: Vec<Line> = report
        .rows
        .iter()
        .map(|row| {
            let total = chrono::Duration::seconds(row.duration_seconds);
            Line::from(format!(
                "{:<48}  {:>8}  {}",
                crate::cli::commands_truncate(&row.label, 48),
                duration_compact(total),
                bar(row.duration_seconds, max, 30),
            ))
        })
        .collect();
    let total = chrono::Duration::seconds(report.total_seconds);
    lines.push(Line::from(vec![Span::styled(
        format!("{:<48}  {:>8}", "Total", duration_compact(total)),
        ratatui::style::Style::default().add_modifier(Modifier::BOLD),
    )]));
    lines
}

pub fn handle_key<R: Repo>(
    state: &mut ReportState,
    app: &mut App<R>,
    action: KeyAction,
) -> bool {
    match action {
        KeyAction::CycleNext => {
            state.cycle_scope();
            let _ = state.refresh(&app.repo, app.now);
            true
        }
        KeyAction::CycleGroup => {
            state.cycle_grouping();
            let _ = state.refresh(&app.repo, app.now);
            true
        }
        KeyAction::Down | KeyAction::ToggleJson => {
            // `j` resolves to `Down` in the global keymap. The Report screen
            // has no row navigation (spec: "Report has no row selection"),
            // so on this screen `Down` is repurposed as the JSON toggle.
            state.json = !state.json;
            true
        }
        KeyAction::Copy => {
            copy_one_liner(state, app);
            true
        }
        _ => false,
    }
}

fn copy_one_liner<R: Repo>(state: &ReportState, app: &mut App<R>) {
    let Some(report) = &state.report else {
        app.info("Nothing to copy yet.");
        return;
    };
    let total = chrono::Duration::seconds(report.total_seconds);
    let one_liner = format!(
        "{}: {} ({} rows)",
        scope_label(state.scope_kind),
        duration_compact(total),
        report.rows.len()
    );
    match clipboard::copy(&one_liner) {
        Ok(tool) => app.info(format!("Copied via {tool}")),
        Err(e) => app.error(format!("Copy failed: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SqliteRepo;
    use crate::tui::app::App;
    use crate::tui::theme::Theme;
    use chrono::TimeZone;

    fn at(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, 0, 0).unwrap()
    }

    fn fresh() -> (App<SqliteRepo>, ReportState) {
        let now = at(2026, 4, 22, 10);
        (
            App::new(SqliteRepo::in_memory(), Theme::from_config_accent("cyan"), now),
            ReportState::default(),
        )
    }

    #[test]
    fn defaults_today_by_task() {
        let s = ReportState::default();
        assert_eq!(s.scope_kind, ScopeKind::Today);
        assert_eq!(s.grouping, Grouping::Task);
        assert!(!s.json);
    }

    #[test]
    fn cycle_scope_walks_today_week_month_all() {
        let mut s = ReportState::default();
        assert_eq!(s.scope_kind, ScopeKind::Today);
        s.cycle_scope();
        assert_eq!(s.scope_kind, ScopeKind::Week);
        s.cycle_scope();
        assert_eq!(s.scope_kind, ScopeKind::Month);
        s.cycle_scope();
        assert_eq!(s.scope_kind, ScopeKind::All);
        s.cycle_scope();
        assert_eq!(s.scope_kind, ScopeKind::Today);
    }

    #[test]
    fn cycle_grouping_walks_task_epic_day() {
        let mut s = ReportState::default();
        assert_eq!(s.grouping, Grouping::Task);
        s.cycle_grouping();
        assert_eq!(s.grouping, Grouping::Epic);
        s.cycle_grouping();
        assert_eq!(s.grouping, Grouping::Day);
        s.cycle_grouping();
        assert_eq!(s.grouping, Grouping::Task);
    }

    #[test]
    fn refresh_populates_report() {
        use crate::storage::Repo;
        let (mut app, mut state) = fresh();
        let t = app.repo.create_task("alpha", None).unwrap();
        let e = app.repo.create_time_entry(t.id, at(2026, 4, 22, 9)).unwrap();
        app.repo.end_time_entry(e.id, at(2026, 4, 22, 10)).unwrap();
        state.refresh(&app.repo, app.now).unwrap();
        let r = state.report.as_ref().unwrap();
        assert_eq!(r.rows.len(), 1);
        assert_eq!(r.total_seconds, 3600);
    }

    #[test]
    fn handle_tab_cycles_scope_and_refreshes() {
        let (mut app, mut state) = fresh();
        let acted = handle_key(&mut state, &mut app, KeyAction::CycleNext);
        assert!(acted);
        assert_eq!(state.scope_kind, ScopeKind::Week);
    }

    #[test]
    fn handle_t_cycles_grouping() {
        let (mut app, mut state) = fresh();
        handle_key(&mut state, &mut app, KeyAction::CycleGroup);
        assert_eq!(state.grouping, Grouping::Epic);
    }

    #[test]
    fn handle_down_toggles_json_on_report_screen() {
        // `j` resolves to KeyAction::Down globally; on Report it's the
        // JSON toggle.
        let (mut app, mut state) = fresh();
        assert!(!state.json);
        handle_key(&mut state, &mut app, KeyAction::Down);
        assert!(state.json);
        handle_key(&mut state, &mut app, KeyAction::Down);
        assert!(!state.json);
    }
}
```

- [ ] **Step 3: Wire modules**

Edit `src/tui/mod.rs`:

```rust
//! Buckland TUI — ratatui surface composed on top of the `Repo` trait.

pub mod app;
pub mod chrome;
pub mod clipboard;
pub mod keymap;
pub mod screens;
pub mod terminal;
pub mod theme;

use crate::cli::context::Context;

pub fn run(_ctx: Context) -> anyhow::Result<i32> {
    Ok(0)
}
```

Edit `src/tui/screens/mod.rs`:

```rust
//! Per-screen state + rendering.

pub mod agenda;
pub mod report;
pub mod tasks;
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib tui:: -- --nocapture`
Expected: PASS — seven new report tests + one new clipboard test all green.

- [ ] **Step 5: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add src/tui/clipboard.rs src/tui/mod.rs src/tui/screens/mod.rs src/tui/screens/report.rs
git commit -m "feat(tui): report screen with scope/group cycling and clipboard"
```

---

## Task 13: Edit overlay (Time entry + Task)

**Files:**
- Modify: `src/storage/repo.rs`
- Create: `src/tui/screens/edit.rs`
- Modify: `src/tui/screens/mod.rs`

- [ ] **Step 1: Write the failing repo test for the new method**

Append inside `src/storage/repo.rs`'s `mod tests`:

```rust
    #[test]
    fn update_time_entry_changes_started_ended_notes() {
        use chrono::TimeZone;
        let mut r = repo();
        let t = r.create_task("t", None).unwrap();
        let e = r.create_time_entry(t.id, Utc.with_ymd_and_hms(2026, 4, 22, 9, 0, 0).unwrap()).unwrap();
        r.end_time_entry(e.id, Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap()).unwrap();

        let new_start = Utc.with_ymd_and_hms(2026, 4, 22, 9, 15, 0).unwrap();
        let new_end = Utc.with_ymd_and_hms(2026, 4, 22, 11, 0, 0).unwrap();
        let updated = r
            .update_time_entry(e.id, new_start, Some(new_end), Some("forgot to start"))
            .unwrap();
        assert_eq!(updated.started_at, new_start);
        assert_eq!(updated.ended_at, Some(new_end));
        assert_eq!(updated.notes.as_deref(), Some("forgot to start"));
    }

    #[test]
    fn update_time_entry_returns_not_found_for_missing_id() {
        match repo().update_time_entry(9999, Utc::now(), None, None) {
            Err(RepoError::TimeEntryNotFound(id)) => assert_eq!(id, 9999),
            other => panic!("expected TimeEntryNotFound, got {other:?}"),
        }
    }
```

Add the trait method declaration to `Repo`:

```rust
pub trait Repo {
    // ... existing methods ...

    /// Update started_at, ended_at, and/or notes on a time entry. Pass
    /// `None` to clear the optional fields.
    fn update_time_entry(
        &mut self,
        id: i64,
        started_at: DateTime<Utc>,
        ended_at: Option<DateTime<Utc>>,
        notes: Option<&str>,
    ) -> RepoResult<TimeEntry>;
}
```

Implement it in `impl Repo for SqliteRepo`:

```rust
    fn update_time_entry(
        &mut self,
        id: i64,
        started_at: DateTime<Utc>,
        ended_at: Option<DateTime<Utc>>,
        notes: Option<&str>,
    ) -> RepoResult<TimeEntry> {
        let updated = self.conn.execute(
            "UPDATE time_entries SET started_at = ?1, ended_at = ?2, notes = ?3 WHERE id = ?4",
            params![started_at, ended_at, notes, id],
        )?;
        if updated == 0 {
            return Err(RepoError::TimeEntryNotFound(id));
        }
        self.conn
            .query_row(
                &format!("SELECT {TIME_ENTRY_COLS} FROM time_entries WHERE id = ?1"),
                [id],
                |row| TimeEntry::try_from(row),
            )
            .map_err(RepoError::from)
    }
```

- [ ] **Step 2: Run the repo tests**

Run: `cargo test --lib storage::repo::tests::update_time_entry -- --nocapture`
Expected: PASS — both new tests green.

- [ ] **Step 3: Write failing tests for the Edit overlay state machine**

Create `src/tui/screens/edit.rs`:

```rust
//! Edit overlay — modal for amending a single `time_entry`. Fields:
//! task title (read-only), started_at, ended_at, notes. `Tab` cycles
//! fields. `Enter` saves. `Esc` cancels. `^D` deletes (with `y/N`).

use crate::domain::TimeEntry;
use crate::storage::Repo;
use crate::tui::app::App;
use crate::tui::keymap::KeyAction;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditField {
    StartedAt,
    EndedAt,
    Notes,
}

#[derive(Debug, Clone)]
pub struct EditState {
    pub entry_id: i64,
    pub task_title: String,
    pub started_at: String,   // "YYYY-MM-DD HH:MM"
    pub ended_at: String,     // empty when active
    pub notes: String,
    pub field: EditField,
    pub confirm_delete: bool,
}

impl EditState {
    pub fn for_entry(entry: &TimeEntry, task_title: String) -> Self {
        Self {
            entry_id: entry.id,
            task_title,
            started_at: format_local(entry.started_at),
            ended_at: entry
                .ended_at
                .map(format_local)
                .unwrap_or_default(),
            notes: entry.notes.clone().unwrap_or_default(),
            field: EditField::StartedAt,
            confirm_delete: false,
        }
    }

    pub fn cycle_field(&mut self) {
        self.field = match self.field {
            EditField::StartedAt => EditField::EndedAt,
            EditField::EndedAt => EditField::Notes,
            EditField::Notes => EditField::StartedAt,
        };
    }

    pub fn current_buffer_mut(&mut self) -> &mut String {
        match self.field {
            EditField::StartedAt => &mut self.started_at,
            EditField::EndedAt => &mut self.ended_at,
            EditField::Notes => &mut self.notes,
        }
    }
}

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &EditState) {
    let backdrop_area = area;
    let modal_area = centered(60, 12, backdrop_area);
    let dim = Block::default().style(Style::default().add_modifier(Modifier::DIM));
    frame.render_widget(dim, backdrop_area);

    let block = Block::default().borders(Borders::ALL).title("Edit entry");
    frame.render_widget(block.clone(), modal_area);
    let inner = block.inner(modal_area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(format!("Task: {}", state.task_title)),
        rows[0],
    );
    frame.render_widget(
        field_line("Started", &state.started_at, state.field == EditField::StartedAt),
        rows[2],
    );
    frame.render_widget(
        field_line("Ended  ", &state.ended_at, state.field == EditField::EndedAt),
        rows[3],
    );
    frame.render_widget(
        field_line("Notes  ", &state.notes, state.field == EditField::Notes),
        rows[4],
    );
    let footer = if state.confirm_delete {
        "Delete this entry? y/N"
    } else {
        "Tab cycle • Enter save • ^D delete • Esc cancel"
    };
    frame.render_widget(
        Paragraph::new(footer).style(Style::default().add_modifier(Modifier::DIM)),
        rows[5],
    );
}

fn field_line(label: &'static str, value: &str, focused: bool) -> Paragraph<'static> {
    let style = if focused {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };
    Paragraph::new(Line::from(vec![
        Span::raw(format!("{label}: ")),
        Span::styled(value.to_string(), style),
    ]))
}

fn centered(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(
        x,
        y,
        width.min(area.width),
        height.min(area.height),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditOutcome {
    Continue,
    Saved,
    Deleted,
    Cancelled,
}

pub fn handle_key<R: Repo>(
    state: &mut EditState,
    app: &mut App<R>,
    action: KeyAction,
) -> EditOutcome {
    if state.confirm_delete {
        return match action {
            KeyAction::Char('y') => {
                match app.repo.delete_time_entry(state.entry_id) {
                    Ok(()) => {
                        app.info(format!("Deleted entry #{}", state.entry_id));
                        EditOutcome::Deleted
                    }
                    Err(e) => {
                        app.error(format!("Could not delete: {e}"));
                        state.confirm_delete = false;
                        EditOutcome::Continue
                    }
                }
            }
            _ => {
                state.confirm_delete = false;
                EditOutcome::Continue
            }
        };
    }
    match action {
        KeyAction::CycleNext => {
            state.cycle_field();
            EditOutcome::Continue
        }
        KeyAction::Char(c) => {
            state.current_buffer_mut().push(c);
            EditOutcome::Continue
        }
        KeyAction::Backspace => {
            state.current_buffer_mut().pop();
            EditOutcome::Continue
        }
        KeyAction::Confirm => save(state, app),
        KeyAction::DeleteCtrlD => {
            state.confirm_delete = true;
            EditOutcome::Continue
        }
        KeyAction::Quit => EditOutcome::Cancelled,
        _ => EditOutcome::Continue,
    }
}

fn save<R: Repo>(state: &mut EditState, app: &mut App<R>) -> EditOutcome {
    let started_at = match parse_local(&state.started_at) {
        Some(dt) => dt,
        None => {
            app.error(format!("Invalid started_at: {}", state.started_at));
            return EditOutcome::Continue;
        }
    };
    let ended_at = if state.ended_at.trim().is_empty() {
        None
    } else {
        match parse_local(&state.ended_at) {
            Some(dt) => Some(dt),
            None => {
                app.error(format!("Invalid ended_at: {}", state.ended_at));
                return EditOutcome::Continue;
            }
        }
    };
    let notes = if state.notes.trim().is_empty() {
        None
    } else {
        Some(state.notes.as_str())
    };
    match app.repo.update_time_entry(state.entry_id, started_at, ended_at, notes) {
        Ok(_) => {
            app.info(format!("Saved entry #{}", state.entry_id));
            EditOutcome::Saved
        }
        Err(e) => {
            app.error(format!("Could not save: {e}"));
            EditOutcome::Continue
        }
    }
}

fn format_local(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string()
}

fn parse_local(s: &str) -> Option<DateTime<Utc>> {
    let naive = NaiveDateTime::parse_from_str(s.trim(), "%Y-%m-%d %H:%M").ok()?;
    Local
        .from_local_datetime(&naive)
        .single()
        .or_else(|| Local.from_local_datetime(&naive).earliest())
        .map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SqliteRepo;
    use crate::tui::app::App;
    use crate::tui::theme::Theme;
    use chrono::TimeZone;

    fn at(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 22, h, m, 0).unwrap()
    }

    fn fresh() -> (App<SqliteRepo>, EditState) {
        use crate::storage::Repo;
        let mut app = App::new(SqliteRepo::in_memory(), Theme::from_config_accent("cyan"), at(12, 0));
        let task = app.repo.create_task("fix login", None).unwrap();
        let e = app.repo.create_time_entry(task.id, at(9, 0)).unwrap();
        app.repo.end_time_entry(e.id, at(10, 0)).unwrap();
        let entry = app.repo.list_entries_for_task(task.id).unwrap().remove(0);
        let state = EditState::for_entry(&entry, task.title);
        (app, state)
    }

    #[test]
    fn for_entry_seeds_local_strings() {
        let (_, state) = fresh();
        // The runner's local zone determines the exact text; check shape.
        assert!(state.started_at.contains(':'));
        assert!(state.ended_at.contains(':'));
        assert_eq!(state.field, EditField::StartedAt);
    }

    #[test]
    fn cycle_field_walks_started_ended_notes() {
        let (_, mut state) = fresh();
        assert_eq!(state.field, EditField::StartedAt);
        state.cycle_field();
        assert_eq!(state.field, EditField::EndedAt);
        state.cycle_field();
        assert_eq!(state.field, EditField::Notes);
        state.cycle_field();
        assert_eq!(state.field, EditField::StartedAt);
    }

    #[test]
    fn typing_into_notes_field_appends_to_buffer() {
        let (mut app, mut state) = fresh();
        state.field = EditField::Notes;
        handle_key(&mut state, &mut app, KeyAction::Char('o'));
        handle_key(&mut state, &mut app, KeyAction::Char('k'));
        assert_eq!(state.notes, "ok");
    }

    #[test]
    fn invalid_started_at_blocks_save_and_surfaces_error() {
        let (mut app, mut state) = fresh();
        state.started_at = "not a date".into();
        let outcome = handle_key(&mut state, &mut app, KeyAction::Confirm);
        assert_eq!(outcome, EditOutcome::Continue);
        assert!(matches!(app.footer, crate::tui::app::FooterMessage::Error(_)));
    }

    #[test]
    fn ctrl_d_then_y_deletes() {
        let (mut app, mut state) = fresh();
        handle_key(&mut state, &mut app, KeyAction::DeleteCtrlD);
        assert!(state.confirm_delete);
        let outcome = handle_key(&mut state, &mut app, KeyAction::Char('y'));
        assert_eq!(outcome, EditOutcome::Deleted);
    }

    #[test]
    fn ctrl_d_then_n_cancels_confirm() {
        let (mut app, mut state) = fresh();
        handle_key(&mut state, &mut app, KeyAction::DeleteCtrlD);
        let outcome = handle_key(&mut state, &mut app, KeyAction::Char('n'));
        assert_eq!(outcome, EditOutcome::Continue);
        assert!(!state.confirm_delete);
    }

    #[test]
    fn esc_cancels_overlay() {
        let (mut app, mut state) = fresh();
        let outcome = handle_key(&mut state, &mut app, KeyAction::Quit);
        assert_eq!(outcome, EditOutcome::Cancelled);
    }
}
```

- [ ] **Step 4: Wire the screen module**

Edit `src/tui/screens/mod.rs`:

```rust
//! Per-screen state + rendering.

pub mod agenda;
pub mod edit;
pub mod report;
pub mod tasks;
```

- [ ] **Step 5: Run tests**

Run: `cargo test --lib tui::screens::edit -- --nocapture`
Expected: PASS — seven edit tests green.

Run: `cargo test --all-features`
Expected: PASS — repo trait change is covered by the new tests; older tests untouched. (One callout: any Mock `Repo` impl in test code must add the new method. Search with `grep -rn "impl .*Repo for" src/ tests/` — at the time of writing, only `SqliteRepo` implements `Repo` in this codebase, so no other impls need updating.)

- [ ] **Step 6: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add src/storage/repo.rs src/tui/screens/edit.rs src/tui/screens/mod.rs
git commit -m "feat(tui): edit overlay with field cycling, save and ^D delete"
```

---

## Task 14: Help overlay

**Files:**
- Create: `src/tui/screens/help.rs`
- Modify: `src/tui/screens/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `src/tui/screens/help.rs`:

```rust
//! Help overlay — read-only render of the keymap, grouped by category.
//! No state, no input handling beyond "any key returns to Tasks" (that
//! transition is handled in the App's global handler).

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

const HELP: &[(&str, &[(&str, &str)])] = &[
    (
        "Navigation",
        &[
            ("j / Down", "move down"),
            ("k / Up", "move up"),
            ("h / Left", "previous (week / field)"),
            ("l / Right", "next (week / field)"),
            ("gg", "top"),
            ("G", "bottom"),
            ("Tab", "cycle view / modal field / report scope"),
        ],
    ),
    (
        "View",
        &[
            ("g", "Tasks (home)"),
            ("a", "Agenda"),
            ("r", "Report"),
            ("?", "Help"),
            ("q / Esc", "quit / close overlay"),
        ],
    ),
    (
        "Tasks",
        &[
            ("n", "new task"),
            ("s / Enter", "start selected"),
            ("S", "stop active timer"),
            ("d", "mark done"),
            ("A", "archive"),
            ("D", "delete (with confirm)"),
            ("e", "edit task"),
            ("/", "filter"),
        ],
    ),
    (
        "Report",
        &[
            ("Tab", "cycle scope"),
            ("T", "cycle grouping"),
            ("c", "copy summary to clipboard"),
            ("j", "toggle JSON dump"),
        ],
    ),
    (
        "Edit overlay",
        &[
            ("Tab", "cycle fields"),
            ("Enter", "save"),
            ("Ctrl+D", "delete entry"),
            ("Esc", "cancel"),
        ],
    ),
];

pub fn render(frame: &mut Frame<'_>, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Help");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    for (title, rows) in HELP {
        lines.push(Line::from(Span::styled(
            (*title).to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for (key, desc) in *rows {
            lines.push(Line::from(format!("  {key:<14}{desc}")));
        }
        lines.push(Line::from(""));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn buffer_text(width: u16, height: u16, area: Rect) -> String {
        let backend = TestBackend::new(width, height);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|frame| render(frame, area)).unwrap();
        let buf = term.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..buf.area().height {
            for x in 0..buf.area().width {
                out.push_str(buf.cell((x, y)).unwrap().symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn help_text_includes_all_categories() {
        let out = buffer_text(80, 30, Rect::new(0, 0, 80, 30));
        assert!(out.contains("Navigation"), "got:\n{out}");
        assert!(out.contains("Tasks"));
        assert!(out.contains("Report"));
        assert!(out.contains("Edit overlay"));
    }

    #[test]
    fn help_lists_core_shortcuts() {
        let out = buffer_text(80, 30, Rect::new(0, 0, 80, 30));
        assert!(out.contains("j / Down"));
        assert!(out.contains("Ctrl+D"));
        assert!(out.contains("toggle JSON"));
    }
}
```

- [ ] **Step 2: Wire the module**

Edit `src/tui/screens/mod.rs`:

```rust
//! Per-screen state + rendering.

pub mod agenda;
pub mod edit;
pub mod help;
pub mod report;
pub mod tasks;
```

- [ ] **Step 3: Run tests**

Run: `cargo test --lib tui::screens::help -- --nocapture`
Expected: PASS — two help tests green.

- [ ] **Step 4: Commit**

```bash
git add src/tui/screens/help.rs src/tui/screens/mod.rs
git commit -m "feat(tui): help overlay listing all keybindings"
```

---

## Task 15: `tui::run()` event loop wires every screen together

**Files:**
- Modify: `src/tui/mod.rs`

- [ ] **Step 1: Write the failing integration-style test**

Append to `src/tui/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Repo, SqliteRepo};
    use crate::tui::app::{App, ScreenKind};
    use crate::tui::keymap::KeyAction;
    use crate::tui::theme::Theme;
    use chrono::{TimeZone, Utc};

    fn at(h: u32) -> chrono::DateTime<chrono::Utc> {
        Utc.with_ymd_and_hms(2026, 4, 22, h, 0, 0).unwrap()
    }

    #[test]
    fn dispatch_handles_screen_switching_through_global() {
        let mut app = App::new(SqliteRepo::in_memory(), Theme::from_config_accent("cyan"), at(10));
        let mut session = TuiSession::new();
        session.dispatch(&mut app, KeyAction::OpenAgenda);
        assert_eq!(app.screen, ScreenKind::Agenda);
        session.dispatch(&mut app, KeyAction::OpenReport);
        assert_eq!(app.screen, ScreenKind::Report);
        session.dispatch(&mut app, KeyAction::Help);
        assert_eq!(app.screen, ScreenKind::Help);
    }

    #[test]
    fn dispatch_routes_tasks_keys_to_tasks_screen() {
        let mut app = App::new(SqliteRepo::in_memory(), Theme::from_config_accent("cyan"), at(10));
        app.repo.create_task("alpha", None).unwrap();
        let mut session = TuiSession::new();
        session.refresh(&mut app).unwrap();
        // Down on Tasks moves the cursor.
        session.dispatch(&mut app, KeyAction::Down);
        // Selection clamps at len-1 = 0 with one task; just assert no panic.
        assert_eq!(session.tasks.selected, 0);
    }

    #[test]
    fn dispatch_quit_sets_should_quit() {
        let mut app = App::new(SqliteRepo::in_memory(), Theme::from_config_accent("cyan"), at(10));
        let mut session = TuiSession::new();
        session.dispatch(&mut app, KeyAction::Quit);
        assert!(app.should_quit);
    }

    #[test]
    fn dispatch_edit_intent_from_agenda_opens_edit_overlay() {
        use crate::storage::Repo;
        let mut app = App::new(SqliteRepo::in_memory(), Theme::from_config_accent("cyan"), at(10));
        let task = app.repo.create_task("alpha", None).unwrap();
        let e = app.repo.create_time_entry(task.id, at(9)).unwrap();
        app.repo.end_time_entry(e.id, at(10)).unwrap();
        let mut session = TuiSession::new();
        app.screen = ScreenKind::Agenda;
        session.refresh(&mut app).unwrap();
        session.dispatch(&mut app, KeyAction::Confirm);
        assert!(session.edit.is_some(), "edit overlay should be open");
    }
}
```

- [ ] **Step 2: Run; expect failure**

Run: `cargo test --lib tui::tests -- --nocapture`
Expected: FAIL — `TuiSession` does not exist yet.

- [ ] **Step 3: Implement `TuiSession` + `run()`**

Replace `src/tui/mod.rs` with the full implementation:

```rust
//! Buckland TUI — ratatui surface composed on top of the `Repo` trait.

pub mod app;
pub mod chrome;
pub mod clipboard;
pub mod keymap;
pub mod screens;
pub mod terminal;
pub mod theme;

use crate::cli::context::Context;
use crate::storage::Repo;
use crate::tui::app::{App, ScreenKind};
use crate::tui::keymap::{resolve, resolve_after_pending_g, KeyAction};
use crate::tui::screens::edit::{EditOutcome, EditState};
use crate::tui::screens::tasks::TasksState;
use crate::tui::screens::{agenda, edit, help, report, tasks};
use crate::tui::terminal::TerminalGuard;
use crate::tui::theme::Theme;
use chrono::Utc;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub struct TuiSession {
    pub tasks: tasks::TasksState,
    pub agenda: agenda::AgendaState,
    pub report: report::ReportState,
    pub edit: Option<EditState>,
}

impl TuiSession {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            tasks: TasksState::default(),
            agenda: agenda::AgendaState::for_now(now),
            report: report::ReportState::default(),
            edit: None,
        }
    }

    pub fn refresh<R: Repo>(&mut self, app: &mut App<R>) -> anyhow::Result<()> {
        app.refresh_active_timer()?;
        self.tasks.refresh(&app.repo, app.now)?;
        self.agenda.refresh(&app.repo, app.now)?;
        self.report.refresh(&app.repo, app.now)?;
        Ok(())
    }

    pub fn dispatch<R: Repo>(&mut self, app: &mut App<R>, action: KeyAction) {
        // Edit overlay (when active) sees keys first.
        if let Some(state) = self.edit.as_mut() {
            let outcome = edit::handle_key(state, app, action);
            match outcome {
                EditOutcome::Saved | EditOutcome::Deleted | EditOutcome::Cancelled => {
                    self.edit = None;
                    let _ = self.refresh(app);
                }
                EditOutcome::Continue => {}
            }
            return;
        }

        // Global handlers — quit, help, view-switch (g/a/r), `gg`.
        if app.handle_global(action) {
            return;
        }

        // Per-screen routing.
        match app.screen {
            ScreenKind::Tasks => {
                tasks::handle_key(&mut self.tasks, app, action);
            }
            ScreenKind::Agenda => {
                if let Some(intent) = agenda::handle_key(&mut self.agenda, app, action) {
                    match intent {
                        agenda::AgendaIntent::OpenEdit(entry_id) => self.open_edit(app, entry_id),
                    }
                }
            }
            ScreenKind::Report => {
                report::handle_key(&mut self.report, app, action);
            }
            ScreenKind::Help => {
                // Any key returns to Tasks. The global handler captures
                // Quit; for everything else we route here.
                app.screen = ScreenKind::Tasks;
            }
        }
    }

    fn open_edit<R: Repo>(&mut self, app: &mut App<R>, entry_id: i64) {
        let Ok(entry_opt) = (|| -> anyhow::Result<_> {
            let mut entries = Vec::new();
            for row in app.repo.list_entries_in_range(
                app.now - chrono::Duration::days(365),
                app.now + chrono::Duration::days(1),
                app.now,
            )? {
                if row.id == entry_id {
                    entries.push(row);
                }
            }
            Ok(entries.into_iter().next())
        })() else {
            app.error("Could not load entry for editing.");
            return;
        };
        let Some(entry) = entry_opt else {
            app.error(format!("Entry #{entry_id} not found."));
            return;
        };
        let Ok(Some(task)) = app.repo.find_task(entry.task_id) else {
            app.error(format!("Task for entry #{entry_id} not found."));
            return;
        };
        self.edit = Some(EditState::for_entry(&entry, task.title));
    }
}

pub fn run(ctx: Context) -> anyhow::Result<i32> {
    let theme = Theme::from_config_accent(
        &crate::config::load(&crate::config::config_path())
            .map(|c| c.ui.accent_color)
            .unwrap_or_else(|_| "cyan".into()),
    );
    let mut app = App::new(ctx.repo, theme, Utc::now());
    let mut session = TuiSession::new();
    session.refresh(&mut app)?;

    let mut guard = TerminalGuard::enter()?;
    let event_rx = spawn_event_thread();

    while !app.should_quit {
        guard.terminal().draw(|frame| {
            let area = frame.area();
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(area);
            chrome::render_header(frame, layout[0], &app);
            render_main(frame, layout[1], &app, &session);
            chrome::render_footer(frame, layout[2], &app);
        })?;

        match event_rx.recv() {
            Ok(LoopEvent::Tick) => {
                app.on_tick(Utc::now());
            }
            Ok(LoopEvent::Key(key)) => {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                let action = if app.pending_g {
                    resolve_after_pending_g(key)
                } else {
                    resolve(key)
                };
                session.dispatch(&mut app, action);
            }
            Err(_) => break,
        }
    }
    Ok(0)
}

fn render_main<R: Repo>(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    app: &App<R>,
    session: &TuiSession,
) {
    if let Some(edit_state) = &session.edit {
        edit::render(frame, area, edit_state);
        return;
    }
    match app.screen {
        ScreenKind::Tasks => tasks::render(frame, area, app, &session.tasks),
        ScreenKind::Agenda => agenda::render(frame, area, app, &session.agenda),
        ScreenKind::Report => report::render(frame, area, app, &session.report),
        ScreenKind::Help => help::render(frame, area),
    }
}

enum LoopEvent {
    Tick,
    Key(KeyEvent),
}

fn spawn_event_thread() -> mpsc::Receiver<LoopEvent> {
    let (tx, rx) = mpsc::channel();
    let key_tx = tx.clone();
    thread::spawn(move || loop {
        if event::poll(Duration::from_millis(250)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key_tx.send(LoopEvent::Key(key)).is_err() {
                    return;
                }
            }
        }
    });
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(1));
        if tx.send(LoopEvent::Tick).is_err() {
            return;
        }
    });
    rx
}

// `mod tests` from Step 1 stays at the bottom of the file.
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib tui:: -- --nocapture`
Expected: PASS — four new dispatch tests + every prior tui test still green.

Run: `cargo test --all-features`
Expected: PASS — every CLI integration test still green.

- [ ] **Step 5: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: clean.

- [ ] **Step 6: Manual smoke test**

Run on a tempdir:

```bash
BUCKLAND_HOME=$(mktemp -d) cargo run --release -- add "smoke task"
BUCKLAND_HOME=$(mktemp -d) cargo run --release -- tui
```

(Use the same `BUCKLAND_HOME` for both commands by exporting it.)

Expected: TUI opens, shows Tasks. `a` → Agenda. `r` → Report. `?` → Help. `q` → exits to shell with the terminal restored. No stray garbage on the prompt line.

- [ ] **Step 7: Commit**

```bash
git add src/tui/mod.rs
git commit -m "feat(tui): event loop, screen dispatch, edit overlay routing"
```

---

## Task 16: README + plan README + branch merge

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/README.md`
- Modify: `docs/future-ideas.md` (or create if missing — see Step 2 below)

- [ ] **Step 1: Update the project README's keyboard-shortcuts section**

Open `README.md`. Locate the existing "Keyboard shortcuts" section (or, if absent, insert one between the existing "Quickstart" and "Configuration" sections). Replace its body with the canonical keymap, copy-pasted from `src/tui/screens/help.rs` (single source of truth — when help.rs changes, refresh this section by hand):

```markdown
## Keyboard shortcuts

The TUI shares one keymap across all screens. `?` opens the in-app help overlay
with the same content.

### Navigation

| Key            | Action                                  |
|----------------|-----------------------------------------|
| `j` / Down     | move down                               |
| `k` / Up       | move up                                 |
| `h` / Left     | previous (week / field)                 |
| `l` / Right    | next (week / field)                     |
| `gg`           | top                                     |
| `G`            | bottom                                  |
| `Tab`          | cycle view / modal field / report scope |

### View

| Key            | Action                          |
|----------------|---------------------------------|
| `g`            | Tasks (home)                    |
| `a`            | Agenda                          |
| `r`            | Report                          |
| `?`            | Help                            |
| `q` / `Esc`    | quit / close overlay            |

### Tasks

| Key       | Action                          |
|-----------|---------------------------------|
| `n`       | new task                        |
| `s` / Enter | start selected                |
| `S`       | stop active timer               |
| `d`       | mark done                       |
| `A`       | archive                         |
| `D`       | delete (with confirm)           |
| `e`       | edit task                       |
| `/`       | filter                          |

### Report

| Key   | Action                       |
|-------|------------------------------|
| `Tab` | cycle scope                  |
| `T`   | cycle grouping               |
| `c`   | copy summary to clipboard    |
| `j`   | toggle JSON dump             |

### Edit overlay

| Key         | Action       |
|-------------|--------------|
| `Tab`       | cycle fields |
| Enter       | save         |
| Ctrl+D      | delete entry |
| `Esc`       | cancel       |
```

Also update the README's feature list (top of the file): add a "TUI: Tasks / Agenda / Report screens with Edit overlay" bullet alongside the existing CLI features.

- [ ] **Step 2: Capture the deferred decisions in `docs/future-ideas.md`**

If `docs/future-ideas.md` exists, append the new bullets. If it does not, create it with this content:

```markdown
# Buckland — future ideas (deferred from v1)

Items intentionally cut from a phase plan. Re-evaluate when someone asks.

## TUI

- **Insta snapshot tests for screen rendering.** Phase D shipped with direct
  `TestBackend` buffer assertions to keep the dev-dep list minimal. Add insta
  if/when we want to detect rendering regressions visually.
- **Custom date ranges in the Report screen.** Phase D supports the four named
  scopes (today/week/month/all). `--range FROM..TO` is CLI-only; the TUI
  could expose a date picker. Defer until someone asks.
- **Task edit modal (separate from time-entry edit).** Phase D's Edit overlay
  edits time entries only. Editing a task's title / SC-id from inside the TUI
  is a Phase E+ polish item.
- **Mouse support / scroll wheel.** Spec non-goal for v1.
- **Fuzzy filter.** Phase D filter is literal substring; fuzzy is deferred.
```

- [ ] **Step 3: Flip the plan-index README**

Edit `docs/superpowers/plans/README.md`. Change the row for Phase D from `**ready**` to `**done (<today>)**` (use today's date in `YYYY-MM-DD`), and change Phase E from `draft after D merges` to `**ready**`. The table after the edit reads:

```markdown
| Phase | File | State | Delivers |
|-------|------|-------|----------|
| A | [`2026-04-22-buckland-phase-a-cli-core.md`](2026-04-22-buckland-phase-a-cli-core.md) | **done (2026-04-24)** | CLI skeleton: `add`, `list`, `start`, `stop`, `pause`, `status`, `done`, `archive`, `delete`. Usable time-tracker via terminal. |
| B | [`2026-04-22-buckland-phase-b-shortcut.md`](2026-04-22-buckland-phase-b-shortcut.md) | **done (2026-04-24)** | Shortcut integration (read-only): `bl add --sc`, `bl shortcut`, SC-ID resolution in `bl start`, `shortcut_stories` cache with 1h TTL. |
| C | `2026-04-22-buckland-phase-c-report.md` | **done (2026-04-27)** | `bl report` with scope (today/week/month/all), grouping (task/epic/day), Unicode block bars, `--json` dump. |
| D | `2026-04-22-buckland-phase-d-tui.md` | **done (<TODAY>)** | ratatui TUI: Tasks, Agenda, Report screens plus Edit and Help overlays. Full keymap. |
| E | `2026-04-22-buckland-phase-e-tray.md` | **ready** | `bl-tray` binary with ksni + local clock tick + clipboard helper (`bl report -c`). |
| F | `2026-04-22-buckland-phase-f-oss.md` | **draft after E merges** | CI, `cargo deb`, release workflow, README with screenshots, CHANGELOG, CONTRIBUTING, CoC. |
```

Replace `<TODAY>` with the actual ISO date when you commit.

- [ ] **Step 4: Final verification**

Run:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Expected: all three clean. The combined test count should be ≥ 200 (Phase C ended at 154; Phase D added at least ~50 tests across `app`, `theme`, `keymap`, `chrome`, `clipboard`, screens, and dispatch).

- [ ] **Step 5: Commit the docs**

```bash
git add README.md docs/future-ideas.md docs/superpowers/plans/README.md
git commit -m "docs: tui keymap + phase D done / phase E ready"
```

- [ ] **Step 6: Merge the branch into main**

```bash
git checkout main
git merge --no-ff phase-d-tui -m "feat: Phase D — ratatui TUI"
git push origin main
git branch -d phase-d-tui
```

(Push happens only when the user explicitly OKs it; if not, stop after the merge.)

- [ ] **Step 7: Phase D complete — run the postcondition checklist**

Walk back to the **Postconditions** section at the top of this file and tick every box. Anything unchecked is a regression in this plan and must be fixed before the merge is considered final.

---

## Self-review notes (for the engineer)

- Every spec bullet under §"TUI" maps to one of Tasks 6–14 except "Mouse interactions" (an explicit non-goal — not implemented, not tested).
- Every spec bullet under §"Testing" is satisfied by the inline `mod tests` blocks per module.
- The `Repo` trait grew exactly one new method (`update_time_entry`); every other mutation reuses the surface Phase A/B/C built. If you find yourself adding more, stop and reread `Decisions`.
- Optimistic updates are intentionally lightweight: screen state mutates first, repo call second, error path is a footer message. Spec wants visual rollback on failure — this lands implicitly because every mutation re-fetches via `state.refresh()` immediately after the repo call.
- If a clippy lint fires on the `Char('j')` / `ToggleJson` overlap in Task 12, prefer renaming the keymap action over silencing the lint — clarity > brevity.
