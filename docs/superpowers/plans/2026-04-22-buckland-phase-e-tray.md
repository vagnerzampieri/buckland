---
title: "Buckland Phase E — Tray, clipboard helper, bl tray"
phase: E
status: ready
date: 2026-04-22
previous: 2026-04-22-buckland-phase-d-tui.md
next: 2026-04-22-buckland-phase-f-oss.md
---

# Buckland Phase E — Tray, clipboard helper, `bl tray`

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended — fresh subagent per task) or `superpowers:executing-plans` (inline). Steps use checkbox (`- [ ]`) syntax for tracking. Each task ends with exactly one commit. Do **not** merge to `main` without the user running the smoke checklist in Task E8 and giving an explicit OK.

**Goal:** Ship the `bl-tray` passive tray icon, a `bl tray` subcommand that runs the same loop, and a Linux clipboard helper used by `bl report --copy` and the TUI Report `c` shortcut. Phase E is the last surface the spec promises before OSS release (Phase F).

**Architecture:** A new `src/tray/` module sits next to `cli/`, `tui/`, and `shortcut/` as a fourth surface. It exposes:

- `tray::assets` — embedded SVG byte slices for the three icon states.
- `tray::state` — the pure `TrayState` enum + tooltip / icon-mapping / transition functions.
- `tray::runtime` — the ksni `Tray` impl + a polling thread + a 1Hz `glib::timeout_add_local` tick.

`bl-tray` and `bl tray` are two entry points to the same `tray::runtime::run(...)` function — no logic duplication. Read access to the database goes through a new `SqliteRepo::open_read_only(&Path)` constructor (the polling thread owns its own connection in read-only mode; never the writer's). A new `Repo::active_snapshot()` query joins `time_entries`, `tasks`, and `shortcut_stories` so the tray gets everything it needs in one round-trip.

A new top-level `src/clipboard.rs` replaces `src/tui/clipboard.rs`. Phase D put clipboard inside `tui::clipboard` because only the TUI needed it; Phase E promotes it so the CLI's `bl report --copy` can reach it without depending on the `tui` module. The `tui::clipboard` re-export becomes a one-line delegation to the new top-level module so existing callers stay green.

**Tech stack additions:** `ksni = "0.3"` and `glib = "0.20"`, both gated by the existing `tray` feature. No async runtime — polling is `std::thread::spawn` + `Arc<Mutex<TrayState>>`; the 1Hz tick is `glib::timeout_add_local`. SVGs are embedded via `include_bytes!`. The tray uses ksni's named-icon path (`icon_name`) by installing the SVGs as theme icons under `~/.local/share/icons/hicolor/scalable/apps/` at runtime; this avoids a heavy raster dependency (`tiny-skia`/`resvg`) — see Decisions.

---

## What this phase delivers

CLI surface added in this phase:

```
bl tray
    Runs the tray-icon process until SIGINT or "Quit" from the menu. Same
    loop as the bl-tray binary; both call into tray::runtime::run().

bl report --copy
bl report -c
    Copies a one-line summary to the system clipboard via wl-copy (Wayland)
    or xclip (X11). Mutually exclusive with stdout: when --copy is set,
    nothing is printed to stdout (success messages go to stderr). When
    --copy is combined with --json, the JSON body is what gets copied.
```

Behavioral changes layered on top of Phase D:

- `bl-tray` (already a binary stub since Phase A) becomes functional. The placeholder `println!` body is replaced with a real call to `tray::runtime::run(...)`.
- The TUI Report screen's `c` keymap (already wired in Phase D against `tui::clipboard::copy`) keeps working — the helper module is moved to `crate::clipboard` and `tui::clipboard` becomes a thin re-export.
- `Config.tray.poll_seconds` (already in `Config` since Phase D's TrayConfig) becomes load-bearing: the polling thread reads it once at startup.
- `bl config get tray.poll_seconds` and `bl config set tray.poll_seconds <n>` work via the existing config plumbing — Phase E only verifies the dot-key path resolves and adds a unit test.

User-facing surface in the tray:

- **Icon states** — idle (outlined clock), running (filled clock), error (clock + exclamation). Swapped only on state transitions; tooltip recomputes every second from `started_at`.
- **Tooltip** — see "Tooltip strings" under Decisions for exact wording.
- **Menu** — one item, "Quit". Everything else goes through `bl` CLI/TUI (passive-tray decision in the spec).

## Required reading (load before executing the first task)

- **Spec:** `docs/superpowers/specs/2026-04-22-buckland-design.md`. Sections that matter most:
  - "Architecture" → "Crate layout" (the `src/tray/` block is implicit there; read the surrounding text for the surface boundary).
  - "Feature flags" — confirms `tray = ["dep:ksni", "dep:glib"]`.
  - "Command Grammar" → `bl tray` and `bl report` (note the `-c` / `--copy` flag was deferred to Phase E in the original Phase C plan).
  - "TUI" → keymap row for `c` (already implemented in Phase D against `tui::clipboard`).
  - "Tray" — entire section, including the GNOME AppIndicator caveat.
  - "Testing" — "Tray — manual smoke test documented in CONTRIBUTING."
  - "Open Questions" — clipboard library decision (we lock in OS commands, not `arboard`).
- **Project guidelines:** `CLAUDE.md`. Sections:
  - "Tray Conventions (ksni)" — the entire section is the contract for this phase.
  - "Rust Idioms First" — sync by default, `thiserror` in lib + `anyhow` at bin entry.
  - "Object Design" — file-size budget (≤ ~300 LOC per file including tests).
- **Prior phases:**
  - `docs/superpowers/plans/2026-04-22-buckland-phase-a-cli-core.md` — `Repo` trait shape, `Context::open`, integration test harness with `assert_cmd` + `tempfile`.
  - `docs/superpowers/plans/2026-04-22-buckland-phase-c-report.md` — `bl report` body — Phase E adds one flag without touching the report builder.
  - `docs/superpowers/plans/2026-04-22-buckland-phase-d-tui.md` — `tui::clipboard` (we move it) and `Config.tray.poll_seconds` (we wire it).
- **External:**
  - ksni 0.3 docs (https://docs.rs/ksni/0.3) — focus on `Tray` trait, `service_handle.update()`, `MenuItem` / `StandardItem`.
  - glib 0.20 docs (https://docs.rs/glib/0.20) — focus on `MainLoop::new`, `timeout_add_local`, `ControlFlow`.
  - StatusNotifierItem spec (https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/) — for the icon-name-vs-pixmap question.

## Preconditions

Before starting Task E1:

- [ ] `git status` on `main` is clean.
- [ ] Phase D is marked `**done (<date>)**` in `docs/superpowers/plans/README.md`.
- [ ] `cargo test --all-features` passes (run it once to confirm green baseline).
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` is clean.
- [ ] `cargo fmt --all --check` is clean.
- [ ] You are on branch `phase-e-tray` (already created by the user). Confirm with `git status -sb` — output should start with `## phase-e-tray`.
- [ ] On a system with `libdbus-1-dev` and `libglib2.0-dev` installed (Debian/Ubuntu) so the `tray` feature compiles. KDE Plasma, XFCE, or Cinnamon for visual smoke; on GNOME, install the AppIndicator extension first.

## Postconditions (how to verify Phase E is done)

After Task E8:

- [ ] `cargo test --all-features` green; `cargo clippy --all-targets --all-features -- -D warnings` clean; `cargo fmt --all --check` clean.
- [ ] `cargo build --no-default-features` green (proves the `tray` feature is properly gated).
- [ ] `bl --help` lists `tray` as a subcommand.
- [ ] `bl tray --help` exits 0 with descriptive text.
- [ ] `bl report --help` lists `--copy` / `-c`.
- [ ] `bl-tray` (built with default features) launches, shows the idle icon, swaps to running when a `bl start` runs in another terminal, and quits cleanly via the menu.
- [ ] `bl report --copy` on Wayland (`echo $WAYLAND_DISPLAY` non-empty) lands a one-liner in the clipboard verifiable with `wl-paste`. Same on X11 with `xclip -o`.
- [ ] TUI `c` shortcut on the Report screen still works (regression test from Phase D unchanged).
- [ ] `Cargo.toml`'s `version` is `0.5.0`.
- [ ] `CHANGELOG.md` exists and has a `## [0.5.0] - <today>` section listing the three Phase E additions.
- [ ] `docs/smoke-tests/phase-e-tray.md` exists, registered in `docs/smoke-tests/README.md`.
- [ ] `docs/superpowers/plans/README.md` shows Phase E as `**done (<today>)**` and Phase F as `**ready**`.
- [ ] User has run `docs/smoke-tests/phase-e-tray.md` end-to-end and given an explicit "merge OK" message. Branch `phase-e-tray` is **not** merged into `main` until that OK lands. (Smoke test is the merge gate.)

## Architecture (in scope for this phase)

```
buckland/
├── Cargo.toml              # +ksni, +glib (both optional, gated by tray feature)
├── CHANGELOG.md            # NEW (or amended) — 0.5.0 entry
├── resources/              # NEW directory — embedded assets
│   ├── tray-idle.svg
│   ├── tray-running.svg
│   ├── tray-error.svg
│   ├── buckland.svg
│   └── buckland-tray.desktop
├── src/
│   ├── clipboard.rs        # NEW top-level module (was src/tui/clipboard.rs)
│   ├── tray/               # NEW module (feature-gated)
│   │   ├── mod.rs          # pub use re-exports + cfg gate
│   │   ├── assets.rs       # include_bytes! constants for each SVG
│   │   ├── state.rs        # TrayState, tooltip, icon_for, transition, poll_to_state
│   │   └── runtime.rs      # TrayRuntimeConfig + run() — ksni service + polling + glib tick
│   ├── domain/
│   │   └── active_snapshot.rs  # NEW: ActiveSnapshot struct (joined view) + minor mod.rs add
│   ├── storage/
│   │   └── repo.rs         # +open_read_only, +active_snapshot, +RepoError::DatabaseMissing
│   ├── cli/
│   │   ├── args.rs         # +Tray variant (feature-gated), +Report.copy field
│   │   ├── mod.rs          # +Commands::Tray dispatch
│   │   └── commands/
│   │       ├── report.rs   # +copy path
│   │       └── tray.rs     # NEW: thin wrapper around tray::runtime::run
│   ├── tui/
│   │   └── clipboard.rs    # SHRINKS: pub use crate::clipboard::*; (or deleted, see Task E6)
│   ├── config.rs           # already has TrayConfig — Phase E only adds tests
│   ├── lib.rs              # +pub mod clipboard; +#[cfg(feature = "tray")] pub mod tray;
│   └── bin/
│       └── bl-tray.rs      # body replaced: derive db_path, call tray::runtime::run
└── tests/
    ├── cli_tray.rs                # NEW: bl tray --help smoke test
    ├── cli_report_copy.rs         # NEW: --copy flag end-to-end via PATH-shimmed wl-copy
    └── cli_report.rs              # untouched
```

`src/lib.rs` after this phase reads:

```rust
//! Buckland — personal time tracker core library.

pub mod cli;
pub mod clipboard;
pub mod config;
pub mod domain;
pub mod shortcut;
pub mod storage;
#[cfg(feature = "tray")]
pub mod tray;
pub mod tui;
```

## Tech stack (this phase)

| Concern | Choice | Why |
|---------|--------|-----|
| Tray host (StatusNotifierItem) | `ksni = "0.3"` | Spec mandate. Latest 0.3.x as of writing. |
| GLib bindings (event loop, 1Hz tick) | `glib = "0.20"` | Spec mandate. ksni's docs use it for the main loop on the calling thread. |
| Icon delivery to tray | Theme icon names + on-startup SVG copy to `~/.local/share/icons/hicolor/scalable/apps/` | Avoids a raster dep. ksni accepts a string returned from `icon_name()`. SVGs are embedded with `include_bytes!`; on first run we write them to the user's icon dir if the bytes differ from disk. |
| Active-timer DB read | New `Repo::active_snapshot()` joining `time_entries`, `tasks`, `shortcut_stories` | One round-trip, one `&Connection`. No N+1. |
| DB connection mode | `OpenFlags::SQLITE_OPEN_READ_ONLY \| SQLITE_OPEN_NO_MUTEX` | Tray must never write. SQLite enforces it. |
| Clipboard (wl-copy / xclip) | Shell-out via `std::process::Command` with stdin pipe | Same pattern Phase D used. We move and harden it. |
| Tooltip clock | Recompute from `started_at` on every glib tick | Same correctness argument as the TUI header in Phase D — DB row is the source of truth, no drift. |
| Polling | `std::thread::spawn` reading `Arc<Mutex<TrayState>>` + a `service.update()` call when state class changes | Sync. No tokio. |

## Decisions (lock in before coding)

- **Icon delivery uses theme icon names, not raw pixmaps.** ksni's `Tray::icon_name()` returns a `String` that the host (KDE Plasma's status applet, GNOME's AppIndicator extension, KsTrayer) resolves against the freedesktop icon theme. We install our three SVGs into `~/.local/share/icons/hicolor/scalable/apps/buckland-tray-{idle,running,error}.svg` at startup if absent (or stale, matched by exact byte equality with the embedded constants). This sidesteps the SVG-rasterization cost of returning ARGB pixmaps and keeps the dep list at ksni + glib. If a host can't find the named icon, ksni falls back to a default — we accept that as a documented constraint in the smoke test.
- **`bl tray` and `bl-tray` share `tray::runtime::run`.** Both bins call the same function with the same `TrayRuntimeConfig`. Process supervision (autostart, systemd user unit) is left to the user — the `.desktop` file shipped in `resources/` is opt-in.
- **Read-only DB access only.** The polling thread opens its own connection with `OpenFlags::SQLITE_OPEN_READ_ONLY`. Any attempted write panics in tests and produces a `RepoError::Sqlite` in production; the tray treats both as "Error" state.
- **Polling interval is config-driven, not hardcoded.** `Config.tray.poll_seconds` (already added in Phase D) is the source. Default 30s. Read once at startup; changes to the config require a tray restart (documented).
- **`--copy` and `--json` interaction:** when both flags are set on `bl report`, the JSON body is what gets copied. This matches the principle of least surprise — `--json` controls the *shape* of the report, `--copy` controls the *destination*. Documented in `--help`.
- **Tooltip strings** (locked exact wording so tests assert on them):
  - `Idle` → `"Buckland: idle"`
  - `Active` with SC linked → `"SC-{external_id} {task_title} — {HH:MM:SS} (started {HH:MM} local)"`, e.g. `"SC-123 Fix login flow — 01:23:45 (started 09:15)"`
  - `Active` without SC → `"#{task_id} {task_title} — {HH:MM:SS} (started {HH:MM})"`, e.g. `"#7 Refactor imports — 00:12:03 (started 14:02)"`
  - `NoDatabase` → `"Buckland: no database yet"`
  - `Error` → `"Buckland: cannot read database — {reason}"` (reason is the short error message, truncated to 60 chars).
- **Error state never blocks the loop.** A failed read flips the icon to error and updates the tooltip; the next poll either recovers (back to idle/active) or stays in error.
- **Clipboard tool detection is env-driven, not capability-probed.** `WAYLAND_DISPLAY` non-empty → `wl-copy`. Else `DISPLAY` non-empty → `xclip -selection clipboard`. Else `ClipboardError::NoServer`. We do NOT try to spawn each binary in succession (Phase D's `try_copy_with` did that as a fallback chain — Phase E keeps that behavior internally for robustness, but the user-visible error message names the *expected* tool based on the env).
- **The existing `tui::clipboard` import path stays valid** via a one-line `pub use crate::clipboard::*;` after the move. We do not break Phase D callers.
- **Version bump is the final commit of Task E8** (mirroring Phase D's `chore: bump version to 0.4.0 (Phase D)` commit `01e890b`). The bump is included in the same commit that updates `CHANGELOG.md` and the plan index.
- **No CI changes in this phase.** CI tweaks for the tray feature (e.g. installing libdbus on the runner) are deferred to Phase F's CI work.

## Out of scope

- Tray menu items beyond "Quit" (spec non-goal: passive tray).
- Tray running on macOS or Windows (`ksni` is Linux-only by design).
- Auto-rasterizing SVGs to PNG inside the binary (`resvg`, `tiny-skia`) — we lean on theme icons.
- Auto-detection of the desktop environment to install GNOME's AppIndicator extension. We document the prerequisite in the smoke test.
- Notifications (toast on timer state change). Out of scope for v1.
- Tray-driven actions (Start / Stop from the tray menu). Spec non-goal.
- Caching `wl-copy` / `xclip` discovery. Negligible cost on the second call.

## Dependencies (crate adds)

```toml
# In [dependencies], move ksni and glib to optional with the tray feature.
ksni = { version = "0.3", optional = true }
glib = { version = "0.20", optional = true }

# In [features]:
default = ["tray"]
tray = ["dep:ksni", "dep:glib"]
```

No new dev-dependencies. We use `tempfile` (already in dev-deps) for the `cli_report_copy.rs` PATH-shim test.

## Next phase

Phase F — OSS readiness — becomes `ready` once Phase E merges. Phase F adds CI workflows (including a Linux-only `tray` feature smoke build), `cargo deb` packaging, the README screenshot pass, `CHANGELOG.md` polish, `CONTRIBUTING.md`, and `CODE_OF_CONDUCT.md`. Nothing in `src/tray/` is touched.

---

## E1 — Tray resources + cargo deps

**Goal:** Land the embedded SVG assets, the `.desktop` autostart file, the new optional crate deps, and the `tray::assets` module that exposes them as byte slices. After this task, `cargo build --features tray` compiles cleanly, no behavior is wired up yet, and the assets module has a unit test asserting non-emptiness.

**Files:**
- Create: `resources/tray-idle.svg`
- Create: `resources/tray-running.svg`
- Create: `resources/tray-error.svg`
- Create: `resources/buckland.svg`
- Create: `resources/buckland-tray.desktop`
- Create: `src/tray/mod.rs`
- Create: `src/tray/assets.rs`
- Modify: `Cargo.toml`
- Modify: `src/lib.rs`

- [ ] **Step 1 (red): write the failing assets test**

Create `src/tray/assets.rs` with the test only:

```rust
//! Embedded SVG assets for the tray icon. Assets are bundled at build
//! time via `include_bytes!` so the binary does not depend on a writable
//! resources/ directory at runtime.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_svg_is_non_empty_and_starts_with_svg_tag() {
        assert!(!TRAY_IDLE_SVG.is_empty());
        assert!(TRAY_IDLE_SVG.starts_with(b"<?xml") || TRAY_IDLE_SVG.starts_with(b"<svg"));
    }

    #[test]
    fn running_svg_is_non_empty_and_starts_with_svg_tag() {
        assert!(!TRAY_RUNNING_SVG.is_empty());
        assert!(TRAY_RUNNING_SVG.starts_with(b"<?xml") || TRAY_RUNNING_SVG.starts_with(b"<svg"));
    }

    #[test]
    fn error_svg_is_non_empty_and_starts_with_svg_tag() {
        assert!(!TRAY_ERROR_SVG.is_empty());
        assert!(TRAY_ERROR_SVG.starts_with(b"<?xml") || TRAY_ERROR_SVG.starts_with(b"<svg"));
    }

    #[test]
    fn app_icon_svg_is_non_empty_and_starts_with_svg_tag() {
        assert!(!APP_ICON_SVG.is_empty());
        assert!(APP_ICON_SVG.starts_with(b"<?xml") || APP_ICON_SVG.starts_with(b"<svg"));
    }
}
```

Run: `cargo test --features tray --lib tray::assets -- --nocapture`
Expected: FAIL — `TRAY_IDLE_SVG`, `TRAY_RUNNING_SVG`, `TRAY_ERROR_SVG`, `APP_ICON_SVG` are not yet defined; the module isn't even reachable from `lib.rs` yet.

- [ ] **Step 2 (green): write the four SVG resource files**

Create `resources/tray-idle.svg` (outlined clock — neutral state):

```xml
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64">
  <circle cx="32" cy="32" r="26" fill="none" stroke="currentColor" stroke-width="4"/>
  <line x1="32" y1="32" x2="32" y2="14" stroke="currentColor" stroke-width="4" stroke-linecap="round"/>
  <line x1="32" y1="32" x2="46" y2="32" stroke="currentColor" stroke-width="4" stroke-linecap="round"/>
  <circle cx="32" cy="32" r="2" fill="currentColor"/>
</svg>
```

Create `resources/tray-running.svg` (filled clock — active state):

```xml
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64">
  <circle cx="32" cy="32" r="26" fill="currentColor"/>
  <line x1="32" y1="32" x2="32" y2="14" stroke="#ffffff" stroke-width="4" stroke-linecap="round"/>
  <line x1="32" y1="32" x2="46" y2="32" stroke="#ffffff" stroke-width="4" stroke-linecap="round"/>
  <circle cx="32" cy="32" r="2" fill="#ffffff"/>
</svg>
```

Create `resources/tray-error.svg` (clock + exclamation — error state):

```xml
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64">
  <circle cx="32" cy="32" r="26" fill="none" stroke="currentColor" stroke-width="4"/>
  <line x1="32" y1="18" x2="32" y2="36" stroke="currentColor" stroke-width="6" stroke-linecap="round"/>
  <circle cx="32" cy="44" r="3" fill="currentColor"/>
</svg>
```

Create `resources/buckland.svg` (application icon — used as the desktop icon):

```xml
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128" width="128" height="128">
  <rect x="8" y="8" width="112" height="112" rx="20" ry="20" fill="#2563eb"/>
  <circle cx="64" cy="64" r="44" fill="none" stroke="#ffffff" stroke-width="6"/>
  <line x1="64" y1="64" x2="64" y2="32" stroke="#ffffff" stroke-width="6" stroke-linecap="round"/>
  <line x1="64" y1="64" x2="92" y2="64" stroke="#ffffff" stroke-width="6" stroke-linecap="round"/>
  <circle cx="64" cy="64" r="4" fill="#ffffff"/>
</svg>
```

Create `resources/buckland-tray.desktop` (autostart file — disabled by default; user opts in via `cp resources/buckland-tray.desktop ~/.config/autostart/`):

```ini
[Desktop Entry]
Type=Application
Name=Buckland Tray
Comment=Time tracker tray icon
Exec=bl-tray
Icon=buckland
Terminal=false
Categories=Utility;Office;
StartupNotify=false
X-GNOME-Autostart-enabled=true
NoDisplay=false
```

- [ ] **Step 3 (green): replace `src/tray/assets.rs` with the byte constants**

Overwrite `src/tray/assets.rs`:

```rust
//! Embedded SVG assets for the tray icon. Assets are bundled at build
//! time via `include_bytes!` so the binary does not depend on a writable
//! resources/ directory at runtime.
//!
//! These constants are also used at startup by `tray::runtime` to install
//! the icons under `~/.local/share/icons/hicolor/scalable/apps/` so the
//! StatusNotifierItem host can resolve them by theme name.

pub const TRAY_IDLE_SVG: &[u8] = include_bytes!("../../resources/tray-idle.svg");
pub const TRAY_RUNNING_SVG: &[u8] = include_bytes!("../../resources/tray-running.svg");
pub const TRAY_ERROR_SVG: &[u8] = include_bytes!("../../resources/tray-error.svg");
pub const APP_ICON_SVG: &[u8] = include_bytes!("../../resources/buckland.svg");

/// The freedesktop icon-theme names we expose. Hosts resolve these
/// against `~/.local/share/icons/hicolor/scalable/apps/<name>.svg`.
pub const ICON_NAME_IDLE: &str = "buckland-tray-idle";
pub const ICON_NAME_RUNNING: &str = "buckland-tray-running";
pub const ICON_NAME_ERROR: &str = "buckland-tray-error";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_svg_is_non_empty_and_starts_with_svg_tag() {
        assert!(!TRAY_IDLE_SVG.is_empty());
        assert!(TRAY_IDLE_SVG.starts_with(b"<?xml") || TRAY_IDLE_SVG.starts_with(b"<svg"));
    }

    #[test]
    fn running_svg_is_non_empty_and_starts_with_svg_tag() {
        assert!(!TRAY_RUNNING_SVG.is_empty());
        assert!(TRAY_RUNNING_SVG.starts_with(b"<?xml") || TRAY_RUNNING_SVG.starts_with(b"<svg"));
    }

    #[test]
    fn error_svg_is_non_empty_and_starts_with_svg_tag() {
        assert!(!TRAY_ERROR_SVG.is_empty());
        assert!(TRAY_ERROR_SVG.starts_with(b"<?xml") || TRAY_ERROR_SVG.starts_with(b"<svg"));
    }

    #[test]
    fn app_icon_svg_is_non_empty_and_starts_with_svg_tag() {
        assert!(!APP_ICON_SVG.is_empty());
        assert!(APP_ICON_SVG.starts_with(b"<?xml") || APP_ICON_SVG.starts_with(b"<svg"));
    }

    #[test]
    fn icon_names_are_unique_and_kebab_cased() {
        let names = [ICON_NAME_IDLE, ICON_NAME_RUNNING, ICON_NAME_ERROR];
        let mut sorted = names.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 3, "icon names must be distinct");
        for n in &names {
            assert!(n.starts_with("buckland-"));
            assert!(n.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
        }
    }
}
```

- [ ] **Step 4 (green): create the tray module entry point**

Create `src/tray/mod.rs`:

```rust
//! The `bl-tray` surface — a passive StatusNotifierItem icon that
//! polls the SQLite file in read-only mode and surfaces the active
//! timer's state via tooltip + icon swap.
//!
//! Layered as:
//!
//! - [`assets`]  — embedded SVG byte slices.
//! - [`state`]   — pure `TrayState` + tooltip/icon mappers (Task E3).
//! - [`runtime`] — ksni service + polling thread + glib tick (Task E4).

pub mod assets;
// state and runtime are added by Tasks E3 and E4.
```

- [ ] **Step 5 (green): wire the module into `src/lib.rs`**

Edit `src/lib.rs` — add a feature-gated `tray` line. After the edit:

```rust
//! Buckland — personal time tracker core library.

pub mod cli;
pub mod config;
pub mod domain;
pub mod shortcut;
pub mod storage;
#[cfg(feature = "tray")]
pub mod tray;
pub mod tui;
```

(The new `pub mod clipboard;` line is added in Task E6, not here. Don't add it now.)

- [ ] **Step 6 (green): add ksni + glib deps**

Edit `Cargo.toml`. The `[features]` block becomes:

```toml
[features]
default = ["tray"]
tray = ["dep:ksni", "dep:glib"]
```

In `[dependencies]`, add the two optional deps in alphabetical order:

```toml
glib = { version = "0.20", optional = true }
# ... existing entries ...
ksni = { version = "0.3", optional = true }
```

The complete `[dependencies]` block after the edit reads:

```toml
[dependencies]
anyhow = "1"
chrono = { version = "0.4", default-features = false, features = ["clock", "serde"] }
clap = { version = "4", features = ["derive"] }
crossterm = "0.28"
dirs = "5"
glib = { version = "0.20", optional = true }
ksni = { version = "0.3", optional = true }
ratatui = "0.29"
reqwest = { version = "0.12", default-features = false, features = ["blocking", "rustls-tls", "json"] }
rusqlite = { version = "0.32", features = ["bundled", "chrono"] }
rusqlite_migration = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
toml = "0.8"
```

- [ ] **Step 7: Verify**

Run:

```bash
cargo build --features tray
cargo build --no-default-features
cargo test --features tray --lib tray::assets
cargo test --no-default-features --lib   # tray module is gated out
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

Expected: every command green. The `--no-default-features` test run should not even attempt to compile `src/tray/`.

- [ ] **Step 8: Commit**

```text
chore(tray): embed SVG assets and add ksni/glib feature deps
```

Files in this commit: `Cargo.toml`, `Cargo.lock`, `resources/tray-idle.svg`, `resources/tray-running.svg`, `resources/tray-error.svg`, `resources/buckland.svg`, `resources/buckland-tray.desktop`, `src/tray/mod.rs`, `src/tray/assets.rs`, `src/lib.rs`.

```bash
git add Cargo.toml Cargo.lock resources/ src/tray/ src/lib.rs
git commit -m "chore(tray): embed SVG assets and add ksni/glib feature deps"
```

---

## E2 — Read-only `Repo` open + `active_snapshot` query

**Goal:** Give the tray a way to read the database without ever holding the writer's connection. Add `SqliteRepo::open_read_only(&Path)`, a new `RepoError::DatabaseMissing` variant, an `ActiveSnapshot` value type, and a `Repo::active_snapshot()` method that joins three tables in one round-trip.

**Files:**
- Create: `src/domain/active_snapshot.rs`
- Modify: `src/domain/mod.rs`
- Modify: `src/storage/repo.rs`

- [ ] **Step 1 (red): write the failing tests for `active_snapshot`**

Add the following test block to `src/storage/repo.rs` inside `mod tests`:

```rust
#[test]
fn active_snapshot_returns_none_when_idle() {
    let r = repo();
    assert!(r.active_snapshot().unwrap().is_none());
}

#[test]
fn active_snapshot_returns_task_and_started_at_for_active_entry() {
    use chrono::TimeZone;
    let mut r = repo();
    let t = r.create_task("fix login", None).unwrap();
    let started = Utc.with_ymd_and_hms(2026, 4, 22, 9, 15, 0).unwrap();
    r.create_time_entry(t.id, started).unwrap();

    let snap = r.active_snapshot().unwrap().expect("active snapshot");
    assert_eq!(snap.task_id, t.id);
    assert_eq!(snap.task_title, "fix login");
    assert_eq!(snap.sc_external_id, None);
    assert_eq!(snap.started_at, started);
}

#[test]
fn active_snapshot_includes_shortcut_external_id_when_linked() {
    use crate::shortcut::Story;
    use chrono::TimeZone;
    let mut r = repo();
    let row = r
        .upsert_shortcut_story(
            &Story {
                external_id: 4242,
                title: Some("SC story".into()),
                epic_id: None,
                epic_name: None,
                state: None,
            },
            Utc::now(),
        )
        .unwrap();
    let t = r.create_task("linked", None).unwrap();
    r.link_task_to_story(t.id, row.id, Utc::now()).unwrap();
    let started = Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap();
    r.create_time_entry(t.id, started).unwrap();

    let snap = r.active_snapshot().unwrap().expect("active");
    assert_eq!(snap.sc_external_id, Some(4242));
    assert_eq!(snap.task_title, "linked");
}
```

Add tests for `open_read_only`. Append to the same `mod tests`:

```rust
#[test]
fn open_read_only_rejects_writes() {
    use crate::storage::open;
    use rusqlite::ErrorCode;
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("buckland.db");
    // Create the schema with a writable open first so the DB exists.
    let _ = open(&path).unwrap();

    let ro = SqliteRepo::open_read_only(&path).expect("open ro");
    let res = ro.connection().execute(
        "INSERT INTO tasks (title, created_at, updated_at) VALUES ('x', ?1, ?1)",
        rusqlite::params![Utc::now()],
    );
    let err = res.expect_err("read-only must reject writes");
    let rusqlite::Error::SqliteFailure(e, _) = err else {
        panic!("expected SqliteFailure, got {err:?}");
    };
    assert_eq!(e.code, ErrorCode::ReadOnly);
}

#[test]
fn open_read_only_returns_database_missing_when_file_absent() {
    use crate::storage::repo::RepoError;
    use std::path::Path;
    let res = SqliteRepo::open_read_only(Path::new("/tmp/buckland-does-not-exist-zzz.db"));
    match res {
        Err(RepoError::DatabaseMissing(_)) => {}
        other => panic!("expected DatabaseMissing, got {other:?}"),
    }
}
```

Run: `cargo test --lib storage::repo::tests::active_snapshot --lib storage::repo::tests::open_read_only -- --nocapture`
Expected: FAIL — `active_snapshot`, `open_read_only`, `RepoError::DatabaseMissing`, `ActiveSnapshot` are not yet defined.

- [ ] **Step 2 (green): create the `ActiveSnapshot` domain type**

Create `src/domain/active_snapshot.rs`:

```rust
use chrono::{DateTime, Utc};

/// A flat view of the currently active timer with everything the tray
/// needs to render its tooltip in one allocation.
///
/// Produced by [`crate::storage::Repo::active_snapshot`] from a single
/// `JOIN` across `time_entries`, `tasks`, and `shortcut_stories`. Pure
/// data; the tray's display logic lives in `crate::tray::state`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveSnapshot {
    /// The active task's primary key.
    pub task_id: i64,
    /// The task's user-facing title.
    pub task_title: String,
    /// The Shortcut story external id, if the task is linked. `None`
    /// for unlinked tasks.
    pub sc_external_id: Option<i64>,
    /// The timer's started timestamp, in UTC.
    pub started_at: DateTime<Utc>,
}
```

Edit `src/domain/mod.rs` to register and re-export the new module:

```rust
//! Domain types and operations.

pub mod active_snapshot;
pub mod report;
pub mod shortcut_story;
pub mod task;
pub mod time_entry;
pub mod timer_ops;

pub use active_snapshot::ActiveSnapshot;
pub use report::{Grouping, Report, ReportBuilder, ReportRow, Scope, ScopeError, ScopeKind};
pub use shortcut_story::ShortcutStory;
pub use task::Task;
pub use time_entry::TimeEntry;
pub use timer_ops::TimerOps;
```

- [ ] **Step 3 (green): extend `RepoError` and add the `Repo::active_snapshot` method**

Edit `src/storage/repo.rs`:

In `RepoError`, add the new variant:

```rust
#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("task {0} not found")]
    TaskNotFound(i64),
    #[error("time entry {0} not found")]
    TimeEntryNotFound(i64),
    #[error("cannot hard-delete task {0}: it has time entries. Use archive instead.")]
    TaskHasEntries(i64),
    /// The database file does not exist on disk. Distinct from a generic
    /// open error so the tray can render "no database yet" instead of an
    /// alarming "cannot read database" message.
    #[error("database file not found: {0}")]
    DatabaseMissing(std::path::PathBuf),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}
```

In the `Repo` trait, add the `active_snapshot` method (right after `active_time_entry`):

```rust
/// One-shot read of the currently active timer joined with its task
/// and (if linked) the Shortcut story's `external_id`. Returns
/// `Ok(None)` when no row in `time_entries` has `ended_at IS NULL`.
fn active_snapshot(&self) -> RepoResult<Option<crate::domain::ActiveSnapshot>>;
```

In `impl SqliteRepo`, add the new constructor right after `pub fn new(...)`:

```rust
/// Open the database at `path` for read-only access. Returns
/// [`RepoError::DatabaseMissing`] if the file does not exist (so the
/// tray can render "no database yet" without alarming the user) and
/// [`RepoError::Sqlite`] for any other failure (locked, corrupt,
/// permission denied).
pub fn open_read_only(path: &std::path::Path) -> RepoResult<Self> {
    if !path.exists() {
        return Err(RepoError::DatabaseMissing(path.to_path_buf()));
    }
    let conn = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    // Foreign keys aren't enforced by default on read-only connections;
    // we don't need them for SELECTs but turning them on costs nothing.
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "busy_timeout", 3000)?;
    Ok(Self { conn })
}
```

In `impl Repo for SqliteRepo`, add the `active_snapshot` method body (place it after `active_time_entry` so the source order matches the trait):

```rust
fn active_snapshot(&self) -> RepoResult<Option<crate::domain::ActiveSnapshot>> {
    use crate::domain::ActiveSnapshot;
    let sql = "SELECT te.task_id, t.title, ss.external_id, te.started_at \
               FROM time_entries te \
               JOIN tasks t ON t.id = te.task_id \
               LEFT JOIN shortcut_stories ss ON ss.id = t.shortcut_story_id \
               WHERE te.ended_at IS NULL \
               LIMIT 1";
    self.conn
        .query_row(sql, [], |row| {
            Ok(ActiveSnapshot {
                task_id: row.get(0)?,
                task_title: row.get(1)?,
                sc_external_id: row.get(2)?,
                started_at: row.get(3)?,
            })
        })
        .optional()
        .map_err(RepoError::from)
}
```

- [ ] **Step 4 (verify)**

Run:

```bash
cargo test --lib storage::repo
cargo test --lib domain::active_snapshot
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

Expected: green. The two new failing tests now pass; the existing 154+ tests stay green.

- [ ] **Step 5 (refactor): document the join in a doc comment on the SQL**

Optional cleanup — extract the `LEFT JOIN ss` SQL string into a `const ACTIVE_SNAPSHOT_SQL: &str` at the top of `impl Repo for SqliteRepo` if and only if you find yourself reading it twice while writing the test. If you don't, skip — premature DRY is more expensive than three copies (we have one).

- [ ] **Step 6: Commit**

```text
feat(storage): read-only Repo open and active_snapshot join
```

```bash
git add src/domain/ src/storage/repo.rs
git commit -m "feat(storage): read-only Repo open and active_snapshot join"
```

---

## E3 — Tray state machine + tooltip formatting (pure logic)

**Goal:** Encode the tray's behavior as pure functions on a `TrayState` enum — no I/O, no glib, no ksni. Tooltip text, icon-name selection, and "did the icon-class change since last poll" are all unit-testable in isolation.

**Files:**
- Create: `src/tray/state.rs`
- Modify: `src/tray/mod.rs`

- [ ] **Step 1 (red): write the failing pure-function tests**

Create `src/tray/state.rs` with the test stub first:

```rust
//! Pure logic for the tray surface. No I/O, no glib, no ksni.
//!
//! - [`TrayState`] models the four possible visible states.
//! - [`tooltip`] renders the SNI tooltip text from a `TrayState` + the
//!   current local time (used by Active to compute elapsed seconds).
//! - [`icon_name`] picks the freedesktop icon-theme name to expose.
//! - [`transition`] decides whether a state change warrants a redraw
//!   (vs a tooltip-only refresh on a 1Hz tick).
//! - [`poll_to_state`] turns a poll result into the next `TrayState`,
//!   used by the runtime thread.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ActiveSnapshot;
    use chrono::{DateTime, Local, TimeZone, Utc};

    fn snap(task_id: i64, title: &str, sc: Option<i64>, started: DateTime<Utc>) -> ActiveSnapshot {
        ActiveSnapshot {
            task_id,
            task_title: title.into(),
            sc_external_id: sc,
            started_at: started,
        }
    }

    fn at_local(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(y, m, d, h, min, 0).single().unwrap()
    }

    fn at_utc(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
    }

    #[test]
    fn tooltip_idle_is_buckland_idle() {
        assert_eq!(tooltip(&TrayState::Idle, at_local(2026, 4, 22, 9, 0)), "Buckland: idle");
    }

    #[test]
    fn tooltip_no_database_is_friendly() {
        assert_eq!(
            tooltip(&TrayState::NoDatabase, at_local(2026, 4, 22, 9, 0)),
            "Buckland: no database yet"
        );
    }

    #[test]
    fn tooltip_error_includes_reason() {
        let s = TrayState::Error("permission denied".into());
        assert_eq!(
            tooltip(&s, at_local(2026, 4, 22, 9, 0)),
            "Buckland: cannot read database — permission denied"
        );
    }

    #[test]
    fn tooltip_error_truncates_long_reason() {
        let long = "x".repeat(200);
        let s = TrayState::Error(long);
        let t = tooltip(&s, at_local(2026, 4, 22, 9, 0));
        // 60-char cap on the reason, plus the prefix.
        assert!(t.len() <= "Buckland: cannot read database — ".len() + 60);
        assert!(t.starts_with("Buckland: cannot read database — "));
    }

    #[test]
    fn tooltip_active_with_sc_shows_sc_prefix() {
        // Started at 09:15 local; "now" is 10:38:45 local => 1h23m45s elapsed.
        let started_local = at_local(2026, 4, 22, 9, 15);
        let now_local = at_local(2026, 4, 22, 10, 38) + chrono::Duration::seconds(45);
        let s = TrayState::Active(snap(1, "Fix login flow", Some(123), started_local.with_timezone(&Utc)));
        assert_eq!(
            tooltip(&s, now_local),
            "SC-123 Fix login flow — 01:23:45 (started 09:15 local)"
        );
    }

    #[test]
    fn tooltip_active_without_sc_shows_hash_prefix() {
        let started_local = at_local(2026, 4, 22, 14, 2);
        let now_local = at_local(2026, 4, 22, 14, 14) + chrono::Duration::seconds(3);
        let s = TrayState::Active(snap(7, "Refactor imports", None, started_local.with_timezone(&Utc)));
        assert_eq!(
            tooltip(&s, now_local),
            "#7 Refactor imports — 00:12:03 (started 14:02)"
        );
    }

    #[test]
    fn tooltip_active_clamps_negative_elapsed_to_zero() {
        // "Now" is before "started" — clock drift; render 00:00:00 not a
        // negative number.
        let started_local = at_local(2026, 4, 22, 10, 0);
        let now_local = at_local(2026, 4, 22, 9, 30);
        let s = TrayState::Active(snap(1, "T", None, started_local.with_timezone(&Utc)));
        assert!(tooltip(&s, now_local).contains("00:00:00"));
    }

    #[test]
    fn icon_name_maps_each_state() {
        assert_eq!(icon_name(&TrayState::Idle), crate::tray::assets::ICON_NAME_IDLE);
        assert_eq!(
            icon_name(&TrayState::Active(snap(1, "x", None, at_utc(2026, 4, 22, 9, 0)))),
            crate::tray::assets::ICON_NAME_RUNNING
        );
        assert_eq!(icon_name(&TrayState::NoDatabase), crate::tray::assets::ICON_NAME_IDLE);
        assert_eq!(
            icon_name(&TrayState::Error("x".into())),
            crate::tray::assets::ICON_NAME_ERROR
        );
    }

    #[test]
    fn transition_returns_false_for_same_class_and_same_task() {
        let started = at_utc(2026, 4, 22, 9, 0);
        let a = TrayState::Active(snap(1, "x", None, started));
        let b = TrayState::Active(snap(1, "x", None, started));
        assert!(!transition(&a, &b));
        assert!(!transition(&TrayState::Idle, &TrayState::Idle));
    }

    #[test]
    fn transition_returns_true_when_active_task_id_changes() {
        let started = at_utc(2026, 4, 22, 9, 0);
        let a = TrayState::Active(snap(1, "x", None, started));
        let b = TrayState::Active(snap(2, "y", None, started));
        assert!(transition(&a, &b));
    }

    #[test]
    fn transition_returns_true_when_class_changes() {
        let started = at_utc(2026, 4, 22, 9, 0);
        let a = TrayState::Idle;
        let b = TrayState::Active(snap(1, "x", None, started));
        assert!(transition(&a, &b));
        assert!(transition(&b, &a));
        assert!(transition(&TrayState::Idle, &TrayState::Error("e".into())));
    }

    #[test]
    fn transition_treats_started_at_change_on_same_task_as_a_transition() {
        // Re-starting the same task: stop+start atomically gives a new
        // started_at on the same task_id. Treat as a transition so the
        // tooltip's elapsed clock resets visibly.
        let a = TrayState::Active(snap(1, "x", None, at_utc(2026, 4, 22, 9, 0)));
        let b = TrayState::Active(snap(1, "x", None, at_utc(2026, 4, 22, 10, 0)));
        assert!(transition(&a, &b));
    }

    #[test]
    fn poll_to_state_missing_db_yields_no_database() {
        assert_eq!(poll_to_state(Err(()), true), TrayState::NoDatabase);
    }

    #[test]
    fn poll_to_state_ok_none_yields_idle() {
        assert_eq!(poll_to_state(Ok(None), false), TrayState::Idle);
    }

    #[test]
    fn poll_to_state_ok_some_yields_active() {
        let s = snap(1, "x", None, at_utc(2026, 4, 22, 9, 0));
        let got = poll_to_state(Ok(Some(s.clone())), false);
        assert_eq!(got, TrayState::Active(s));
    }
}
```

Run: `cargo test --features tray --lib tray::state -- --nocapture`
Expected: FAIL — every symbol referenced in the tests is undefined.

- [ ] **Step 2 (green): write the implementation**

Replace `src/tray/state.rs` with the full implementation (tests stay at the bottom):

```rust
//! Pure logic for the tray surface. No I/O, no glib, no ksni.

use crate::domain::ActiveSnapshot;
use crate::tray::assets;
use chrono::{DateTime, Duration, Local};

/// The four states the tray can be in. The runtime thread is responsible
/// for transitioning between them; everything visible (icon, tooltip)
/// is a pure function of the current state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayState {
    /// Database file is missing on disk. The user has not run `bl`
    /// yet. Render with the idle icon and a friendly tooltip.
    NoDatabase,
    /// Database is reachable but no timer is running.
    Idle,
    /// A timer is running. The snapshot is the joined view of the
    /// active row.
    Active(ActiveSnapshot),
    /// We tried to read the database and failed. The string is a short
    /// human-readable reason (truncated by [`tooltip`]).
    Error(String),
}

const ERROR_REASON_MAX: usize = 60;

/// Render the SNI tooltip for `state` at the given local time.
///
/// `now_local` is the current wall clock in the user's timezone. For
/// `Active` we compute elapsed seconds from `started_at` (clamped to
/// zero on clock drift). For everything else `now_local` is unused.
pub fn tooltip(state: &TrayState, now_local: DateTime<Local>) -> String {
    match state {
        TrayState::Idle => "Buckland: idle".to_string(),
        TrayState::NoDatabase => "Buckland: no database yet".to_string(),
        TrayState::Error(reason) => {
            let r: String = reason.chars().take(ERROR_REASON_MAX).collect();
            format!("Buckland: cannot read database — {r}")
        }
        TrayState::Active(snap) => format_active_tooltip(snap, now_local),
    }
}

fn format_active_tooltip(snap: &ActiveSnapshot, now_local: DateTime<Local>) -> String {
    let started_local = snap.started_at.with_timezone(&Local);
    let elapsed = (now_local - started_local).max(Duration::zero());
    let total_secs = elapsed.num_seconds();
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    let elapsed_str = format!("{h:02}:{m:02}:{s:02}");
    let started_str = started_local.format("%H:%M").to_string();
    match snap.sc_external_id {
        Some(ext) => format!(
            "SC-{ext} {title} — {elapsed_str} (started {started_str} local)",
            title = snap.task_title,
        ),
        None => format!(
            "#{id} {title} — {elapsed_str} (started {started_str})",
            id = snap.task_id,
            title = snap.task_title,
        ),
    }
}

/// Pick the freedesktop icon-theme name to expose for the given state.
pub fn icon_name(state: &TrayState) -> &'static str {
    match state {
        // NoDatabase is intentionally idle-shaped: nothing is wrong, the
        // user just hasn't started using `bl` yet.
        TrayState::Idle | TrayState::NoDatabase => assets::ICON_NAME_IDLE,
        TrayState::Active(_) => assets::ICON_NAME_RUNNING,
        TrayState::Error(_) => assets::ICON_NAME_ERROR,
    }
}

/// Decide whether the icon should swap between two consecutive states.
///
/// A 1Hz tooltip refresh on the same active task is **not** a transition.
/// Returns true when:
///
/// - the variant changed (e.g. Idle → Active, Active → Error), OR
/// - both are `Active` but the task id or `started_at` differ (i.e. the
///   user stopped and started a different task, or restarted the same
///   one — visually a fresh elapsed clock should reset).
pub fn transition(prev: &TrayState, next: &TrayState) -> bool {
    match (prev, next) {
        (TrayState::Active(a), TrayState::Active(b)) => {
            a.task_id != b.task_id || a.started_at != b.started_at
        }
        (a, b) => std::mem::discriminant(a) != std::mem::discriminant(b),
    }
}

/// Convert a poll result into the next [`TrayState`].
///
/// `result` is the outcome of `Repo::active_snapshot()` reduced to
/// `Result<Option<ActiveSnapshot>, ()>` (callers map their error type
/// down to `()` because the reason is captured in `error_reason`).
/// `database_missing` is true iff the most recent open attempt produced
/// `RepoError::DatabaseMissing`.
///
/// We accept a pre-reduced `Result<_, ()>` here so this stays pure
/// (no dependency on `RepoError`); the runtime thread does the mapping.
pub fn poll_to_state(
    result: Result<Option<ActiveSnapshot>, ()>,
    database_missing: bool,
) -> TrayState {
    if database_missing {
        return TrayState::NoDatabase;
    }
    match result {
        Ok(None) => TrayState::Idle,
        Ok(Some(snap)) => TrayState::Active(snap),
        Err(()) => TrayState::Error("read failed".into()),
    }
}

/// Variant of [`poll_to_state`] that lets the caller pass the actual
/// error reason. Equivalent for the success case.
pub fn poll_to_state_with_reason(
    result: Result<Option<ActiveSnapshot>, String>,
    database_missing: bool,
) -> TrayState {
    if database_missing {
        return TrayState::NoDatabase;
    }
    match result {
        Ok(None) => TrayState::Idle,
        Ok(Some(snap)) => TrayState::Active(snap),
        Err(reason) => TrayState::Error(reason),
    }
}

// `mod tests` from Step 1 stays at the bottom.
```

Append a small extra test for `poll_to_state_with_reason` inside `mod tests`:

```rust
#[test]
fn poll_to_state_with_reason_propagates_message() {
    let s = poll_to_state_with_reason(Err::<Option<ActiveSnapshot>, _>("locked".into()), false);
    assert_eq!(s, TrayState::Error("locked".into()));
}
```

- [ ] **Step 3 (green): register the module in `tray/mod.rs`**

Edit `src/tray/mod.rs`:

```rust
//! The `bl-tray` surface — a passive StatusNotifierItem icon that
//! polls the SQLite file in read-only mode and surfaces the active
//! timer's state via tooltip + icon swap.

pub mod assets;
pub mod state;
// runtime is added by Task E4.
```

- [ ] **Step 4 (verify)**

Run:

```bash
cargo test --features tray --lib tray::state
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

Expected: every test green, clippy clean, fmt clean.

- [ ] **Step 5: Commit**

```text
feat(tray): pure TrayState with tooltip and icon mapping
```

```bash
git add src/tray/mod.rs src/tray/state.rs
git commit -m "feat(tray): pure TrayState with tooltip and icon mapping"
```

---

## E4 — Tray runtime: ksni service + polling thread + glib 1Hz tick

**Goal:** Glue the pure logic from E3 onto a real ksni `Tray` impl, install the embedded SVGs as theme icons under `~/.local/share/icons/hicolor/scalable/apps/`, run a polling thread that hits the read-only repo every `poll_seconds`, and tick the tooltip locally at 1Hz via glib so the elapsed clock advances between poll cycles. Replace `src/bin/bl-tray.rs`'s placeholder body so the binary actually does something.

**Files:**
- Create: `src/tray/runtime.rs`
- Modify: `src/tray/mod.rs`
- Modify: `src/bin/bl-tray.rs`

- [ ] **Step 1 (red): write the failing tests for the icon installer + the poll-result mapper**

Create `src/tray/runtime.rs` with the test stub first (we test only the pure helpers; the ksni service itself is exercised by the manual smoke test in Task E8 per the spec):

```rust
//! Runtime glue: ksni service, polling thread, glib 1Hz tick.
//!
//! The pure parts (icon installation, poll-result mapping) are unit-tested
//! here. The DBus surface, glib loop, and ksni service are exercised by
//! the manual smoke test (`docs/smoke-tests/phase-e-tray.md`).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ActiveSnapshot;
    use crate::storage::repo::RepoError;
    use chrono::{TimeZone, Utc};
    use tempfile::TempDir;

    fn snap(id: i64, started_h: u32) -> ActiveSnapshot {
        ActiveSnapshot {
            task_id: id,
            task_title: format!("task {id}"),
            sc_external_id: None,
            started_at: Utc.with_ymd_and_hms(2026, 4, 22, started_h, 0, 0).unwrap(),
        }
    }

    #[test]
    fn install_theme_icons_writes_three_svg_files_into_target_dir() {
        let dir = TempDir::new().unwrap();
        install_theme_icons_at(dir.path()).unwrap();
        let apps = dir.path().join("hicolor/scalable/apps");
        assert!(apps.join("buckland-tray-idle.svg").exists());
        assert!(apps.join("buckland-tray-running.svg").exists());
        assert!(apps.join("buckland-tray-error.svg").exists());
    }

    #[test]
    fn install_theme_icons_is_idempotent_when_bytes_match() {
        let dir = TempDir::new().unwrap();
        install_theme_icons_at(dir.path()).unwrap();
        let path = dir.path().join("hicolor/scalable/apps/buckland-tray-idle.svg");
        let mtime1 = std::fs::metadata(&path).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        // Second call: bytes are identical, file mtime should not change.
        install_theme_icons_at(dir.path()).unwrap();
        let mtime2 = std::fs::metadata(&path).unwrap().modified().unwrap();
        assert_eq!(mtime1, mtime2, "idempotent install must not rewrite when bytes match");
    }

    #[test]
    fn install_theme_icons_overwrites_when_existing_bytes_differ() {
        let dir = TempDir::new().unwrap();
        let apps = dir.path().join("hicolor/scalable/apps");
        std::fs::create_dir_all(&apps).unwrap();
        std::fs::write(apps.join("buckland-tray-idle.svg"), b"<svg>old</svg>").unwrap();
        install_theme_icons_at(dir.path()).unwrap();
        let written = std::fs::read(apps.join("buckland-tray-idle.svg")).unwrap();
        assert_eq!(written, crate::tray::assets::TRAY_IDLE_SVG);
    }

    #[test]
    fn map_poll_result_database_missing_yields_no_database() {
        use std::path::PathBuf;
        let s = map_poll_result(Err::<Option<ActiveSnapshot>, _>(RepoError::DatabaseMissing(
            PathBuf::from("/nope"),
        )));
        assert_eq!(s, super::super::state::TrayState::NoDatabase);
    }

    #[test]
    fn map_poll_result_other_error_yields_error_state() {
        let inner = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(5),
            Some("database is locked".into()),
        );
        let s = map_poll_result(Err::<Option<ActiveSnapshot>, _>(RepoError::Sqlite(inner)));
        match s {
            super::super::state::TrayState::Error(reason) => {
                assert!(reason.to_lowercase().contains("locked") || !reason.is_empty());
            }
            other => panic!("expected Error state, got {other:?}"),
        }
    }

    #[test]
    fn map_poll_result_ok_some_yields_active() {
        let snap0 = snap(1, 9);
        let s = map_poll_result(Ok::<_, RepoError>(Some(snap0.clone())));
        assert_eq!(s, super::super::state::TrayState::Active(snap0));
    }

    #[test]
    fn map_poll_result_ok_none_yields_idle() {
        assert_eq!(
            map_poll_result(Ok::<_, RepoError>(None)),
            super::super::state::TrayState::Idle
        );
    }

    #[test]
    fn tray_runtime_config_has_sensible_default_poll_seconds() {
        let cfg = TrayRuntimeConfig::for_path(std::path::PathBuf::from("/tmp/x"));
        assert_eq!(cfg.poll_seconds, 30);
    }
}
```

Run: `cargo test --features tray --lib tray::runtime -- --nocapture`
Expected: FAIL — `install_theme_icons_at`, `map_poll_result`, `TrayRuntimeConfig`, `TrayRuntimeConfig::for_path` are not yet defined.

- [ ] **Step 2 (green): write the runtime implementation**

Replace `src/tray/runtime.rs` with the full implementation. The file is split into three logical sections (config, helpers, service) and stays under the 300-LOC budget — the `tests` module above adds ~60 lines, the implementation below ~210.

```rust
//! Runtime glue: ksni service, polling thread, glib 1Hz tick.

use crate::domain::ActiveSnapshot;
use crate::storage::repo::{RepoError, RepoResult};
use crate::storage::{Repo, SqliteRepo};
use crate::tray::{
    assets,
    state::{icon_name, tooltip, transition, TrayState},
};
use anyhow::Context;
use chrono::Local;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// --- Config ----------------------------------------------------------------

/// Inputs to `run`. Keeps the parameter count in check (CLAUDE.md says
/// ≤ 4 params per function; bundling these lets the public surface stay
/// at one argument).
#[derive(Debug, Clone)]
pub struct TrayRuntimeConfig {
    /// Path to the SQLite file. Read-only access only.
    pub db_path: PathBuf,
    /// Seconds between polls. Tooltip ticks at 1Hz independently.
    pub poll_seconds: u64,
}

impl TrayRuntimeConfig {
    pub fn for_path(db_path: PathBuf) -> Self {
        Self {
            db_path,
            poll_seconds: 30,
        }
    }
}

// --- Pure helpers (tested) -------------------------------------------------

/// Reduce a `RepoResult<Option<ActiveSnapshot>>` to a `TrayState`.
///
/// `RepoError::DatabaseMissing` becomes `NoDatabase` (intentional —
/// the user just hasn't run `bl` yet, not an alarming error). Every
/// other error becomes `Error(reason)`.
pub(crate) fn map_poll_result(result: RepoResult<Option<ActiveSnapshot>>) -> TrayState {
    match result {
        Ok(None) => TrayState::Idle,
        Ok(Some(snap)) => TrayState::Active(snap),
        Err(RepoError::DatabaseMissing(_)) => TrayState::NoDatabase,
        Err(other) => TrayState::Error(short_reason(&other)),
    }
}

fn short_reason(err: &RepoError) -> String {
    match err {
        RepoError::Sqlite(e) => e.to_string(),
        other => other.to_string(),
    }
}

/// Install the embedded SVGs into a hicolor theme tree rooted at
/// `target_root` (typically `~/.local/share/icons`). Idempotent: if
/// the destination file's bytes already match the embedded constant,
/// the file is not rewritten. Used at startup; failure is logged and
/// ignored — the tray falls back to the host's default icon.
pub(crate) fn install_theme_icons_at(target_root: &Path) -> std::io::Result<()> {
    let apps = target_root.join("hicolor/scalable/apps");
    std::fs::create_dir_all(&apps)?;
    write_if_changed(&apps.join("buckland-tray-idle.svg"), assets::TRAY_IDLE_SVG)?;
    write_if_changed(
        &apps.join("buckland-tray-running.svg"),
        assets::TRAY_RUNNING_SVG,
    )?;
    write_if_changed(&apps.join("buckland-tray-error.svg"), assets::TRAY_ERROR_SVG)?;
    Ok(())
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Ok(existing) = std::fs::read(path) {
        if existing == bytes {
            return Ok(());
        }
    }
    std::fs::write(path, bytes)
}

fn data_icons_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("icons")
}

// --- ksni service ----------------------------------------------------------

/// The shared mutex our three actors (poll thread, glib tick, ksni
/// service) hand off through. `Arc<Mutex<TrayState>>` is enough because
/// no actor holds the lock long: the poll thread reads the DB outside
/// the lock and only swaps the state atomically.
type SharedState = Arc<Mutex<TrayState>>;

struct BucklandTray {
    state: SharedState,
}

#[cfg(feature = "tray")]
impl ksni::Tray for BucklandTray {
    fn id(&self) -> String {
        "buckland-tray".into()
    }

    fn title(&self) -> String {
        "Buckland".into()
    }

    fn icon_name(&self) -> String {
        let s = self.state.lock().expect("tray state poisoned");
        icon_name(&s).to_string()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        let s = self.state.lock().expect("tray state poisoned");
        let title = tooltip(&s, Local::now());
        ksni::ToolTip {
            icon_name: icon_name(&s).to_string(),
            icon_pixmap: vec![],
            title,
            description: String::new(),
        }
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;
        vec![StandardItem {
            label: "Quit".into(),
            activate: Box::new(|_: &mut Self| std::process::exit(0)),
            ..Default::default()
        }
        .into()]
    }
}

// --- Entry point -----------------------------------------------------------

/// Block the calling thread running the tray loop. Returns `Ok(())`
/// only when glib's main loop ends (e.g. the user picks "Quit", which
/// calls `std::process::exit(0)`). Errors are returned for setup
/// failures (DBus session unreachable, glib init).
#[cfg(feature = "tray")]
pub fn run(config: TrayRuntimeConfig) -> anyhow::Result<()> {
    // Best-effort: install the icons. If this fails (read-only home,
    // for instance), the host falls back to its default icon — not
    // worth aborting startup.
    let _ = install_theme_icons_at(&data_icons_dir());

    let state: SharedState = Arc::new(Mutex::new(TrayState::Idle));
    let tray = BucklandTray {
        state: Arc::clone(&state),
    };
    let service = ksni::TrayService::new(tray);
    let handle = service.handle();
    service.spawn();

    spawn_polling_thread(Arc::clone(&state), handle.clone(), config.clone());

    let main_loop = glib::MainLoop::new(None, false);
    let tick_state = Arc::clone(&state);
    let tick_handle = handle.clone();
    glib::timeout_add_local(Duration::from_secs(1), move || {
        // 1Hz tooltip refresh. State doesn't change, but the elapsed
        // clock is rendered from `Local::now()`, so `update()` is what
        // tells SNI hosts "redraw the tooltip text".
        let _ = &tick_state; // keep the Arc alive in the closure
        tick_handle.update(|_t: &mut BucklandTray| {});
        glib::ControlFlow::Continue
    });

    main_loop.run();
    Ok(())
}

#[cfg(feature = "tray")]
fn spawn_polling_thread(
    state: SharedState,
    handle: ksni::Handle<BucklandTray>,
    config: TrayRuntimeConfig,
) {
    thread::spawn(move || {
        let interval = Duration::from_secs(config.poll_seconds.max(1));
        loop {
            let next = poll_once(&config.db_path);
            let changed = swap_state(&state, next);
            if changed {
                handle.update(|_t: &mut BucklandTray| {});
            }
            thread::sleep(interval);
        }
    });
}

#[cfg(feature = "tray")]
fn poll_once(db_path: &Path) -> TrayState {
    let result = SqliteRepo::open_read_only(db_path).and_then(|repo| repo.active_snapshot());
    map_poll_result(result)
}

fn swap_state(state: &SharedState, next: TrayState) -> bool {
    let mut guard = state.lock().expect("tray state poisoned");
    let changed = transition(&guard, &next);
    *guard = next;
    changed
}

// `mod tests` from Step 1 stays at the bottom.
```

- [ ] **Step 3 (green): replace `bl-tray` placeholder body**

Edit `src/bin/bl-tray.rs`:

```rust
//! `bl-tray` — the StatusNotifierItem tray icon.
//!
//! Reads the SQLite file in read-only mode, polls every
//! `Config.tray.poll_seconds` (default 30s), and ticks the tooltip
//! locally at 1Hz so the elapsed clock advances between polls.

use anyhow::Context;
use buckland::config;
use buckland::tray::runtime::{run, TrayRuntimeConfig};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let cfg = config::load(&config::config_path())
        .context("loading config from $XDG_CONFIG_HOME/buckland/config.toml")?;
    let db_path = resolve_db_path();
    let runtime_config = TrayRuntimeConfig {
        db_path,
        poll_seconds: cfg.tray.poll_seconds,
    };
    run(runtime_config)
}

fn resolve_db_path() -> PathBuf {
    match std::env::var("BUCKLAND_HOME") {
        Ok(home) if !home.trim().is_empty() => PathBuf::from(home.trim()).join("buckland.db"),
        _ => config::db_path(),
    }
}
```

- [ ] **Step 4 (green): register `runtime` in `tray/mod.rs`**

Edit `src/tray/mod.rs`:

```rust
//! The `bl-tray` surface — a passive StatusNotifierItem icon that
//! polls the SQLite file in read-only mode and surfaces the active
//! timer's state via tooltip + icon swap.

pub mod assets;
pub mod runtime;
pub mod state;

pub use runtime::{run, TrayRuntimeConfig};
```

- [ ] **Step 5 (verify)**

Run:

```bash
cargo build --features tray
cargo build --bin bl-tray --features tray
cargo test --features tray --lib tray::runtime
cargo test --features tray --lib   # the rest stays green
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

Expected: every command green. The ksni `Tray` impl compiles even though we don't unit-test it directly — the runtime tests cover the pure helpers.

- [ ] **Step 6 (manual smoke — optional at this point)**

If `libdbus-1-dev` and a tray-aware desktop are at hand:

```bash
cargo run --features tray --bin bl-tray
```

Expected: a clock-shaped icon appears in the tray. Right-click → Quit. (Full smoke is in Task E8; this is just an early sanity check.)

- [ ] **Step 7: Commit**

```text
feat(tray): ksni service with polling thread and glib 1Hz tick
```

```bash
git add src/tray/mod.rs src/tray/runtime.rs src/bin/bl-tray.rs
git commit -m "feat(tray): ksni service with polling thread and glib 1Hz tick"
```

---

## E5 — `bl tray` subcommand

**Goal:** Expose the same runtime as a CLI subcommand so users on `cargo install buckland --no-default-features` can still launch the tray when they later install the feature, and so the spec's `bl tray` synonym for `bl-tray` works as documented.

**Files:**
- Modify: `src/cli/args.rs`
- Modify: `src/cli/mod.rs`
- Create: `src/cli/commands/tray.rs`
- Modify: `src/cli/commands/mod.rs`
- Create: `tests/cli_tray.rs`

- [ ] **Step 1 (red): write the failing CLI smoke test**

Create `tests/cli_tray.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;

#[test]
#[cfg(feature = "tray")]
fn tray_subcommand_is_listed_in_help() {
    Command::cargo_bin("bl")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("tray"));
}

#[test]
#[cfg(feature = "tray")]
fn tray_help_describes_command() {
    Command::cargo_bin("bl")
        .unwrap()
        .args(["tray", "--help"])
        .assert()
        .success()
        .stdout(contains("tray icon"));
}

#[test]
#[cfg(not(feature = "tray"))]
fn tray_subcommand_absent_when_feature_disabled() {
    // No-op assertion — when tray is off, the subcommand isn't even
    // compiled in. We only assert the help still works.
    Command::cargo_bin("bl")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
}
```

Run: `cargo test --features tray --test cli_tray -- --nocapture`
Expected: FAIL — `tray` is not yet a subcommand.

- [ ] **Step 2 (green): add the `Tray` variant to `Commands` (feature-gated)**

Edit `src/cli/args.rs`. After the `Tui` variant, add:

```rust
    /// Run the tray icon process. Same loop as the `bl-tray` binary.
    #[cfg(feature = "tray")]
    Tray,
```

The full enum tail after the edit reads:

```rust
    /// Open the TUI. Same as running `bl` with no subcommand.
    Tui,
    /// Run the tray icon process. Same loop as the `bl-tray` binary.
    #[cfg(feature = "tray")]
    Tray,
}
```

- [ ] **Step 3 (green): create `src/cli/commands/tray.rs`**

Create `src/cli/commands/tray.rs`:

```rust
//! `bl tray` — runs the tray-icon loop in this process.
//!
//! Equivalent to invoking the `bl-tray` binary; the subcommand exists
//! so users who installed `bl` without that binary on PATH can still
//! launch the tray with one command. Reads the same config and DB path
//! as the CLI surface (no special context).

use crate::cli::context::Context;

#[cfg(feature = "tray")]
pub fn tray(ctx: &Context) -> anyhow::Result<i32> {
    use crate::config;
    let cfg = config::load(&config::config_path())?;
    let runtime = crate::tray::runtime::TrayRuntimeConfig {
        db_path: ctx.db_path.clone(),
        poll_seconds: cfg.tray.poll_seconds,
    };
    crate::tray::runtime::run(runtime)?;
    Ok(0)
}

#[cfg(not(feature = "tray"))]
pub fn tray(_ctx: &Context) -> anyhow::Result<i32> {
    // Unreachable in practice — clap won't surface the variant when
    // the feature is off — but a defensive stub makes the dispatch in
    // `cli/mod.rs` compile under both feature configurations.
    eprintln!("`bl tray` requires the `tray` feature. Reinstall with `cargo install buckland`.");
    Ok(2)
}
```

Edit `src/cli/commands/mod.rs` to register the new submodule and re-export the function. Locate the `pub mod` lines and add:

```rust
pub mod tray;
pub use tray::tray;
```

(Place the lines alphabetically within the existing list.)

- [ ] **Step 4 (green): wire dispatch in `src/cli/mod.rs`**

Edit `src/cli/mod.rs`. Add a new arm to the match in `run`:

```rust
        Commands::Tui => crate::tui::run(ctx),
        #[cfg(feature = "tray")]
        Commands::Tray => commands::tray(&ctx),
    }
}
```

(The `#[cfg]` on the match arm is required because the variant itself is gated.)

- [ ] **Step 5 (verify)**

Run:

```bash
cargo test --features tray --test cli_tray
cargo test --no-default-features --test cli_tray   # exercises the absent-feature path
cargo test --features tray --test cli_smoke        # regression: tui dispatch still works
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --all-targets --no-default-features -- -D warnings
cargo fmt --all --check
```

Expected: every command green. The clippy passes confirm that the feature gate is wired symmetrically.

- [ ] **Step 6: Commit**

```text
feat(cli): bl tray subcommand wired to tray::runtime::run
```

```bash
git add src/cli/args.rs src/cli/mod.rs src/cli/commands/ tests/cli_tray.rs
git commit -m "feat(cli): bl tray subcommand wired to tray::runtime::run"
```

---

## E6 — Clipboard helper + `bl report --copy` + TUI Report `c` reuse

**Goal:** Promote the Phase-D `tui::clipboard` module to a top-level `crate::clipboard` so the CLI can reach it without a TUI dependency. Add `--copy` / `-c` to `bl report`. Make the existing TUI Report `c` shortcut delegate to the new top-level module via a one-line re-export — no behavioral change in the TUI.

**Files:**
- Create: `src/clipboard.rs`
- Modify: `src/lib.rs`
- Modify: `src/tui/clipboard.rs`
- Modify: `src/tui/mod.rs` (only if the `clipboard` mod line needs reordering — likely not)
- Modify: `src/cli/args.rs`
- Modify: `src/cli/commands/report.rs`
- Create: `tests/cli_report_copy.rs`

- [ ] **Step 1 (red): write the failing unit test for `detect_tool`**

Create the new top-level `src/clipboard.rs` with the test stub first (envelope only — empty env detection):

```rust
//! Cross-desktop clipboard helper. Shells out to `wl-copy` (Wayland)
//! or `xclip -selection clipboard` (X11) — no clipboard crate dep.
//!
//! Detection is env-driven, matching the freedesktop convention:
//!
//! - `$WAYLAND_DISPLAY` non-empty → `wl-copy`
//! - else `$DISPLAY` non-empty     → `xclip -selection clipboard`
//! - else                          → `ClipboardError::NoServer`
//!
//! If detection picks a tool but the binary is missing on `PATH`, we
//! return `ClipboardError::ToolMissing(name)`. Callers use this to
//! render a footer error like "Copy failed: wl-copy not found".

#[cfg(test)]
mod tests {
    use super::*;

    fn env_for<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k: &str| pairs.iter().find(|(name, _)| *name == k).map(|(_, v)| (*v).to_string())
    }

    #[test]
    fn detect_picks_wl_copy_when_wayland_display_set() {
        let env = env_for(&[("WAYLAND_DISPLAY", "wayland-0")]);
        assert_eq!(detect_tool(&env), Ok(Tool::WlCopy));
    }

    #[test]
    fn detect_falls_back_to_xclip_when_only_display_set() {
        let env = env_for(&[("DISPLAY", ":0")]);
        assert_eq!(detect_tool(&env), Ok(Tool::Xclip));
    }

    #[test]
    fn detect_prefers_wayland_when_both_are_set() {
        let env = env_for(&[("WAYLAND_DISPLAY", "wayland-0"), ("DISPLAY", ":0")]);
        assert_eq!(detect_tool(&env), Ok(Tool::WlCopy));
    }

    #[test]
    fn detect_fails_when_neither_env_var_is_set() {
        let env = env_for(&[]);
        assert_eq!(detect_tool(&env), Err(ClipboardError::NoServer));
    }

    #[test]
    fn detect_treats_empty_env_var_as_unset() {
        let env = env_for(&[("WAYLAND_DISPLAY", ""), ("DISPLAY", "")]);
        assert_eq!(detect_tool(&env), Err(ClipboardError::NoServer));
    }
}
```

Run: `cargo test --lib clipboard -- --nocapture`
Expected: FAIL — every symbol referenced by the tests is undefined; the module isn't declared yet.

- [ ] **Step 2 (green): write `src/clipboard.rs`**

Replace `src/clipboard.rs` with the full implementation:

```rust
//! Cross-desktop clipboard helper. Shells out to `wl-copy` (Wayland)
//! or `xclip -selection clipboard` (X11) — no clipboard crate dep.

use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    WlCopy,
    Xclip,
}

impl Tool {
    pub fn binary(self) -> &'static str {
        match self {
            Tool::WlCopy => "wl-copy",
            Tool::Xclip => "xclip",
        }
    }

    fn args(self) -> &'static [&'static str] {
        match self {
            Tool::WlCopy => &[],
            Tool::Xclip => &["-selection", "clipboard"],
        }
    }
}

impl std::fmt::Display for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.binary())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ClipboardError {
    #[error("no display server detected (WAYLAND_DISPLAY and DISPLAY both unset)")]
    NoServer,
    #[error("{0} not found on PATH")]
    ToolMissing(&'static str),
    #[error("{tool} exited with code {code}")]
    ToolFailed { tool: &'static str, code: i32 },
    #[error("io error talking to clipboard tool: {0}")]
    Io(String),
}

impl From<std::io::Error> for ClipboardError {
    fn from(e: std::io::Error) -> Self {
        ClipboardError::Io(e.to_string())
    }
}

/// Pick the clipboard tool from the env. `read_env` is injected so
/// tests don't need to mutate the real process env.
pub fn detect_tool(read_env: &dyn Fn(&str) -> Option<String>) -> Result<Tool, ClipboardError> {
    let wayland = read_env("WAYLAND_DISPLAY").unwrap_or_default();
    if !wayland.is_empty() {
        return Ok(Tool::WlCopy);
    }
    let display = read_env("DISPLAY").unwrap_or_default();
    if !display.is_empty() {
        return Ok(Tool::Xclip);
    }
    Err(ClipboardError::NoServer)
}

/// Copy `text` to the system clipboard. Returns the tool that was used
/// on success so callers can render "Copied via wl-copy" in the UI.
pub fn copy(text: &str) -> Result<Tool, ClipboardError> {
    let env = |k: &str| std::env::var(k).ok();
    let tool = detect_tool(&env)?;
    spawn_and_pipe(tool, text)?;
    Ok(tool)
}

fn spawn_and_pipe(tool: Tool, text: &str) -> Result<(), ClipboardError> {
    let mut cmd = Command::new(tool.binary());
    cmd.args(tool.args())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(ClipboardError::ToolMissing(tool.binary()));
        }
        Err(e) => return Err(e.into()),
    };
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes())?;
    }
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(ClipboardError::ToolFailed {
            tool: tool.binary(),
            code: status.code().unwrap_or(-1),
        })
    }
}

// `mod tests` from Step 1 stays at the bottom.
```

- [ ] **Step 3 (green): wire the module in `src/lib.rs`**

Edit `src/lib.rs`. After the edit:

```rust
//! Buckland — personal time tracker core library.

pub mod cli;
pub mod clipboard;
pub mod config;
pub mod domain;
pub mod shortcut;
pub mod storage;
#[cfg(feature = "tray")]
pub mod tray;
pub mod tui;
```

- [ ] **Step 4 (green): make `src/tui/clipboard.rs` a one-line re-export**

Replace `src/tui/clipboard.rs` (it currently has its own `ClipboardError`, `copy`, and `try_copy_with` from Phase D — we collapse to a re-export, preserving the public surface for `tui::screens::report`):

```rust
//! Phase D wired the TUI Report screen against `tui::clipboard::copy`.
//! Phase E promoted the helper to the top-level `crate::clipboard`
//! module so the CLI's `bl report --copy` can use it without a TUI
//! dependency. We keep the old import path alive via a re-export.

pub use crate::clipboard::{copy, ClipboardError, Tool};
```

(If the TUI Report screen used a private symbol like `try_copy_with`, audit it now: search with `cargo check --features tray` after the change. Expectation per Phase D's `report.rs:177-180` is that only `clipboard::copy` is imported; the change should be transparent.)

- [ ] **Step 5 (verify the TUI regression)**

Run:

```bash
cargo test --features tray --lib tui::screens::report
cargo test --features tray --lib tui::clipboard
```

Expected: every Phase-D TUI Report test still passes. The `tui::clipboard` re-export means `tui::clipboard::copy` resolves to `crate::clipboard::copy` exactly.

- [ ] **Step 6 (red): write the failing CLI flag test**

Add `--copy` / `-c` to `bl report`. Edit `src/cli/args.rs` — locate `Commands::Report { ... }` and add the new field after `json`:

```rust
        /// Emit a JSON object instead of a table.
        #[arg(long)]
        json: bool,
        /// Copy the one-line summary (or the JSON body, when --json is
        /// set) to the system clipboard via wl-copy or xclip.
        #[arg(long, short = 'c')]
        copy: bool,
```

The dispatch arm in `src/cli/mod.rs` needs the new field threaded through. Edit `src/cli/mod.rs`:

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
            copy,
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
                copy,
            },
        ),
```

Add `copy: bool` to `ReportArgs` in `src/cli/commands/report.rs`:

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
    pub copy: bool,
}
```

Now write the failing integration test. Create `tests/cli_report_copy.rs`:

```rust
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

    bl(&home, &path_dir).args(["add", "smoke"]).assert().success();
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

    bl(&home, &path_dir).args(["add", "smoke"]).assert().success();
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
```

Add `shell-escape = "0.1"` to `[dev-dependencies]` in `Cargo.toml`:

```toml
[dev-dependencies]
assert_cmd = "2"
mockito = "1"
predicates = "3"
shell-escape = "0.1"
tempfile = "3"
```

Run: `cargo test --features tray --test cli_report_copy -- --nocapture`
Expected: FAIL — `--copy` is wired into args/dispatch but `commands::report` does not yet branch on it.

- [ ] **Step 7 (green): implement the copy path in `commands::report`**

Edit `src/cli/commands/report.rs`. Replace the body of `report()`:

```rust
pub fn report(ctx: &mut Context, args: ReportArgs) -> anyhow::Result<i32> {
    use crate::domain::ReportBuilder;

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

    let payload = if args.json {
        serde_json::to_string_pretty(&report)?
    } else {
        format_one_liner(&report)
    };

    if args.copy {
        match crate::clipboard::copy(&payload) {
            Ok(tool) => {
                eprintln!("Copied to clipboard via {tool}");
                Ok(0)
            }
            Err(e) => {
                eprintln!("clipboard copy failed: {e}");
                Ok(1)
            }
        }
    } else if args.json {
        println!("{payload}");
        Ok(0)
    } else if report.rows.is_empty() {
        println!("No time tracked in this scope.");
        Ok(0)
    } else {
        print_table(&report);
        Ok(0)
    }
}

fn format_one_liner(report: &crate::domain::Report) -> String {
    use crate::cli::format::duration_compact;
    use crate::domain::ScopeKind;
    let scope_label = match report.scope.kind {
        ScopeKind::Today => "today",
        ScopeKind::Week => "this week",
        ScopeKind::Month => "this month",
        ScopeKind::All => "all time",
        ScopeKind::Range => "range",
    };
    let total = chrono::Duration::seconds(report.total_seconds);
    format!(
        "buckland {} — {} across {} {}",
        scope_label,
        duration_compact(total),
        report.rows.len(),
        if report.rows.len() == 1 { "row" } else { "rows" },
    )
}
```

(The empty-rows case still works: `format_one_liner` produces e.g. `"buckland today — 0h across 0 rows"`, which is what the user asked the clipboard to receive.)

- [ ] **Step 8 (verify)**

Run:

```bash
cargo test --features tray --test cli_report_copy
cargo test --features tray --test cli_report          # regression
cargo test --features tray --lib clipboard
cargo test --features tray --lib tui::screens::report # regression: TUI c key still works
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

Expected: green across the board.

- [ ] **Step 9: Commit**

```text
feat(cli): bl report --copy and shared crate::clipboard module
```

```bash
git add src/clipboard.rs src/lib.rs src/tui/clipboard.rs src/cli/args.rs src/cli/mod.rs src/cli/commands/report.rs Cargo.toml Cargo.lock tests/cli_report_copy.rs
git commit -m "feat(cli): bl report --copy and shared crate::clipboard module"
```

---

## E7 — Config `tray.poll_seconds` verified + version bump 0.5.0

**Goal:** Confirm that the existing `Config.tray.poll_seconds` field (added in Phase D) is reachable through `bl config get` / `set`, add a unit test that locks the dot-key path, refresh the README's configuration table, and bump the crate version from `0.4.0` to `0.5.0`. The version bump is its own commit so the diff stays narrow and the release tag (Phase F) is one cherry-pick away.

**Files:**
- Modify: `src/config.rs`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `README.md` (configuration section)

- [ ] **Step 1 (red): write the failing test for the dot-key resolver**

The repo already exercises `Config` round-trip in `src/config.rs`'s `mod tests`. Add a test asserting that loading a TOML file with `[tray] poll_seconds = 15` yields a `Config` with `tray.poll_seconds == 15`, and that the default value (30) is preserved when the field is absent.

Append to the `mod tests` block in `src/config.rs`:

```rust
#[test]
fn tray_poll_seconds_loads_from_toml() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "[tray]\npoll_seconds = 15\n").unwrap();
    let cfg = load(&path).unwrap();
    assert_eq!(cfg.tray.poll_seconds, 15);
}

#[test]
fn tray_poll_seconds_default_is_thirty_when_section_absent() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "[ui]\nicons = \"unicode\"\n").unwrap();
    let cfg = load(&path).unwrap();
    assert_eq!(cfg.tray.poll_seconds, 30);
}

#[test]
fn tray_poll_seconds_round_trip_through_save_and_load() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut cfg = Config::default();
    cfg.tray.poll_seconds = 12;
    save(&path, &cfg).unwrap();
    let loaded = load(&path).unwrap();
    assert_eq!(loaded.tray.poll_seconds, 12);
}

#[test]
fn tray_poll_seconds_zero_is_accepted_and_preserved() {
    // Sentinel: `0` is technically valid TOML; the runtime clamps it
    // to `>= 1` in `spawn_polling_thread` so the polling loop doesn't
    // tight-loop. Config layer just stores what the user typed.
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "[tray]\npoll_seconds = 0\n").unwrap();
    let cfg = load(&path).unwrap();
    assert_eq!(cfg.tray.poll_seconds, 0);
}
```

Run: `cargo test --lib config -- --nocapture`
Expected: PASS for the three round-trip / default tests (the field exists since Phase D), and PASS for the zero-acceptance test (TOML's `u64` accepts 0). If any FAIL, the field plumbing has regressed — investigate before continuing.

- [ ] **Step 2 (green): no implementation change required**

The existing `Config`/`TrayConfig`/`default_poll` plumbing already satisfies the test. If a test fails, adjust `src/config.rs` until green; do **not** restructure the dot-key path.

- [ ] **Step 3 (green): bump the crate version**

Edit `Cargo.toml` — change `version = "0.4.0"` to `version = "0.5.0"`. Then run `cargo build` once so `Cargo.lock` records the new version (rerun `cargo build --features tray` to refresh the locked metadata for both feature configurations).

- [ ] **Step 4 (green): refresh the README configuration table**

Edit `README.md`. The existing snippet under "Config file (optional)" already mentions `tray.poll_seconds = 30` with a `# Phase E` comment — strip the `# Phase E` qualifier now that this lands. After the edit:

```toml
[tray]
poll_seconds = 30               # how often the tray polls the SQLite file
```

If the README has a separate "Configuration keys" reference table (some Phase D revisions add one), add a row:

| Key | Type | Default | Effect |
|-----|------|---------|--------|
| `tray.poll_seconds` | `u64` | `30` | How often `bl-tray` and `bl tray` re-read the database to detect timer state changes. The tooltip elapsed clock ticks at 1Hz independently. |

If no such table exists, skip — the inline TOML comment is enough.

- [ ] **Step 5 (verify)**

Run:

```bash
cargo test --features tray
cargo build --features tray
cargo build --no-default-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

Expected: all clean. `cargo run -- --version` should now print `bl 0.5.0`.

- [ ] **Step 6: Commit**

```text
chore(config): test tray.poll_seconds plumbing and document it
```

```bash
git add src/config.rs README.md
git commit -m "chore(config): test tray.poll_seconds plumbing and document it"
```

(The `Cargo.toml` / `Cargo.lock` version bump is **not** in this commit — it goes into the final Task E8 commit alongside `CHANGELOG.md`. This keeps a single commit per "phase done" milestone, mirroring Phase D's `01e890b chore: bump version to 0.4.0 (Phase D)`. If you accidentally `git add Cargo.toml`, run `git restore --staged Cargo.toml Cargo.lock` before committing.)

---

## E8 — Smoke test + plan index + CHANGELOG + version bump (final task)

**Goal:** Capture the manual smoke checklist that gates the merge into `main`, register Phase E in the smoke-tests README and the plan-index README, write the `CHANGELOG.md` entry, and land the `0.4.0 → 0.5.0` version bump as the final commit. After this commit the branch is **ready for the user to run the smoke test**; merging into `main` only happens after the user OKs the smoke run.

**Files:**
- Create: `docs/smoke-tests/phase-e-tray.md`
- Modify: `docs/smoke-tests/README.md`
- Modify: `docs/superpowers/plans/README.md`
- Create or modify: `CHANGELOG.md`
- Modify: `Cargo.toml` (already bumped in E7 — staged here)
- Modify: `Cargo.lock`

- [ ] **Step 1: Create `docs/smoke-tests/phase-e-tray.md`**

Mirror the Portuguese-prose conventions Phase D set in `docs/smoke-tests/phase-d-tui.md` (per the user's project memory: smoke-test = merge gate, written in pt-BR with bilingual comfort, numbered scenarios, "Sinais de alerta" + "Cleanup" sections at the end).

````markdown
# Phase E — Smoke tests manuais

Roteiro pra validar Phase E (`bl-tray` + `bl tray` + `bl report --copy`)
antes de mergear `phase-e-tray` para `main`. Esse smoke é o **merge gate**:
mesmo com `cargo test` 100% verde, **não mergear** até esse arquivo passar
e o usuário dar OK explícito.

> Pré-requisito GNOME: instale a extensão "AppIndicator and KStatusNotifierItem
> Support" (https://extensions.gnome.org/extension/615/appindicator-support/).
> Sem ela, o ícone simplesmente não aparece — não é regressão do bl-tray.
> KDE Plasma, XFCE, Cinnamon e MATE já mostram o ícone out of the box.

---

## Setup

```bash
cd /home/nuuvem/Projects/study/buckland
cargo build --release --features tray

alias bl="$PWD/target/release/bl"
alias bl-tray="$PWD/target/release/bl-tray"

export BUCKLAND_HOME=/tmp/bl-phase-e-smoke
rm -rf "$BUCKLAND_HOME"
```

Confira que `wl-copy` (Wayland) ou `xclip` (X11) está instalado:

```bash
which wl-copy || which xclip || echo "FAIL: instale wl-clipboard ou xclip"
```

---

## 1. `bl-tray` sem DB — "no database yet"

- [ ] Sem nada em `$BUCKLAND_HOME` ainda, rode `bl-tray &` num terminal.
- [ ] O ícone aparece na bandeja com o desenho "outlined clock" (idle).
- [ ] Hover sobre o ícone — tooltip diz `Buckland: no database yet`.
- [ ] Process não morre, fica idle.

(Mantenha `bl-tray` rodando para os próximos cenários.)

---

## 2. Polling detecta criação do banco e timer ativo

Em outro terminal:

```bash
bl add "fix login flow"
bl add "refactor imports"
bl start 1
```

- [ ] Dentro de `tray.poll_seconds` (default 30s — espere até 35s pra
      garantir margem), o tooltip muda pra
      `#1 fix login flow — 00:00:XX (started HH:MM)`.
- [ ] O ícone troca pro "filled clock" (running).
- [ ] Hover repetido em segundos consecutivos — o `XX` avança 1, 2, 3...
      mostrando que o tick local de 1Hz está vivo (não dependente de poll).
- [ ] `bl stop`. Dentro do próximo poll, ícone volta pra outlined e
      tooltip vira `Buckland: idle`.

---

## 3. SC-prefix vs hash-prefix no tooltip

Configure um token de Shortcut válido em `$XDG_CONFIG_HOME/buckland/config.toml`
(ou pule esta seção se não tiver token agora):

```bash
bl add "linked task" --sc 4242   # cacheia a story 4242
bl start 3                        # task #3 com SC linkada
```

- [ ] Tooltip mostra `SC-4242 linked task — 00:00:XX (started HH:MM local)`.
- [ ] `bl stop`. `bl start 1` (sem SC). Tooltip volta ao formato `#1 ...`.

---

## 4. Restart da mesma task troca o ícone

- [ ] Com timer rodando (qualquer task), `bl start 2` no shell.
- [ ] Dentro do próximo poll: ícone faz visual flicker (transição
      detectada), tooltip atualiza pro task #2 com `started_at` novo.

---

## 5. Erro de leitura — ícone error

Simule um banco inacessível:

```bash
chmod 000 "$BUCKLAND_HOME/buckland.db"
```

- [ ] Próximo poll: ícone troca pro "exclamation" (error). Tooltip:
      `Buckland: cannot read database — <razão truncada>`.

```bash
chmod 600 "$BUCKLAND_HOME/buckland.db"
```

- [ ] Próximo poll: tray recupera (idle ou active conforme estado real).

---

## 6. Quit pelo menu

- [ ] Right-click no ícone do tray → menu mostra **um único item**: "Quit".
- [ ] Click em "Quit" — `bl-tray` termina cleanly (`echo $?` → 0).

---

## 7. `bl tray` (subcomando) é equivalente

- [ ] `bl tray` num terminal — comportamento idêntico ao `bl-tray`.
      Mesma nuance de poll, mesmo tooltip, mesmo menu.
- [ ] `Ctrl+C` no terminal fecha o tray (SIGINT). Sem ressaca no shell.
- [ ] `bl tray --help` exibe descrição "tray icon".

---

## 8. `bl report --copy` em Wayland

```bash
echo $WAYLAND_DISPLAY        # deve ser "wayland-0" ou similar
bl start 1; sleep 5; bl stop  # gera entry
```

- [ ] `bl report --copy` — stderr mostra `Copied to clipboard via wl-copy`.
      Stdout vazio. Exit 0.
- [ ] `wl-paste` cola exatamente uma linha tipo
      `buckland today — 5s across 1 row`.
- [ ] `bl report --copy --json` — `wl-paste` retorna um objeto JSON
      válido (começa com `{`, termina com `}`).

---

## 9. `bl report --copy` em X11

(Pule se não tiver acesso a uma sessão X11. Pode ser pulado em laptops
puramente Wayland.)

```bash
unset WAYLAND_DISPLAY
echo $DISPLAY                 # deve ser ":0" ou similar
```

- [ ] `bl report --copy` — stderr mostra `Copied via xclip`. Exit 0.
- [ ] `xclip -o -selection clipboard` retorna o one-liner.

---

## 10. `bl report --copy` sem display server

Em uma TTY pura (Ctrl+Alt+F3) ou via SSH sem `-X`:

```bash
unset WAYLAND_DISPLAY DISPLAY
bl report --copy
```

- [ ] Exit code 1. Stderr contém `clipboard copy failed: no display server detected`.
- [ ] Nada vai pro stdout.

---

## 11. TUI Report `c` (regressão Phase D)

```bash
bl tui
# r → Report → c
```

- [ ] Wayland: footer mostra `Copied via wl-copy`. `q` sai limpo.
- [ ] X11: `Copied via xclip`. Sem ambiente: `Copy failed: no display
      server detected (...)`. Sem panic.

---

## 12. `--no-default-features` build (regressão)

```bash
cargo build --no-default-features 2>&1 | tail -5
```

- [ ] Build verde. `target/debug/bl tray --help` retorna erro de
      subcomando desconhecido (clap), porque `Tray` está cfg-gated.
- [ ] `target/debug/bl-tray` **não existe** (binary requires `tray`).

---

## Sinais de alerta (qualquer um aborta o merge)

- Ícone não aparece em **nenhum** desktop testado (sem ser o caveat do GNOME).
- Tooltip mostra timestamp em UTC em vez de local.
- Tooltip avança o `XX` por menos de 1Hz ou trava.
- `bl-tray` consome CPU acima de ~1% em idle (poll mais 1Hz tick deve ser ruído).
- `bl tray` segura DB lock (rode `bl add "x"` com `bl tray` rodando — deve
  funcionar instantaneamente; se travar, há regressão de read-only mode).
- Menu do tray tem mais de um item.
- `bl report --copy` panica em qualquer ambiente em vez de exit code 1.
- `bl report --copy --json` cola uma tabela ASCII em vez de JSON.

---

## Cleanup

```bash
pkill -f bl-tray || true
rm -rf "$BUCKLAND_HOME"
unset BUCKLAND_HOME
unalias bl bl-tray
```
````

- [ ] **Step 2: Update `docs/smoke-tests/README.md`**

Append a row to the table at the bottom of the file. After the edit the table reads:

```markdown
| Fase | Arquivo | Status |
|------|---------|--------|
| B — Shortcut integration | [`phase-b-shortcut.md`](phase-b-shortcut.md) | ativo |
| C — Reports               | [`phase-c-report.md`](phase-c-report.md)   | ativo |
| D — TUI                   | [`phase-d-tui.md`](phase-d-tui.md)         | ativo |
| E — Tray + clipboard      | [`phase-e-tray.md`](phase-e-tray.md)       | ativo |
```

- [ ] **Step 3: Update `docs/superpowers/plans/README.md`**

Flip Phase E from `**ready**` to `**done (<TODAY>)**` and Phase F from `**draft after E merges**` to `**ready**`. Replace `<TODAY>` with the actual ISO date when you commit. The table after the edit reads:

```markdown
| Phase | File | State | Delivers |
|-------|------|-------|----------|
| A | [`2026-04-22-buckland-phase-a-cli-core.md`](2026-04-22-buckland-phase-a-cli-core.md) | **done (2026-04-24)** | CLI skeleton: `add`, `list`, `start`, `stop`, `pause`, `status`, `done`, `archive`, `delete`. Usable time-tracker via terminal. |
| B | [`2026-04-22-buckland-phase-b-shortcut.md`](2026-04-22-buckland-phase-b-shortcut.md) | **done (2026-04-24)** | Shortcut integration (read-only): `bl add --sc`, `bl shortcut`, SC-ID resolution in `bl start`, `shortcut_stories` cache with 1h TTL. |
| C | `2026-04-22-buckland-phase-c-report.md` | **done (2026-04-27)** | `bl report` with scope (today/week/month/all), grouping (task/epic/day), Unicode block bars, `--json` dump. |
| D | `2026-04-22-buckland-phase-d-tui.md` | **done (2026-04-28)** | ratatui TUI: Tasks, Agenda, Report screens plus Edit and Help overlays. Full keymap. |
| E | `2026-04-22-buckland-phase-e-tray.md` | **done (<TODAY>)** | `bl-tray` binary with ksni + local clock tick + `bl report --copy` clipboard helper, `bl tray` subcommand. |
| F | `2026-04-22-buckland-phase-f-oss.md` | **ready** | CI, `cargo deb`, release workflow, README with screenshots, CHANGELOG, CONTRIBUTING, CoC. |
```

- [ ] **Step 4: Create or extend `CHANGELOG.md`**

The repo currently lacks `CHANGELOG.md`. Create it now in [Keep a Changelog](https://keepachangelog.com/) format with the 0.5.0 entry and stub entries for the prior phases (versions inferred from commit history — they were each their own merge):

```markdown
# Changelog

All notable changes to Buckland will be documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - <TODAY>

### Added
- `bl-tray` binary: passive StatusNotifierItem tray icon (ksni) with idle
  / running / error states, 1Hz tooltip tick (glib), and a one-item Quit
  menu. Polls the SQLite file in read-only mode every
  `tray.poll_seconds` (default 30s).
- `bl tray` subcommand — same loop as `bl-tray`, exposed through the
  main binary so users on `cargo install buckland` always have the tray
  reachable from one entry point.
- `bl report --copy` / `-c` flag: copies the one-line summary (or the
  JSON body when `--json` is also set) to the system clipboard via
  `wl-copy` (Wayland) or `xclip` (X11). The TUI Report screen's `c`
  shortcut delegates to the same `crate::clipboard` module.

### Changed
- `tui::clipboard` is now a thin re-export of the new top-level
  `crate::clipboard` module. Internal refactor; public TUI surface
  unchanged.
- `Repo::active_snapshot` joins `time_entries`, `tasks`, and
  `shortcut_stories` in one round-trip — used by the tray.

### Internal
- New `RepoError::DatabaseMissing` variant distinguishes "user hasn't
  run `bl` yet" from "cannot read DB" so the tray can render the right
  tooltip.
- New `SqliteRepo::open_read_only` constructor wraps
  `OpenFlags::SQLITE_OPEN_READ_ONLY`.

## [0.4.0] - 2026-04-28

### Added
- ratatui TUI: Tasks, Agenda, Report screens plus Edit and Help
  overlays. Full keymap documented in the README.

## [0.3.0] - 2026-04-27

### Added
- `bl report` with scope (today/week/month/all/range), grouping
  (task/epic/day), Unicode block bars, and `--json` output.

## [0.2.0] - 2026-04-24

### Added
- Shortcut integration (read-only): `bl add --sc`, `bl shortcut`,
  SC-ID resolution in `bl start`, 1h-TTL cache.

## [0.1.0] - 2026-04-24

### Added
- CLI skeleton: `add`, `list`, `start`, `stop`, `pause`, `status`,
  `done`, `archive`, `delete`. Single-active-timer invariant enforced
  in schema and code.
```

Replace `<TODAY>` with the actual ISO date at commit time.

- [ ] **Step 5: Final verification (postcondition gauntlet)**

Run:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --all-targets --no-default-features -- -D warnings
cargo test --all-features
cargo test --no-default-features
cargo build --release --features tray
cargo build --release --no-default-features
```

Expected: every command green. The combined test count should be ≥ 220 (Phase D ended at ~205; Phase E adds at least 15 across `tray::assets`, `tray::state`, `tray::runtime`, `clipboard`, `cli_tray`, `cli_report_copy`, `config`, `storage::repo`).

`./target/release/bl --version` should print `bl 0.5.0`.

- [ ] **Step 6: Commit**

```text
chore: bump version to 0.5.0 (Phase E)
```

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md docs/smoke-tests/README.md docs/smoke-tests/phase-e-tray.md docs/superpowers/plans/README.md
git commit -m "chore: bump version to 0.5.0 (Phase E)"
```

- [ ] **Step 7: STOP — wait for the human smoke gate**

Per project memory rules: smoke test is the merge gate. Do NOT merge `phase-e-tray` into `main`. Surface the smoke checklist (`docs/smoke-tests/phase-e-tray.md`) to the user, wait for their explicit "merge OK", and only then run:

```bash
git checkout main
git merge --no-ff phase-e-tray -m "feat: Phase E — bl-tray + clipboard + bl tray"
# Push only when the user explicitly OKs it.
```

If the user reports a failed smoke step: do **not** amend any prior commit. Open a new branch off `phase-e-tray`, fix the regression, append commits, restart the smoke from the affected scenario.

---

## Self-review checklist (for the executing engineer)

Before handing the phase off to the user for the smoke gate, walk this list top-to-bottom:

1. **Spec coverage.** Every bullet under §"Tray" in the spec maps to either a unit test (state machine, tooltip, transition) or a smoke-test scenario (icon swap, polling cadence, GNOME caveat note). Every line of §"Command Grammar" → `bl tray` and `bl report -c` has an integration test.
2. **Feature gating.** `cargo build --no-default-features` succeeds without `ksni` or `glib` in the dep tree (run `cargo tree --no-default-features | grep -E '(ksni|glib)'` — must be empty).
3. **Read-only DB access.** `Repo::open_read_only` is the *only* path the tray uses. Grep `src/tray/` for `Connection::open(` (without `_with_flags`) — must be empty.
4. **No tokio.** Grep `src/tray/` and `src/clipboard.rs` for `tokio` and `async fn` — must be empty.
5. **Size budgets.** Each file ≤ 300 LOC including tests:
   - `src/tray/assets.rs` — well under (mostly constants and 5 tests).
   - `src/tray/state.rs` — implementation ~110 LOC, tests ~120 LOC, total ≤ 230.
   - `src/tray/runtime.rs` — implementation ~210 LOC, tests ~80 LOC, total ≤ 290 (just under). If it overflows, extract `BucklandTray` into its own file.
   - `src/clipboard.rs` — implementation ~110 LOC, tests ~50 LOC.
6. **No `unwrap()` in non-test library code.** Grep `src/tray/runtime.rs` for `.unwrap()` — only acceptable inside `expect("tray state poisoned")` (lock poisoning is a logic bug, not a data path).
7. **Error paths exercise `RepoError::DatabaseMissing`.** Smoke scenario 1 covers it from the user side; the unit test in Task E2 covers it from the code side.
8. **TUI regression: `c` key still copies.** Run `cargo test --features tray --lib tui::screens::report` — every Phase-D Report test stays green after the clipboard module move.
9. **Conventional Commits everywhere.** `git log phase-e-tray ^main --oneline` — every line starts with `feat:`/`fix:`/`chore:`/`docs:`/`refactor:`/`test:` and a scope where applicable.
10. **Eight commits exactly.** One per task. Run `git rev-list --count main..phase-e-tray` — must equal 8.
11. **No `todo!()`, `unimplemented!()`, or `dbg!()` in `src/`.** Grep — empty.
12. **`Cargo.lock` is committed alongside `Cargo.toml`.** Without it CI builds drift across machines.
13. **`CHANGELOG.md` `[0.5.0]` section uses today's date, not `<TODAY>`.** Same for the plan-index README.
14. **Smoke-test file renders without raw `<TODAY>` placeholders or `bash` syntax errors.** `bash -n docs/smoke-tests/phase-e-tray.md` won't parse markdown; instead, copy each fenced bash block into a shell and ensure it's syntactically valid (`bash -n <(extract)`).

If any item is unchecked, fix it on a follow-up commit on the same branch before surfacing the smoke checklist to the user. Do not amend prior commits.

## Postconditions (re-stated, copy from the top)

Walk the **Postconditions** section at the top of this file and tick every box. Anything still open is a regression in this plan and must be fixed before the smoke gate runs.

## Smoke test

The single source of truth for the merge gate is `docs/smoke-tests/phase-e-tray.md` (created in Task E8, Step 1). After the eighth commit lands and every automated test is green:

1. Surface the smoke file path to the user.
2. Walk through the checklist together (or have the user walk it solo and report).
3. Treat any "Sinais de alerta" hit as a hard merge block — open a follow-up commit, do not paper over.
4. After the user types "merge OK" (or equivalent explicit approval), run the merge from the **Step 7 STOP** block in Task E8.

Until that explicit OK lands, the branch is **done from the agent's side** but **not merged**. This separation is by user request — see the project memory entry `feedback_smoke_test_is_merge_gate.md`.

---

## What's next

When Phase E is merged on `main` and `0.5.0` is the working version:

- **Phase F — OSS readiness.** CI workflows (lint, test, MSRV, audit, build), `cargo deb` packaging, GitHub Actions release workflow on tag, README screenshot pass, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, issue templates. The first published `crates.io` release. Plan file: `docs/superpowers/plans/2026-04-22-buckland-phase-f-oss.md` (drafted after this phase merges, per the user's "draft only after the previous phase ships" rule).

See `docs/superpowers/plans/README.md` for the full phase index.
