# Changelog

All notable changes to Buckland will be documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-05-17

### Added
- `bl-tray` binary: passive StatusNotifierItem tray icon (ksni) with idle
  / running / error states. Icons are rasterized (resvg) and shipped as
  ARGB32 pixmaps via SNI's `IconPixmap` — bypassing the icon-theme path
  so GNOME's `St.Icon` can't recolor monochrome SVGs as symbolic masks.
  The right-click menu has a disabled state line (`#N task — HH:MM:SS`)
  above a separator and Quit, and the same string is also published as
  `Title` / `ToolTip` for hosts that surface those (KDE Plasma).
  Polls the SQLite file in read-only mode every `tray.poll_seconds`
  (default 2s) and ticks the menu at 1Hz so the elapsed clock advances
  between polls.
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
- New `tray::render` module rasterizes the embedded SVGs once at startup
  to 16/22/32/48 px ARGB32 buffers (resvg + tiny_skia). Adds `resvg`
  dependency, gated behind the `tray` feature.
- Removed the `icon_name`-based pipeline: `state::icon_name`,
  `ICON_NAME_*` constants, `APP_ICON_SVG`, and the
  `install_theme_icons_at` helper. Hosts now receive raw bytes only.

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
