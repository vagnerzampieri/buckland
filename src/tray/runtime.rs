//! Runtime glue: ksni service, polling thread, glib 1Hz tick.
//!
//! The pure parts (icon installation, poll-result mapping) are unit-tested
//! here. The DBus surface, glib loop, and ksni service are exercised by
//! the manual smoke test (`docs/smoke-tests/phase-e-tray.md`).

use crate::domain::ActiveSnapshot;
use crate::storage::repo::{RepoError, RepoResult};
use crate::storage::{Repo, SqliteRepo};
use crate::tray::{
    render::{render_state_icons, StatePixmaps},
    state::{tooltip, transition, TrayState},
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
            poll_seconds: 2,
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

// --- ksni service ----------------------------------------------------------

/// The shared mutex our three actors (poll thread, glib tick, ksni
/// service) hand off through. `Arc<Mutex<TrayState>>` is enough because
/// no actor holds the lock long: the poll thread reads the DB outside
/// the lock and only swaps the state atomically.
type SharedState = Arc<Mutex<TrayState>>;

struct BucklandTray {
    state: SharedState,
    pixmaps: Arc<StatePixmaps>,
}

#[cfg(feature = "tray")]
impl ksni::Tray for BucklandTray {
    fn id(&self) -> String {
        "buckland-tray".into()
    }

    fn title(&self) -> String {
        let s = self.state.lock().expect("tray state poisoned");
        tooltip(&s, Local::now())
    }

    // No `icon_name` impl: the default returns an empty string. We ship
    // only `icon_pixmap` so St.Icon on GNOME can't apply its symbolic
    // recoloring (which collapses any monochrome SVG to the panel's CSS
    // color and makes our idle icon invisible).
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let s = self.state.lock().expect("tray state poisoned");
        match &*s {
            TrayState::Idle | TrayState::NoDatabase => self.pixmaps.idle.clone(),
            TrayState::Active(_) => self.pixmaps.running.clone(),
            TrayState::Error(_) => self.pixmaps.error.clone(),
        }
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        let s = self.state.lock().expect("tray state poisoned");
        let title = tooltip(&s, Local::now());
        ksni::ToolTip {
            icon_name: String::new(),
            icon_pixmap: vec![],
            title,
            description: String::new(),
        }
    }

    /// Menu layout:
    /// `[ <state line, disabled> | --- | Quit ]`
    ///
    /// GNOME's `ubuntu-appindicators` doesn't render `Title` or `ToolTip`
    /// on hover, so we surface the live state as the first menu item.
    /// `enabled = false` greys it out — it's a label, not a command.
    /// The 1Hz tick triggers `Handle::update`, which re-emits the menu
    /// and the elapsed clock advances visibly when the user opens it.
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;
        let s = self.state.lock().expect("tray state poisoned");
        let state_label = tooltip(&s, Local::now());
        vec![
            StandardItem {
                label: state_label,
                enabled: false,
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|_: &mut Self| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

// --- Entry point -----------------------------------------------------------

/// Block the calling thread running the tray loop. Returns `Ok(())`
/// only when glib's main loop ends (e.g. the user picks "Quit", which
/// calls `std::process::exit(0)`). Errors are returned for setup
/// failures (DBus session unreachable, glib init).
#[cfg(feature = "tray")]
pub fn run(config: TrayRuntimeConfig) -> anyhow::Result<()> {
    use ksni::blocking::TrayMethods;

    let pixmaps = Arc::new(render_state_icons().context("rasterizing tray SVG icons")?);

    let state: SharedState = Arc::new(Mutex::new(TrayState::Idle));
    let tray = BucklandTray {
        state: Arc::clone(&state),
        pixmaps,
    };
    let handle = tray
        .spawn()
        .context("failed to register StatusNotifierItem on D-Bus")?;

    spawn_polling_thread(Arc::clone(&state), handle.clone(), config);

    let main_loop = glib::MainLoop::new(None, false);
    let tick_handle = handle.clone();
    glib::timeout_add_local(Duration::from_secs(1), move || {
        // 1Hz tooltip refresh. State doesn't change, but the elapsed
        // clock is rendered from `Local::now()`, so `update()` is what
        // tells SNI hosts "redraw the tooltip text".
        tick_handle.update(|_t: &mut BucklandTray| {});
        glib::ControlFlow::Continue
    });

    main_loop.run();
    Ok(())
}

#[cfg(feature = "tray")]
fn spawn_polling_thread(
    state: SharedState,
    handle: ksni::blocking::Handle<BucklandTray>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ActiveSnapshot;
    use crate::storage::repo::RepoError;
    use chrono::{TimeZone, Utc};

    fn snap(id: i64, started_h: u32) -> ActiveSnapshot {
        ActiveSnapshot {
            task_id: id,
            task_title: format!("task {id}"),
            sc_external_id: None,
            started_at: Utc.with_ymd_and_hms(2026, 4, 22, started_h, 0, 0).unwrap(),
        }
    }

    #[test]
    fn map_poll_result_database_missing_yields_no_database() {
        use std::path::PathBuf;
        let s = map_poll_result(Err::<Option<ActiveSnapshot>, _>(
            RepoError::DatabaseMissing(PathBuf::from("/nope")),
        ));
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
        assert_eq!(cfg.poll_seconds, 2);
    }
}
