# Buckland

Keyboard-first time tracker for developers who use Shortcut. All-Rust, local SQLite, Linux-first.

## Status

**Phases A, B, and C shipped** — `bl` is a usable CLI time tracker with read-only Shortcut integration and reports against a local SQLite file. TUI and tray are on the roadmap.

See [docs/superpowers/plans/](docs/superpowers/plans/) for the phase breakdown and [docs/superpowers/specs/2026-04-22-buckland-design.md](docs/superpowers/specs/2026-04-22-buckland-design.md) for the design spec.

## What works today

- Full task-and-timer lifecycle from the terminal: `bl add`, `list`, `start`, `stop` / `pause`, `status`, `done`, `archive`, `delete`.
- Shortcut integration (read-only): `bl add --sc SC-123` links a task to a story, `bl start SC-123` resumes or creates a task from a story, `bl shortcut SC-123` force-refreshes the cached story (1 h TTL).
- Reports: `bl report` with scope flags (`--today | --week | --month | --all | --range FROM..TO`), grouping flags (`--by-task | --by-epic | --by-day`), Unicode-block bars, and `--json` for scripting.
- Single-active-timer invariant enforced both in the schema (partial unique index) and in a transaction (`TimerOps::start` stops and starts atomically).
- Hard delete is blocked for tasks that have time entries — the CLI suggests `archive` instead, so history is preserved by default.
- Data lives in one SQLite file; no daemon, no HTTP surface.
- 154 unit + integration tests, `clippy -D warnings` clean, rustfmt clean.

## Coming next

| Phase | Delivers |
|-------|----------|
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

### Tasks and timers

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

### Shortcut integration

Configure a token in `$XDG_CONFIG_HOME/buckland/config.toml` (see Configuration below), then:

```bash
bl add "ship the new auth flow" --sc SC-123   # links the task to a story
bl start SC-123                                # resumes the linked task, or creates one
bl shortcut SC-123                             # force-refresh the cached story metadata
```

### Reports

```bash
bl report                                     # today, by task — table with Unicode bars
bl report --week --by-day                     # this ISO week, one row per local day
bl report --month --by-epic                   # this calendar month, grouped by Shortcut epic
bl report --range 2026-04-01..2026-04-30      # custom date range, inclusive endpoints
bl report --all --json | jq                   # machine-readable dump for scripting
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
# token = "abc..."              # required for bl add --sc, bl start SC-NNN, bl shortcut, bl report --by-epic
# api_base_url = "https://api.app.shortcut.com"  # override only for testing

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
