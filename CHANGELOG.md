# Changelog

All notable changes to Buckland will be documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-05-10

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
