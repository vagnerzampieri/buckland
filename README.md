# Buckland

Keyboard-first time tracker for developers who use Shortcut. All-Rust, local SQLite, Linux-first.

## Status

**Phase A shipped** — `bl` is a usable CLI time tracker against a local SQLite file. TUI, tray, Shortcut integration, and reports are on the roadmap.

See [docs/superpowers/plans/](docs/superpowers/plans/) for the phase breakdown and [docs/superpowers/specs/2026-04-22-buckland-design.md](docs/superpowers/specs/2026-04-22-buckland-design.md) for the design spec.

## What works today

- Full task-and-timer lifecycle from the terminal: `bl add`, `list`, `start`, `stop` / `pause`, `status`, `done`, `archive`, `delete`.
- Single-active-timer invariant enforced both in the schema (partial unique index) and in a transaction (`TimerOps::start` stops and starts atomically).
- Hard delete is blocked for tasks that have time entries — the CLI suggests `archive` instead, so history is preserved by default.
- Data lives in one SQLite file; no daemon, no HTTP surface.
- 68 unit + integration tests, `clippy -D warnings` clean, rustfmt clean.

## Coming next

| Phase | Delivers |
|-------|----------|
| B | Shortcut integration: `bl add --sc SC-123`, `bl shortcut`, SC-ID resolution in `bl start`, 1h-TTL story cache. |
| C | `bl report` with scope (today/week/month/all), grouping (task/epic/day), Unicode bars, `--json`. |
| D | Ratatui TUI: Tasks, Agenda, Report screens plus Edit and Help overlays. |
| E | `bl-tray` StatusNotifierItem icon with a local 1 Hz clock tick. |
| F | CI, `cargo deb`, release workflow, crates.io publish. |

## Requirements

- Rust 1.74+ (MSRV declared in `Cargo.toml`).
- Linux. Tested on recent Ubuntu / Fedora / Arch. macOS and Windows are not supported yet — the core is portable but the tray is Linux-only via `ksni`.

## Install from source

```bash
git clone https://github.com/vagnerzampieri/buckland.git
cd buckland
cargo install --path .
```

This installs `bl` into `~/.cargo/bin` (ensure it is on your `$PATH`). `.deb` packages and `cargo install buckland` from crates.io arrive in Phase F.

## Quickstart

```bash
bl add "fix the login bug"
bl add "refactor the import script"
bl list                         # both tasks listed
bl start 1                      # start task #1
bl status                       # shows running + HH:MM:SS
bl start 2                      # atomically switches to task #2
bl stop
bl done 1                       # mark #1 complete
bl list                         # only open tasks (hides #1)
bl list --completed             # shows #1 with a ✓ glyph
bl archive 2                    # hide from the default list; history kept
bl list --all                   # everything, with status glyphs
```

Use `bl <command> --help` for flag details on any subcommand.

## Configuration

### Data

Default database path: `$XDG_DATA_HOME/buckland/buckland.db` (typically `~/.local/share/buckland/buckland.db`).

Override the whole directory — handy for sandboxing, CI, or parallel profiles:

```bash
export BUCKLAND_HOME=/tmp/bl-sandbox
bl add "scratch task"
```

### Config file (optional)

`$XDG_CONFIG_HOME/buckland/config.toml` — created with mode `0600` so the Shortcut token stays private. Missing file means defaults.

```toml
[shortcut]
# token = "abc..."              # Phase B

[ui]
icons = "unicode"               # or "ascii"
accent_color = "cyan"

[tray]
poll_seconds = 30               # Phase E
```

## Development

```bash
cargo run -- <args>                          # run bl with CLI args
cargo test                                   # full suite
cargo clippy --all-targets -- -D warnings    # lint
cargo fmt --all                              # format
```

See [CLAUDE.md](CLAUDE.md) for the full engineering guidelines (TDD, size budgets, layered design, Rust-idioms-first).

## Contributing

Issues and PRs welcome. A few ground rules:

- Commits follow Conventional Commits (`feat:`, `fix:`, `refactor:`, `docs:`...).
- `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` must be clean before a PR.
- New behavior lands with a test that would have caught its absence.
- Migrations are append-only — never edit a committed migration.

## License

[GPL-3.0-or-later](LICENSE).
