use crate::commands::lock_db;
use crate::error::AppError;
use crate::AppState;

use std::collections::HashMap;
use tauri::State;

#[derive(serde::Serialize, Clone)]
pub struct OpenApp {
    pub(crate) name: String,
    pub(crate) id: String,
    // Group identity: the owning app, without the window title. The frontend
    // looks its icon up by this, so several windows share one payload entry.
    pub(crate) app: String,
}

/// The app list plus a side map of icons, keyed by app identity rather than
/// carried per row: a 20-window app would otherwise repeat the same base64 PNG
/// twenty times in the payload.
#[derive(serde::Serialize)]
pub struct OpenAppsResult {
    pub(crate) apps: Vec<OpenApp>,
    pub(crate) icons: HashMap<String, String>,
}

// Longest process name accepted as a usage key. This is the only path that ever
// creates a row, so it is where junk is kept out of the table.
const MAX_APP_LEN: usize = 256;

fn validate_app(app: &str) -> Result<(), AppError> {

    if app.is_empty() {
        return Err(AppError::Validation("app name is empty".into()));
    }

    if app.len() > MAX_APP_LEN {
        return Err(AppError::Validation(format!(
            "app name too long: {} bytes",
            app.len()
        )));
    }

    Ok(())
}

/// Record one switch to `app`, validating the key on the way in: this is the
/// only path that ever creates a row.
fn record_use(state: &State<'_, AppState>, app: &str) -> Result<(), AppError> {

    validate_app(app)?;

    let conn = lock_db(state)?;

    usage::bump(&conn, app, usage::now())
}

// async so Tauri runs these off the main thread; the platform helpers spawn a
// blocking subprocess and must not freeze the UI event loop.
#[tauri::command]
pub async fn list_open_apps(state: State<'_, AppState>) -> Result<OpenAppsResult, AppError> {

    let mut result = platform::list_open_apps()?;

    // The ordering is a nicety and the list is the feature, so a database
    // failure degrades to the platform's alphabetical order rather than leaving
    // the popup empty. Scoped so the guard, which is not Send, cannot outlive
    // the block and make this future non-Send.
    let scores = match lock_db(&state).and_then(|conn| usage::load_scores(&conn)) {
        Ok(scores) => scores,
        Err(e) => {
            log::warn!("list_open_apps: could not load usage scores: {e}");
            HashMap::new()
        }
    };

    usage::apply_order(&mut result.apps, &scores, usage::now());

    Ok(result)
}

#[tauri::command]
pub async fn focus_app(
    id: String,
    app: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {

    platform::focus_app(&id)?;

    // The switch already happened and the window is up. Nothing about recording
    // it, not even a rejected name, may report a failure for an action that
    // visibly succeeded.
    if let Err(e) = record_use(&state, &app) {
        log::warn!("focus_app: could not record usage for '{app}': {e}");
    }

    Ok(())
}

// Holding the hotkey down makes Windows auto-repeat WM_HOTKEY about thirty
// times a second, which would spin the rotation rather than advance it one
// window per press. Deliberate tapping never gets this close together.
const CYCLE_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(90);

fn cycle_allowed() -> bool {

    static LAST: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

    // A poisoned lock would disable the hotkey for the rest of the process;
    // rotating an extra time is the cheaper failure.
    let Ok(mut last) = LAST.lock() else {
        return true;
    };

    let now = std::time::Instant::now();

    if last.is_some_and(|prev| now.duration_since(prev) < CYCLE_DEBOUNCE) {
        return false;
    }

    *last = Some(now);

    true
}

/// Rotate the windows of whichever app currently has focus.
///
/// Called straight from the shortcut handler rather than through a Tauri
/// command: the rotation shows no UI and never gives ClipX focus, so the
/// frontend is not involved at all.
pub(crate) fn cycle_windows() {

    if !cycle_allowed() {
        return;
    }

    match platform::cycle_active_app_windows() {
        Ok(true) => {}
        Ok(false) => log::debug!("cycle_windows: the focused app has nothing to cycle to"),
        Err(e) => log::warn!("cycle_windows: {e}"),
    }
}

/// Frecency ordering for the open-apps list.
///
/// Keyed on `app`, the process name, never on `id`: `id` carries the window
/// title on macOS and the window handle on Windows, so neither survives a
/// restart, while the process name is also the icon key and the level at which
/// the owner thinks about an app. Every window of an app inherits its score.
///
/// Sits above the `#[cfg]` platform split, so one implementation covers both and
/// neither platform sort is touched.
mod usage {

    use super::OpenApp;
    use crate::error::AppError;

    use rusqlite::Connection;
    use std::cmp::Ordering;
    use std::collections::HashMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    const HOUR: i64 = 3600;
    const DAY: i64 = 24 * HOUR;

    pub fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    /// Uses, discounted by how long ago the app was last focused.
    ///
    /// Buckets rather than a continuous decay, deliberately: the order only moves
    /// when a boundary is crossed, so the top of the list holds still for hours
    /// and the `1`-`9` keys stay usable from muscle memory.
    pub fn score(uses: i64, last_used: i64, now: i64) -> f64 {

        let age = now - last_used;

        let weight = if age <= HOUR {
            8.0
        } else if age <= DAY {
            4.0
        } else if age <= 7 * DAY {
            2.0
        } else if age <= 30 * DAY {
            1.0
        } else {
            0.5
        };

        uses as f64 * weight
    }

    /// Sort by descending score, stably, on top of whatever order the platform
    /// module produced. An app with no row scores `0.0` and therefore keeps its
    /// alphabetical placement, and the several windows of one app share a score
    /// and are already adjacent, so they stay grouped and in title order.
    pub fn apply_order(apps: &mut [OpenApp], scores: &HashMap<String, (i64, i64)>, now: i64) {

        let score_of = |a: &OpenApp| {
            scores
                .get(&a.app)
                .map_or(0.0, |&(uses, last_used)| score(uses, last_used, now))
        };

        // `partial_cmp` cannot see a NaN here (`uses` is an integer and the
        // weights are literals); the fallback keeps the comparator total rather
        // than risking a panic on a contract violation.
        apps.sort_by(|a, b| score_of(b).partial_cmp(&score_of(a)).unwrap_or(Ordering::Equal));
    }

    pub fn load_scores(conn: &Connection) -> Result<HashMap<String, (i64, i64)>, AppError> {

        let mut stmt = conn.prepare("SELECT app, uses, last_used FROM app_usage")?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, (row.get(1)?, row.get(2)?)))
        })?;

        Ok(rows.collect::<Result<HashMap<_, _>, _>>()?)
    }

    pub fn bump(conn: &Connection, app: &str, now: i64) -> Result<(), AppError> {

        conn.execute(
            "INSERT INTO app_usage (app, uses, last_used) VALUES (?1, 1, ?2) \
             ON CONFLICT(app) DO UPDATE SET uses = uses + 1, last_used = ?2",
            rusqlite::params![app, now],
        )?;

        Ok(())
    }
}

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;

#[cfg(any(target_os = "windows", test))]
mod window_cycle;
#[cfg(any(target_os = "windows", test))]
mod windows_icons;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as platform;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod other;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
use other as platform;

#[cfg(test)]
mod tests {

    use super::usage::{apply_order, bump, load_scores, score};
    use super::{validate_app, OpenApp};

    use crate::db::init_db;
    use crate::error::AppError;

    use rusqlite::Connection;
    use std::collections::HashMap;

    const HOUR: i64 = 3600;
    const DAY: i64 = 24 * HOUR;
    const NOW: i64 = 1_700_000_000;

    fn app(name: &str, title: &str) -> OpenApp {
        OpenApp {
            name: format!("{name} - {title}"),
            id: format!("{name}\u{1f}{title}"),
            app: name.to_string(),
        }
    }

    fn names(apps: &[OpenApp]) -> Vec<&str> {
        apps.iter().map(|a| a.name.as_str()).collect()
    }

    fn db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        init_db(&mut conn).unwrap();
        conn
    }

    #[test]
    fn score_weights_each_bucket() {
        assert_eq!(score(1, NOW - 60, NOW), 8.0);
        assert_eq!(score(1, NOW - 2 * HOUR, NOW), 4.0);
        assert_eq!(score(1, NOW - 3 * DAY, NOW), 2.0);
        assert_eq!(score(1, NOW - 10 * DAY, NOW), 1.0);
        assert_eq!(score(1, NOW - 90 * DAY, NOW), 0.5);
    }

    // Pins the comparison operator by test rather than by reading: an age landing
    // exactly on a boundary belongs to the nearer bucket.
    #[test]
    fn score_boundaries_are_inclusive() {
        assert_eq!(score(1, NOW - HOUR, NOW), 8.0);
        assert_eq!(score(1, NOW - HOUR - 1, NOW), 4.0);
        assert_eq!(score(1, NOW - DAY, NOW), 4.0);
        assert_eq!(score(1, NOW - DAY - 1, NOW), 2.0);
        assert_eq!(score(1, NOW - 7 * DAY, NOW), 2.0);
        assert_eq!(score(1, NOW - 7 * DAY - 1, NOW), 1.0);
        assert_eq!(score(1, NOW - 30 * DAY, NOW), 1.0);
        assert_eq!(score(1, NOW - 30 * DAY - 1, NOW), 0.5);
    }

    #[test]
    fn score_multiplies_by_uses() {
        assert_eq!(score(5, NOW - 60, NOW), 40.0);
    }

    #[test]
    fn an_app_with_no_row_scores_zero() {
        let mut apps = vec![app("Zed", "main"), app("Chrome", "Inbox")];
        let scores = HashMap::new();

        apply_order(&mut apps, &scores, NOW);

        // No row anywhere means no reordering at all.
        assert_eq!(names(&apps), vec!["Zed - main", "Chrome - Inbox"]);
    }

    #[test]
    fn a_used_app_rises_above_an_alphabetically_earlier_one() {
        let mut apps = vec![app("Chrome", "Inbox"), app("Zed", "main")];
        let scores = HashMap::from([("Zed".to_string(), (3, NOW - 60))]);

        apply_order(&mut apps, &scores, NOW);

        assert_eq!(names(&apps), vec!["Zed - main", "Chrome - Inbox"]);
    }

    #[test]
    fn windows_of_one_app_stay_adjacent_and_in_order() {
        let mut apps = vec![
            app("Chrome", "Docs"),
            app("Chrome", "Inbox"),
            app("Slack", "general"),
            app("Zed", "main"),
        ];
        let scores = HashMap::from([("Chrome".to_string(), (10, NOW - 60))]);

        apply_order(&mut apps, &scores, NOW);

        assert_eq!(
            names(&apps),
            vec!["Chrome - Docs", "Chrome - Inbox", "Slack - general", "Zed - main"]
        );
    }

    // The "nothing learned yet, behave exactly as today" guarantee.
    #[test]
    fn a_list_with_no_scores_is_left_identical() {
        let original = vec![app("Chrome", "Inbox"), app("Slack", "general"), app("Zed", "main")];
        let mut apps = original.clone();

        apply_order(&mut apps, &HashMap::new(), NOW);

        assert_eq!(names(&apps), names(&original));
        assert_eq!(
            apps.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
            original.iter().map(|a| a.id.as_str()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn first_bump_inserts_and_second_increments() {
        let conn = db();

        bump(&conn, "Zed", NOW).unwrap();
        assert_eq!(load_scores(&conn).unwrap()["Zed"], (1, NOW));

        bump(&conn, "Zed", NOW + 500).unwrap();
        assert_eq!(load_scores(&conn).unwrap()["Zed"], (2, NOW + 500));
    }

    #[test]
    fn load_scores_is_empty_on_a_fresh_db() {
        assert!(load_scores(&db()).unwrap().is_empty());
    }

    #[test]
    fn validate_app_rejects_empty_and_over_long_names() {
        assert!(matches!(validate_app(""), Err(AppError::Validation(_))));
        assert!(matches!(
            validate_app(&"a".repeat(257)),
            Err(AppError::Validation(_))
        ));
        assert!(validate_app(&"a".repeat(256)).is_ok());
        assert!(validate_app("Zed").is_ok());
    }

    #[test]
    fn init_db_prunes_stale_rows_and_spares_recent_ones() {
        let mut conn = db();

        conn.execute(
            "INSERT INTO app_usage (app, uses, last_used) VALUES \
             ('Stale', 9, strftime('%s','now') - 15552001), \
             ('Fresh', 1, strftime('%s','now') - 86400)",
            [],
        )
        .unwrap();

        init_db(&mut conn).unwrap();

        let scores = load_scores(&conn).unwrap();
        assert!(!scores.contains_key("Stale"));
        assert!(scores.contains_key("Fresh"));
    }
}
