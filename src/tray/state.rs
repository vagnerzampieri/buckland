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
            format!("Buckland: cannot read database \u{2014} {r}")
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
            "SC-{ext} {title} \u{2014} {elapsed_str} (started {started_str} local)",
            title = snap.task_title,
        ),
        None => format!(
            "#{id} {title} \u{2014} {elapsed_str} (started {started_str})",
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
        assert_eq!(
            tooltip(&TrayState::Idle, at_local(2026, 4, 22, 9, 0)),
            "Buckland: idle"
        );
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
            "Buckland: cannot read database \u{2014} permission denied"
        );
    }

    #[test]
    fn tooltip_error_truncates_long_reason() {
        // Use a multi-byte codepoint (em-dash = 3 UTF-8 bytes) so we verify
        // the cap is codepoint-based, not byte-based.
        let long = "\u{2014}".repeat(100);
        let s = TrayState::Error(long);
        let t = tooltip(&s, at_local(2026, 4, 22, 9, 0));
        let prefix = "Buckland: cannot read database \u{2014} ";
        assert!(t.starts_with(prefix));
        let reason_part = t.trim_start_matches(prefix);
        assert_eq!(
            reason_part.chars().count(),
            60,
            "error reason must be capped at 60 chars (counted by Unicode scalar values, not bytes)"
        );
    }

    #[test]
    fn tooltip_active_with_sc_shows_sc_prefix() {
        // Started at 09:15 local; "now" is 10:38:45 local => 1h23m45s elapsed.
        let started_local = at_local(2026, 4, 22, 9, 15);
        let now_local = at_local(2026, 4, 22, 10, 38) + chrono::Duration::seconds(45);
        let s = TrayState::Active(snap(
            1,
            "Fix login flow",
            Some(123),
            started_local.with_timezone(&Utc),
        ));
        assert_eq!(
            tooltip(&s, now_local),
            "SC-123 Fix login flow \u{2014} 01:23:45 (started 09:15 local)"
        );
    }

    #[test]
    fn tooltip_active_without_sc_shows_hash_prefix() {
        let started_local = at_local(2026, 4, 22, 14, 2);
        let now_local = at_local(2026, 4, 22, 14, 14) + chrono::Duration::seconds(3);
        let s = TrayState::Active(snap(
            7,
            "Refactor imports",
            None,
            started_local.with_timezone(&Utc),
        ));
        assert_eq!(
            tooltip(&s, now_local),
            "#7 Refactor imports \u{2014} 00:12:03 (started 14:02)"
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
        assert_eq!(
            icon_name(&TrayState::Idle),
            crate::tray::assets::ICON_NAME_IDLE
        );
        assert_eq!(
            icon_name(&TrayState::Active(snap(
                1,
                "x",
                None,
                at_utc(2026, 4, 22, 9, 0)
            ))),
            crate::tray::assets::ICON_NAME_RUNNING
        );
        assert_eq!(
            icon_name(&TrayState::NoDatabase),
            crate::tray::assets::ICON_NAME_IDLE
        );
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

    #[test]
    fn poll_to_state_with_reason_propagates_message() {
        let s = poll_to_state_with_reason(Err::<Option<ActiveSnapshot>, _>("locked".into()), false);
        assert_eq!(s, TrayState::Error("locked".into()));
    }

    #[test]
    fn poll_to_state_with_reason_prefers_no_database_over_active_snapshot() {
        let snap = snap(1, "ignored", None, at_utc(2026, 4, 22, 9, 0));
        let state = poll_to_state_with_reason(Ok(Some(snap)), true);
        assert_eq!(state, TrayState::NoDatabase);
    }
}
