# Buckland implementation plans

Buckland v1 is split into six phases. Each phase is a self-contained plan that produces a working, testable artifact. Execute them in order; each phase's "Preconditions" assume the previous ones have merged cleanly.

**Source of truth (always load alongside any phase):**
- Spec: [`../specs/2026-04-22-buckland-design.md`](../specs/2026-04-22-buckland-design.md)
- Project-wide dev guidelines: [`../../CLAUDE.md`](../../CLAUDE.md)
- Future backlog (out-of-scope for v1): [`../future-ideas.md`](../future-ideas.md)

## Phase index

| Phase | File | State | Delivers |
|-------|------|-------|----------|
| A | [`2026-04-22-buckland-phase-a-cli-core.md`](2026-04-22-buckland-phase-a-cli-core.md) | **done (2026-04-24)** | CLI skeleton: `add`, `list`, `start`, `stop`, `pause`, `status`, `done`, `archive`, `delete`. Usable time-tracker via terminal. |
| B | [`2026-04-22-buckland-phase-b-shortcut.md`](2026-04-22-buckland-phase-b-shortcut.md) | **done (2026-04-24)** | Shortcut integration (read-only): `bl add --sc`, `bl shortcut`, SC-ID resolution in `bl start`, `shortcut_stories` cache with 1h TTL. |
| C | `2026-04-22-buckland-phase-c-report.md` | **done (2026-04-27)** | `bl report` with scope (today/week/month/all), grouping (task/epic/day), Unicode block bars, `--json` dump. |
| D | `2026-04-22-buckland-phase-d-tui.md` | **done (2026-04-28)** | ratatui TUI: Tasks, Agenda, Report screens plus Edit and Help overlays. Full keymap. |
| E | `2026-04-22-buckland-phase-e-tray.md` | **ready** | `bl-tray` binary with ksni + local clock tick + clipboard helper (`bl report -c`). |
| F | `2026-04-22-buckland-phase-f-oss.md` | **draft after E merges** | CI, `cargo deb`, release workflow, README with screenshots, CHANGELOG, CONTRIBUTING, CoC. |

## How to execute a phase

**Fresh session workflow:**

1. `cd /home/nuuvem/Projects/study/buckland && git status` — verify clean tree on `main`.
2. Read `CLAUDE.md`, the spec, and this README.
3. Open the phase file at the top of the index that is currently `ready`.
4. Follow the Preconditions check at the top of that phase.
5. Execute the tasks in order using one of:
   - **`superpowers:executing-plans`** (inline, in this session) — batches with checkpoints between tasks.
   - **`superpowers:subagent-driven-development`** (fresh subagent per task) — higher isolation, better for long-running work.
6. Tick each step's checkbox as you finish it. Each task ends with a commit; keep them one-to-one.
7. After the final task, run the phase's "Self-review checklist" and verify the "Postconditions".
8. Move the phase's state in this table from `ready` to `done (<date>)` and mark the next phase `ready` (or `draft after X merges` if this phase's artifacts must stabilize first).

## When to draft the next phase

Only after the previous phase has merged to `main` and its Postconditions pass. Drafting early tempts us to bake in assumptions from the design that real usage might change.

The source material for drafting any phase is the spec — reread the relevant sections (Architecture, Data Model, Command Grammar, TUI, Tray, Shortcut Integration, Testing, CI & Release) and decompose the new phase into tasks using the same style as Phase A: exact file paths, complete code, TDD steps, one commit per task.

## Historical

The Rails-first plan that preceded this pivot lives at [`../../plans/2026-04-22-001-feat-buckland-timetracking-plan.md`](../../plans/2026-04-22-001-feat-buckland-timetracking-plan.md) with `status: superseded`. Kept as decision history; do not execute.
