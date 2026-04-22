# Buckland — Future Ideas

Running backlog of things **deliberately deferred** during the 2026-04-22 brainstorm and multi-persona review, plus ideas that came up but didn't fit v1. This is a scratchpad, not a commitment — items move to a plan only after real v1 usage proves (or disproves) the need.

Sizing shorthand: **S** = a day, **M** = a week, **L** = multi-week. Complexity is a rough gut feel, not a contract.

---

## Product features

### Shortcut write-back (post time as story comment)
- **Origin:** spec Deferred; product-lens review.
- **Why:** the current flow ends at "copy total to Shortcut manually." If that step gets old, automating it closes the loop.
- **Shape:** a `bl sync <task-id>` command (or `:` in TUI) that posts a comment like `Spent 1h 23m 45s on this story today` via `POST /stories/{id}/comments`. Needs a new column `exported_at` + `external_reference` on `time_entries` so we don't double-post.
- **Watch out:** changing the PAT scope from read to write changes the blast radius — security review before landing.
- **Size:** M.

### Time budgets and targets
- **Origin:** idea from the brainstorm.
- **Why:** "I want to spend about 10h on SC-123 this week" is a real planning question.
- **Shape:** an optional `target_seconds` column on `tasks`; the TUI shows a progress bar; a notification fires at 80% and 100%.
- **Size:** M.

### Estimation vs actual
- **Origin:** idea.
- **Why:** the author might want to track how accurate their estimates are over time — a classic developer retrospection metric.
- **Shape:** `estimate_seconds` on `tasks`; reports add an "over/under" column.
- **Size:** S.

### Pause/resume with gap-aware single entries
- **Origin:** asked during brainstorm; picked stop+start instead for v1 simplicity.
- **Why:** some users prefer one `TimeEntry` per contiguous focus session, with explicit pauses rather than fresh entries.
- **Shape:** `paused_at` + `accumulated_seconds` on `time_entries`. Materially changes the schema; migrate with care.
- **Size:** M.

### Pomodoro / parallel timers
- **Origin:** spec Non-Goal.
- **Why:** some workflows benefit from enforced rest cycles or multiple concurrent focus streams.
- **Shape:** a new kind of timer (`kind` enum) that co-exists with the primary, with auto-stop via configurable duration. Relaxes the single-active invariant to a per-kind invariant.
- **Size:** M, invasive.

### Voice notes on entries
- **Origin:** idea — ties into the sibling `martin` project.
- **Why:** "what was I actually doing when I said the timer was for SC-123?" — a 15-second audio clip is worth more than a text note.
- **Shape:** store audio blobs outside SQLite (in `$XDG_DATA_HOME/buckland/audio/<uuid>.ogg`); `time_entries` gets a `notes_audio_path` column. Reuses martin's PipeWire capture path.
- **Size:** L (enters media territory).

## Integrations

### Linear, Jira, GitHub Issues adapters
- **Origin:** idea — the Shortcut integration is a pattern.
- **Why:** the core abstraction (fetch story → pre-fill title/epic) is issue-tracker-agnostic. A trait `IssueTracker` with per-provider impls would make Buckland useful beyond Shortcut.
- **Shape:** `shortcut::Client` generalized to a trait; providers chosen via `config.toml` (`issue_tracker = "shortcut" | "linear" | "jira" | "github"`). The `shortcut_stories` table becomes `issues` with a `provider` column.
- **Size:** L initially; each new provider after is M.
- **Risk:** scope creep. Only ship a provider when the author actually uses it.

### Git integration (associate commits to entries)
- **Origin:** idea.
- **Why:** a git commit during an active timer is strong evidence the commit is part of that task's work. Pairing is cheap.
- **Shape:** a `post-commit` hook that calls `bl annotate-last-entry --commit <sha>`; reports show commit counts alongside time totals.
- **Size:** S for the hook, S for the query, M for the reports.

### Calendar export (iCal / .ics)
- **Origin:** idea.
- **Why:** some workflows want to treat tracked time as calendar blocks for weekly reviews or reporting to managers.
- **Shape:** `bl export --format ical [range]` emits a valid `.ics` file with entries as events. Standard library + some formatting.
- **Size:** S.

## Platform / distribution

### macOS support
- **Origin:** spec Non-Goal.
- **Why:** if the author adopts a Mac or shares with Mac-using colleagues.
- **Shape:** core + CLI + TUI are already portable (ratatui, crossterm, rusqlite all work). The tray needs a different backend — `tray-icon` crate or NSStatusItem via `cocoa`. Feature-gated behind `--target-os=macos`.
- **Size:** M for tray; the rest is free.

### Windows support
- **Origin:** spec Non-Goal.
- **Why:** low priority, but non-zero user base.
- **Shape:** TUI works via Windows Terminal. Tray via `tray-icon` or raw Win32 NIM_ADD. Config paths via `dirs` crate (`dirs::data_dir()`) already handle this.
- **Size:** M.

### Flatpak / AppImage
- **Origin:** mentioned in the superseded plan.
- **Why:** reach distros where `.deb` doesn't land (Arch, Fedora, NixOS).
- **Shape:** a manifest under `packaging/flatpak/` + CI job. AppImage via `linuxdeploy` + `appimage-builder`.
- **Size:** M each (flatpak is fiddlier).

### Nix / Homebrew / AUR
- **Origin:** idea.
- **Why:** power users prefer their own package managers.
- **Shape:** PRs to `nixpkgs` / `homebrew-core` / AUR templates. Community-maintained if accepted; self-maintained otherwise.
- **Size:** S each.

## UX / interaction

### Global system hotkey
- **Origin:** discussed in brainstorm; CLI + tray ended up being the chosen capture path instead.
- **Why:** "press Ctrl+Shift+T from anywhere, get a capture prompt." Would replace the terminal round-trip for the single most common action.
- **Shape:** a small GTK dialog triggered by a `keybinder-3.0` hotkey. Adds GTK main-loop deps to `bl-tray` (or a third binary `bl-hotkey`). Platform-specific.
- **Size:** M.
- **Cost:** reintroduces a GUI surface. Maybe never needed if the CLI stays fast.

### Command palette (`:` in the TUI)
- **Origin:** spec Deferred.
- **Why:** fuzzy-invoke any action without memorizing keybindings.
- **Shape:** modal input at the bottom that fuzzy-matches command names.
- **Size:** S.

### Fuzzy search on task titles
- **Origin:** spec Deferred.
- **Why:** `/` does literal substring in v1; `sk` / `skim`-style fuzzy is nicer once the list grows.
- **Shape:** `nucleo` crate for the matcher; `/` enters fuzzy mode.
- **Size:** S.

### Mouse support in the TUI
- **Origin:** spec Deferred.
- **Why:** nice-to-have; ratatui supports it, cost is low.
- **Shape:** handle `Event::Mouse` in each screen; click on a task = select, double-click = start.
- **Size:** S.

### Customizable keybindings
- **Origin:** idea.
- **Why:** some users really want `k`/`j` reversed, or emacs bindings, or Dvorak layouts.
- **Shape:** `[keymap]` section in `config.toml`; `Keymap` struct loaded at startup.
- **Size:** M (needs care not to break default muscle memory).

### Theme presets (catppuccin, gruvbox, nord)
- **Origin:** spec Non-Goal.
- **Why:** defaults respect the terminal's 16 colors, which is already great — but some users want full theme independence.
- **Shape:** bundled presets as TOML fragments; `config.toml` names one. Still uses ANSI-indexed colors, just overrides specific roles.
- **Size:** S per preset.

## Data / operations

### Toggl import
- **Origin:** the author currently uses Toggl; this is the on-ramp.
- **Why:** picking up from where you left off is worth more than starting fresh.
- **Shape:** `bl import toggl <csv-file>`. Toggl's CSV export has "Start date", "Start time", "Duration", "Description" — map to `time_entries` with a synthetic task per unique description.
- **Size:** S.

### CSV / JSON export
- **Origin:** idea.
- **Why:** for accountants, managers, tax purposes, year-end reviews. Also future-proofs data portability.
- **Shape:** `bl export --format csv|json [range]`. `serde_json` is already a dep; `csv` crate for CSV.
- **Size:** S.

### Backup strategy documentation (and optional automation)
- **Origin:** review residual concern.
- **Why:** the SQLite file is the only record. A disk failure loses everything.
- **Shape:** document `cp` / `rsync` / `restic` workflows in the README. Optionally add `bl backup <destination>` that does atomic snapshot via `VACUUM INTO` (SQLite's recommended backup path).
- **Size:** S docs, S for the command.

### Auto-backup to a private git repo
- **Origin:** idea — low-tech sync that the author might already have.
- **Why:** gives "sync across machines" without a server, if the user keeps two machines pulling/pushing the same repo.
- **Shape:** `bl backup --to-git` commits an `sqlite3 .dump` to a configured repo. Serialized export is friendlier to git than binary SQLite.
- **Size:** S.

### Healthcheck / fsck
- **Origin:** idea from the review.
- **Why:** detect index corruption, orphaned rows, dangling timers early.
- **Shape:** `bl doctor` runs `PRAGMA integrity_check`, checks for `time_entries` without matching `tasks`, reports stale `shortcut_stories`, flags active timers older than 8h.
- **Size:** S.

### Multi-workspace Shortcut
- **Origin:** review finding (adversarial).
- **Why:** the single-workspace assumption breaks if the user has personal + work PATs.
- **Shape:** add `workspace_slug` column to `shortcut_stories`; unique index becomes `(workspace_slug, external_id)`. `config.toml` allows multiple `[shortcut.workspace.<name>]` tables with named tokens.
- **Size:** M.

## Quality of life

### Desktop notifications (`notify-send`)
- **Origin:** design-lens review.
- **Why:** "timer hit 1h", "you've been on this 8h — forgot to stop?", "Pomodoro cycle complete" are small signals that cost little to send and help a lot.
- **Shape:** `notify-rust` crate; config key `notifications.enabled` defaults to off. Events: timer-passed-threshold, dangling-timer-warning, daily-summary.
- **Size:** S.

### Idle-time detection (auto-stop on inactivity)
- **Origin:** review residual risk (dangling timers).
- **Why:** laptop lid closes, AFK, forgot to stop — all produce bad data.
- **Shape:** on Linux, query X11 / Wayland idle via `xprintidle` or `loginctl`. If idle > N minutes, auto-stop the timer with a notification so the user can resume or keep the auto-stop.
- **Size:** M (platform-specific probing).

### Suspend/resume awareness
- **Origin:** review residual concern.
- **Why:** laptop suspends for hours with a timer running; the tooltip goes stale; on resume we want the tray to re-sync cleanly.
- **Shape:** `bl-tray` subscribes to `org.freedesktop.login1.Manager.PrepareForSleep` via `zbus`. On sleep, pause polling; on wake, immediately re-poll.
- **Size:** S.

### Max-duration clamp in reports
- **Origin:** adversarial review.
- **Why:** a forgotten 14-hour timer distorts weekly totals badly.
- **Shape:** reports cap any single entry at `max_entry_hours` (default 8) and flag the row as "clamped". Raw totals still available via `--no-clamp`.
- **Size:** S.

### Weekly / monthly email (or markdown) summaries
- **Origin:** idea.
- **Why:** "here's what you did this week" in a portable format, maybe scheduled via cron.
- **Shape:** `bl report --format markdown > weekly.md`. Users pipe to `mail`, commit to a journal repo, paste into wikis.
- **Size:** S.

### Bilingual UI (pt/en) matching martin
- **Origin:** OSS-style parity.
- **Why:** the sibling project martin is bilingual; Buckland could follow.
- **Shape:** `fluent` or a lightweight key/value table; locale from `LC_ALL`.
- **Size:** M (mainly translation maintenance).

## Developer ergonomics

### Public local API (reintroduced)
- **Origin:** discussed as v1 primary, discarded in favor of SQLite-shared.
- **Why:** third parties (scripts, extensions, bars like `waybar`) could integrate without parsing CLI output.
- **Shape:** unix socket at `$XDG_RUNTIME_DIR/buckland.sock` with a JSON-line protocol. Sidesteps the browser-origin risk that haunted the HTTP proposal.
- **Size:** M.

### Plugin system
- **Origin:** idea.
- **Why:** extensibility without bloat.
- **Shape:** large design space. Simplest form: shell-out to user-provided scripts on events (`on_timer_start`, `on_timer_stop`). Harder form: WASM plugins via `wasmtime`.
- **Size:** L (and deferred until a concrete use case).

### Keyring support for the token
- **Origin:** security review alternative.
- **Why:** plaintext token in `config.toml` is fine for a personal-laptop context but not ideal. Keyring (Secret Service API on Linux) is the drop-in upgrade.
- **Shape:** `keyring` crate; `bl config set shortcut.token xxx --use-keyring` stores it, config file just notes "stored in keyring". Fallback to plaintext if libsecret is unavailable.
- **Size:** S.

### Structured logging
- **Origin:** idea.
- **Why:** for debugging with an `env_logger` / `tracing-subscriber` setup; helps issue triage in GitHub Issues.
- **Shape:** `tracing` + `tracing-subscriber`; `--verbose` on CLI bumps level; logs to `$XDG_STATE_HOME/buckland/buckland.log`.
- **Size:** S.

### Richer TUI tests (visual regression)
- **Origin:** testing section of spec.
- **Why:** `insta` snapshots of the rendered buffer catch visual regressions.
- **Shape:** extend the TestBackend tests into a full snapshot suite covering each screen in typical states.
- **Size:** M (mostly writing snapshot scenarios).

## Explicitly considered and rejected (for the memory)

- **Web UI** — rejected in the 2026-04-22 pivot. Rationale: the TUI-first hypothesis solves the author's actual friction; a web UI doubles the surface area. Don't revisit without a concrete user need the terminal cannot meet.
- **HTTP local API** — rejected because local browser tabs would be able to drive it without auth, and the shared-SQLite design meets all the original clients. A unix-socket version (listed above) is the acceptable future form.
- **Web UI + native companion split** — rejected in the 2026-04-22 pivot (see the superseded plan for the original shape). A single-user personal tool did not justify two codebases, a daemon, and a contract between them.
- **Mouse-first capture** — rejected: the author is keyboard-first.
- **Cloud sync** — rejected: local-first is a non-goal trade. Backup-to-git (listed above) is the escape hatch if the user ever wants off-machine copy.

---

## How to move an item to v1.x

When an item here graduates:
1. Confirm v1 is stable enough that the new work doesn't destabilize it.
2. Write a short brainstorm under `docs/superpowers/specs/` (even 100 lines) — this backlog doesn't replace the design pass.
3. Move the item here to a `Delivered in vX.Y` section at the bottom so the history stays visible.
4. Write the plan in `docs/plans/` and implement.
