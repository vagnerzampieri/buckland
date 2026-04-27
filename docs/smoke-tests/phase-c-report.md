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
