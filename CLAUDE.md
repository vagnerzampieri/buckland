# Buckland — Development Guidelines

## Project

Personal time-tracking for developers who use Shortcut. All-Rust, open-source (GPL-3.0), Linux-first (GNOME + KDE). One SQLite file owns the data; three surfaces consume it:

- **`bl`** — main binary. CLI mode (`bl add "fix login"`, `bl start SC-123`, `bl stop`, `bl report`) and TUI mode (`bl` with no args opens a fullscreen terminal UI).
- **`bl-tray`** — optional tray-icon binary (StatusNotifierItem via `ksni`). 100% passive: idle/running icon + tooltip with a locally-ticking clock.
- **Shared core** (`src/lib.rs`) — domain (Task, TimeEntry, TimerOps), storage (`rusqlite`), Shortcut client (`reqwest`).

No daemon, no local HTTP, no background service. Everything is a short-lived process reading/writing the same SQLite file under `$XDG_DATA_HOME/buckland/buckland.db`.

Plan of record: the latest file in [docs/plans/](docs/plans/). Historical plans keep `status: superseded` in their frontmatter.

### Shortcut Integration

Read-only. `shortcut::Client::fetch_story(id)` hits `https://api.app.shortcut.com/api/v3/stories/{id}` with header `Shortcut-Token`, then caches into the `shortcut_stories` table (TTL 1h). Token lives in `$XDG_CONFIG_HOME/buckland/config.toml` under `shortcut.token`. No write-back, no status changes — users copy totals from reports into Shortcut manually.

### Timer Invariant

At most one row in `time_entries` with `ended_at IS NULL`. Enforced both at the schema level (partial unique index on the constant expression `(1)` WHERE `ended_at IS NULL` — any two active rows collide on the same indexed value) and at the code level in `TimerOps::start`, which stops the active entry inside the same transaction before creating the new one.

## Core Philosophy

Write code that is easy to change, easy to understand, and easy to delete. Favor simplicity over cleverness. Follow Rust idioms and standard-library defaults unless there's a concrete, named reason not to.

## TDD & Tidy First — Kent Beck

From *Test-Driven Development: By Example* and *Tidy First?*.

1. **Red** — write a failing test that describes the behavior you want.
2. **Green** — write the simplest code that passes the test.
3. **Refactor** — clean up duplication and improve design while tests stay green.

Rules:
- No production code without a failing test.
- Make each step as small as possible. If it feels big, break it down.
- When stuck, write a simpler test.
- Tests are first-class code — keep them clean and readable.

**Separate structural from behavioral changes.** One commit tidies, the next commit adds behavior. A reviewer (human or agent) should be able to tell at a glance which kind a commit is.

## Refactoring — Martin Fowler

From *Refactoring*, 2nd edition.

- Refactor in small, named steps (Extract Function, Inline Variable, Move Function, Introduce Parameter Object, Replace Conditional with Polymorphism).
- "Make the change easy, then make the easy change." Preparatory refactoring lands before the feature, not as part of it.
- Each step keeps tests green. If they break, the step was too big.
- Watch for code smells: Long Function, Large Struct, Feature Envy, Data Clump, Primitive Obsession, Shotgun Surgery, Divergent Change.
- Refactoring preserves behavior. If you're changing behavior, it is not a refactor.

## Clean Code — Robert C. Martin

- **Names reveal intent.** If a name needs a comment, the name is wrong.
- **Functions do one thing.** Small, focused, single level of abstraction.
- **Single Responsibility Principle.** One reason to change per type, one per function.
- **No side effects.** A function named `fetch_story` doesn't also persist a `ShortcutStory`.
- **DRY only when real.** Three or more occurrences, not structural similarity.
- **Boy Scout Rule** — leave the code cleaner than you found it, but only in code you're already touching.

## Object Design — Sandi Metz principles (Rust-adapted)

Metz's four POODR rules are OO-strict; her principles translate well:

- **Inject dependencies, don't hardcode them.** Functions receive collaborators through parameters (trait objects for polymorphism, generics for monomorphization). No hidden globals or singletons.
- **Depend on behavior, not data.** Small traits with focused responsibilities; don't reach into struct internals of another module.
- **Prefer duplication over the wrong abstraction.** Wait until the pattern is real. A premature abstraction is more expensive than three copies.
- **Prefer composition over tall trait hierarchies.** Rust has no inheritance; translate as: compose small structs and implement focused traits, rather than stacking blanket impls on a god trait.

Size heuristics (budget, not dogma):

- Functions ≤ ~30 LOC.
- Source files ≤ ~300 LOC (including tests in the same file).
- Parameter count ≤ 4. For more, introduce a struct.
- One "unit of concern" per CLI subcommand / per TUI screen — the Metz rule-4 spirit.

Break a rule when you have a concrete reason. Then write the reason down.

## Rust Idioms First

Rust ships strong defaults. Before inventing a layer, reach for what the ecosystem already offers:

- **Error handling** — `thiserror` for typed library errors, `anyhow` for binary entry points. Never `.unwrap()` in library code; propagate with `?`. Panic only on genuinely unrecoverable invariants.
- **Config** — `serde` + `toml`, one `Config` struct. No config-loader framework.
- **SQLite** — `rusqlite` directly with prepared statements. No ORM. Domain structs implement `TryFrom<&rusqlite::Row>`.
- **Migrations** — `rusqlite_migration` with append-only SQL files under `migrations/`. Never edit a migration that has been committed.
- **HTTP** — `reqwest::blocking` for the Shortcut client (keeps the code synchronous; the one remote call per task creation is short enough).
- **Async** — sync by default. Only reach for `tokio` if a meaningful UI surface would block otherwise. TUI and tray today are sync.
- **CLI** — `clap` v4 with derive macros.
- **TUI** — `ratatui` + `crossterm` backend.
- **Tray** — `ksni` (StatusNotifierItem). Icon assets as SVG, embedded via `include_bytes!`.
- **Dates/times** — `chrono` with `Local` timezone for day bucketing, `Utc` for storage (convert at the boundary).
- **Testing HTTP** — `wiremock` (preferred) or `mockito`. Stub at the HTTP boundary, not at the client.
- **Lints** — `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` clean before every commit.

## Layered Design When Idioms Aren't Enough

Each layer earns its existence by solving a specific problem:

- **Domain functions** (`src/domain/`) — `TimerOps`, `TaskRepo`, `ReportBuilder`. Pure logic, take a `&Connection` or a `&dyn Repo`, no I/O beyond what's passed in.
- **Storage** (`src/storage/`) — thin `rusqlite` wrappers. One function per query at first. No caching, no retries — that's domain's job if needed.
- **Adapters** (`src/shortcut/`) — external API clients with timeout, typed errors, and a cache policy.
- **Surfaces** — `src/main.rs` (CLI dispatch + TUI entry), `src/bin/bl-tray.rs`. Compose domain + storage + adapters. No domain logic inline.

If a module wraps a single function call with no added behavior, delete it and inline it. Layering is architectural intent made visible, not organizational theater.

## Principles as Filters, Not Dogma

SOLID, KISS, YAGNI as questions:

- **SRP** — Does this type have more than one reason to change?
- **OCP** — Can I extend this without editing it?
- **LSP** — Do my trait impls honor the trait's contract?
- **ISP** — Am I forcing a caller to depend on methods they don't use?
- **DIP** — Does this depend on a concrete type when a trait would do?
- **KISS** — Is there a simpler design that still solves the whole problem?
- **YAGNI** — Do I have a concrete, current requirement for this flexibility? Or am I speculating?

Ask. If the answer is "no, it's fine," move on without ceremony.

## TUI Conventions (ratatui)

- One screen = one module under `src/tui/screens/`. A screen is a struct with `render(&self, frame, area)` and `handle_event(&mut self, event) -> Option<Action>`.
- State is owned by the `App`; screens get `&State` or `&mut State` but never own persistent data.
- Events from `crossterm::event` run on the main thread. Long work (Shortcut fetch, heavy queries) runs off-thread via `std::thread::spawn` with an `mpsc::channel` back to the main loop.
- No direct `rusqlite` calls in screens — always through a `Repo` trait exposed by domain.
- Keyboard map documented in a single `src/tui/keymap.rs` module; screens consume it, don't redefine keys locally.

## Tray Conventions (ksni)

- `bl-tray` is stateless at process level. Reads the SQLite file in read-only mode (`open_with_flags(SQLITE_OPEN_READ_ONLY)`).
- Poll `time_entries` for the active row every N seconds (default 30s) to detect state transitions.
- Tooltip's elapsed time is computed locally from `started_at` plus a monotonic wall-clock tick at 1Hz via `glib::timeout_add_local`. Polling detects state changes; it does not drive the clock.
- Icon swap on state transition only. Two SVGs embedded: `tray-idle.svg`, `tray-running.svg`.
- No menu beyond "Quit" in v1 (passive-tray decision — any action goes through `bl` CLI/TUI).

## Testing Approach

- **Unit tests** — inline `#[cfg(test)] mod tests` per module. Cover pure logic, edge cases, invariants.
- **Integration tests** — `tests/` directory. Cover CLI dispatch end-to-end with an in-memory SQLite (`rusqlite::Connection::open_in_memory()`).
- **HTTP tests** — `wiremock` spins a local server; the Shortcut client is tested against it with real wire bytes.
- **TUI tests** — `ratatui::backend::TestBackend` renders to a buffer; snapshot-style asserts on cell content plus event-driven state transitions.
- **Don't mock what you own.** Feed real domain structs into pure functions; only mock at HTTP and filesystem boundaries.
- Tests are independent and repeatable. No shared fixtures that leak state.

## Sustainable Development

- `rustfmt` and `clippy` run in CI. Warnings deny the build.
- Boring is good. No procedural macros unless there's a concrete reason.
- Schema constraints enforce invariants: `NOT NULL`, FK with `ON DELETE RESTRICT` where history must be preserved, partial unique indexes for "at most one" rules.
- Migrations are append-only code. Never edit a committed migration; create a new one.
- Every dependency is a liability. Add only when a concrete need appears. Keep MSRV declared in `Cargo.toml` and document it in the README.
- Every release buildable on stock Ubuntu 22.04+ with `cargo build --release` — no OS-specific hacks beyond the tray.

## Code Style

- Rust edition 2021, MSRV declared in `Cargo.toml`.
- `snake_case` for functions and variables, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for consts.
- Public items documented with `///`; private items documented only when the *why* is non-obvious.
- Guard clauses + early returns over nesting.
- No comments that restate the code. Comments explain *why*.

## OSS Release

Matches the approach from the sibling `martin` project:

- **License**: GPL-3.0-or-later.
- **README**: features, requirements, install (`.deb` + `cargo install buckland`), quickstart, screenshots of the TUI and tray.
- **Changelog**: `CHANGELOG.md` following [Keep a Changelog](https://keepachangelog.com/).
- **Releases**: GitHub Actions builds `.deb` for Debian/Ubuntu plus `.tar.gz` on tag.
- **crates.io**: publish `buckland` once the CLI surface stabilizes.
- **Issue templates and `CONTRIBUTING.md`** land with the first release.

## Commands

```bash
# Development
cargo run -- <args>                      # Run bl with CLI args
cargo run --bin bl-tray                  # Run tray binary
cargo watch -x test                      # Test loop (requires cargo-watch)

# Testing
cargo test                               # All tests
cargo test --test integration            # Just integration tests
cargo test <name>                        # Filter by name

# Quality
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo audit                              # Security advisories (requires cargo-audit)

# Release
cargo build --release
cargo install --path .                   # Install locally
```

## References

- The Rust Programming Language: https://doc.rust-lang.org/book/
- Rust API Guidelines: https://rust-lang.github.io/api-guidelines/
- Rust Design Patterns: https://rust-unofficial.github.io/patterns/
- ratatui: https://ratatui.rs/
- ksni crate: https://docs.rs/ksni/
- rusqlite: https://docs.rs/rusqlite/
- clap v4 derive: https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html
- Shortcut API: https://developer.shortcut.com/api/rest/v3
