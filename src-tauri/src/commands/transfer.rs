use crate::commands::lock_db;
use crate::error::AppError;
use crate::{AppState, MAX_CLIP_BYTES, MAX_DESC_BYTES};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tauri::State;

/// Bumped only when the file shape changes in a way this reader could not handle.
/// A file carrying a higher version is refused outright rather than guessed at.
const EXPORT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryRow {
    id: i64,
    content: String,
    #[serde(default)]
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionRow {
    id: i64,
    name: String,
    #[serde(default)]
    is_global: bool,
    #[serde(default)]
    is_active: bool,
    #[serde(default)]
    sort_order: i64,
    #[serde(default)]
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PinnedRow {
    id: i64,
    content: String,
    session_id: i64,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    sort_order: i64,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    hidden: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportFile {
    version: u32,
    exported_at: String,
    #[serde(default)]
    clipboard_history: Vec<HistoryRow>,
    #[serde(default)]
    sessions: Vec<SessionRow>,
    #[serde(default)]
    clipboard_pinned: Vec<PinnedRow>,
}

#[derive(Serialize)]
pub struct ExportSummary {
    history: usize,
    sessions: usize,
    pinned: usize,
}

/// Counts shown in the confirm dialog before anything is deleted: what the file
/// holds, what will actually land after the history limit is applied, and what
/// is about to be replaced.
#[derive(Serialize)]
pub struct ImportPreview {
    version: u32,
    exported_at: String,
    file_history: usize,
    history_to_import: usize,
    file_sessions: usize,
    file_pinned: usize,
    current_history: i64,
    current_sessions: i64,
    current_pinned: i64,
}

fn read_all(conn: &Connection) -> Result<ExportFile, AppError> {

    let exported_at: String =
        conn.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%SZ', 'now')", [], |r| r.get(0))?;

    let clipboard_history = conn
        .prepare("SELECT id, content, created_at FROM clipboard_history ORDER BY id")?
        .query_map([], |row| {
            Ok(HistoryRow {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let sessions = conn
        .prepare(
            "SELECT id, name, is_global, is_active, sort_order, created_at \
             FROM sessions ORDER BY id",
        )?
        .query_map([], |row| {
            Ok(SessionRow {
                id: row.get(0)?,
                name: row.get(1)?,
                is_global: row.get::<_, i64>(2)? != 0,
                is_active: row.get::<_, i64>(3)? != 0,
                sort_order: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let clipboard_pinned = conn
        .prepare(
            "SELECT id, content, session_id, created_at, sort_order, description, hidden \
             FROM clipboard_pinned ORDER BY id",
        )?
        .query_map([], |row| {
            Ok(PinnedRow {
                id: row.get(0)?,
                content: row.get(1)?,
                session_id: row.get(2)?,
                created_at: row.get(3)?,
                sort_order: row.get(4)?,
                description: row.get(5)?,
                hidden: row.get::<_, i64>(6)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ExportFile {
        version: EXPORT_VERSION,
        exported_at,
        clipboard_history,
        sessions,
        clipboard_pinned,
    })
}

/// Parse and fully validate before the caller is allowed to touch the database.
///
/// Import replaces, so it deletes before it writes. Anything that would fail
/// mid-insert has to be caught here, while the existing rows are still intact.
fn parse_export(json: &str) -> Result<ExportFile, AppError> {

    let file: ExportFile = serde_json::from_str(json)?;

    if file.version > EXPORT_VERSION {
        return Err(AppError::Validation(format!(
            "This file was created by a newer version of ClipX (file format {}, this build reads {}). Update ClipX and try again.",
            file.version, EXPORT_VERSION
        )));
    }

    let mut session_ids = HashSet::new();
    for s in &file.sessions {
        if s.id <= 0 {
            return Err(AppError::Validation(format!("Invalid session id {}", s.id)));
        }
        if !session_ids.insert(s.id) {
            return Err(AppError::Validation(format!("Duplicate session id {}", s.id)));
        }
        if s.name.trim().is_empty() {
            return Err(AppError::Validation(format!("Session {} has an empty name", s.id)));
        }
    }

    // The app assumes a single global session exists; without it the Pinned tab
    // has nowhere to put items and nothing recreates it until the next restart.
    let global_count = file.sessions.iter().filter(|s| s.is_global).count();
    if global_count != 1 {
        return Err(AppError::Validation(format!(
            "File must contain exactly one global session, found {global_count}"
        )));
    }

    let mut history_ids = HashSet::new();
    for h in &file.clipboard_history {
        if !history_ids.insert(h.id) {
            return Err(AppError::Validation(format!("Duplicate history id {}", h.id)));
        }
        validate_content(&h.content, "History")?;
    }

    let mut pinned_ids = HashSet::new();
    let mut pinned_keys = HashSet::new();
    for p in &file.clipboard_pinned {

        if !pinned_ids.insert(p.id) {
            return Err(AppError::Validation(format!("Duplicate pinned id {}", p.id)));
        }

        if !session_ids.contains(&p.session_id) {
            return Err(AppError::Validation(format!(
                "Pinned item {} references session {}, which is not in the file",
                p.id, p.session_id
            )));
        }

        if !pinned_keys.insert((p.content.as_str(), p.session_id)) {
            return Err(AppError::Validation(format!(
                "Duplicate pinned item in session {}",
                p.session_id
            )));
        }

        validate_content(&p.content, "Pinned item")?;
        if let Some(d) = &p.description {
            if d.len() > MAX_DESC_BYTES {
                return Err(AppError::Validation(format!(
                    "Pinned description exceeds {MAX_DESC_BYTES} bytes"
                )));
            }
        }
    }

    Ok(file)
}

fn validate_content(content: &str, label: &str) -> Result<(), AppError> {
    if content.is_empty() {
        return Err(AppError::Validation(format!("{label} has empty content")));
    }
    if content.len() > MAX_CLIP_BYTES {
        return Err(AppError::Validation(format!(
            "{label} exceeds {MAX_CLIP_BYTES} bytes"
        )));
    }
    Ok(())
}

/// Keep the newest `limit` history rows and drop the rest.
///
/// `history_limit` means "this is how much history I want", so a backup holding
/// more than that is trimmed at import time. Letting the monitor trim later
/// instead would make thousands of rows vanish at an unrelated moment.
fn truncate_history(file: &mut ExportFile, limit: usize) {
    if file.clipboard_history.len() <= limit {
        return;
    }
    file.clipboard_history
        .sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
    file.clipboard_history.truncate(limit);
}

/// Wipe the three data tables and insert the file's rows verbatim, in one
/// transaction. Any error rolls back and leaves the database as it was.
fn write_all(conn: &mut Connection, file: &ExportFile) -> Result<(), AppError> {

    let tx = conn.transaction()?;

    tx.execute("DELETE FROM clipboard_pinned", [])?;
    tx.execute("DELETE FROM clipboard_history", [])?;
    tx.execute("DELETE FROM sessions", [])?;

    for s in &file.sessions {
        tx.execute(
            "INSERT INTO sessions (id, name, is_global, is_active, sort_order, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, COALESCE(NULLIF(?6, ''), CURRENT_TIMESTAMP))",
            rusqlite::params![
                s.id,
                s.name,
                s.is_global as i64,
                s.is_active as i64,
                s.sort_order,
                s.created_at
            ],
        )?;
    }

    for h in &file.clipboard_history {
        tx.execute(
            "INSERT INTO clipboard_history (id, content, created_at) \
             VALUES (?1, ?2, COALESCE(NULLIF(?3, ''), CURRENT_TIMESTAMP))",
            rusqlite::params![h.id, h.content, h.created_at],
        )?;
    }

    for p in &file.clipboard_pinned {
        tx.execute(
            "INSERT INTO clipboard_pinned \
                (id, content, session_id, created_at, sort_order, description, hidden) \
             VALUES (?1, ?2, ?3, COALESCE(NULLIF(?4, ''), CURRENT_TIMESTAMP), ?5, ?6, ?7)",
            rusqlite::params![
                p.id,
                p.content,
                p.session_id,
                p.created_at,
                p.sort_order,
                p.description,
                p.hidden as i64
            ],
        )?;
    }

    // A file with no active session would leave the Sessions tab with nothing
    // selected, so fall back to the global one.
    tx.execute(
        "UPDATE sessions SET is_active = 1 \
         WHERE is_global = 1 AND NOT EXISTS (SELECT 1 FROM sessions WHERE is_active = 1)",
        [],
    )?;

    tx.commit()?;

    Ok(())
}

fn history_limit(state: &State<AppState>) -> Result<usize, AppError> {
    state
        .settings
        .lock()
        .map(|s| s.history_limit as usize)
        .map_err(|e| AppError::State(format!("settings mutex poisoned: {e}")))
}

#[tauri::command]
pub fn export_data(path: String, state: State<AppState>) -> Result<ExportSummary, AppError> {

    let file = {
        let conn = lock_db(&state)?;
        read_all(&conn)?
    };

    let summary = ExportSummary {
        history: file.clipboard_history.len(),
        sessions: file.sessions.len(),
        pinned: file.clipboard_pinned.len(),
    };

    std::fs::write(&path, serde_json::to_string_pretty(&file)?)?;

    log::info!(
        "exported {} history, {} sessions, {} pinned items",
        summary.history,
        summary.sessions,
        summary.pinned
    );

    Ok(summary)
}

#[tauri::command]
pub fn preview_import(path: String, state: State<AppState>) -> Result<ImportPreview, AppError> {

    let file = parse_export(&std::fs::read_to_string(&path)?)?;
    let limit = history_limit(&state)?;

    let conn = lock_db(&state)?;

    Ok(ImportPreview {
        version: file.version,
        exported_at: file.exported_at,
        file_history: file.clipboard_history.len(),
        history_to_import: file.clipboard_history.len().min(limit),
        file_sessions: file.sessions.len(),
        file_pinned: file.clipboard_pinned.len(),
        current_history: conn.query_row("SELECT COUNT(*) FROM clipboard_history", [], |r| r.get(0))?,
        current_sessions: conn.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))?,
        current_pinned: conn.query_row("SELECT COUNT(*) FROM clipboard_pinned", [], |r| r.get(0))?,
    })
}

#[tauri::command]
pub fn import_data(path: String, state: State<AppState>) -> Result<ExportSummary, AppError> {

    // Re-read and re-validate rather than trusting anything carried back from
    // the preview call, so the file on disk is always the thing that lands.
    let mut file = parse_export(&std::fs::read_to_string(&path)?)?;

    truncate_history(&mut file, history_limit(&state)?);

    let summary = ExportSummary {
        history: file.clipboard_history.len(),
        sessions: file.sessions.len(),
        pinned: file.clipboard_pinned.len(),
    };

    let mut conn = lock_db(&state)?;
    write_all(&mut conn, &file)?;

    log::info!(
        "imported {} history, {} sessions, {} pinned items",
        summary.history,
        summary.sessions,
        summary.pinned
    );

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::init_db(&mut conn).unwrap();
        conn
    }

    fn seeded() -> Connection {
        let conn = setup();
        conn.execute("INSERT INTO clipboard_history (content) VALUES ('one')", []).unwrap();
        conn.execute("INSERT INTO clipboard_history (content) VALUES ('two')", []).unwrap();
        conn.execute(
            "INSERT INTO clipboard_pinned (content, session_id, description, sort_order) \
             VALUES ('kept', 1, 'a note', 3)",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn roundtrip_preserves_rows() {
        let source = seeded();
        let exported = read_all(&source).unwrap();
        let json = serde_json::to_string(&exported).unwrap();

        let mut target = setup();
        let parsed = parse_export(&json).unwrap();
        write_all(&mut target, &parsed).unwrap();

        let history: Vec<(i64, String)> = target
            .prepare("SELECT id, content FROM clipboard_history ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(history, vec![(1, "one".into()), (2, "two".into())]);

        let (id, content, session_id, description, sort_order): (i64, String, i64, String, i64) =
            target
                .query_row(
                    "SELECT id, content, session_id, description, sort_order FROM clipboard_pinned",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                )
                .unwrap();
        assert_eq!((id, content, session_id, description, sort_order), (1, "kept".into(), 1, "a note".into(), 3));
    }

    #[test]
    fn import_replaces_existing_rows() {
        let source = seeded();
        let json = serde_json::to_string(&read_all(&source).unwrap()).unwrap();

        let mut target = setup();
        target.execute("INSERT INTO clipboard_history (content) VALUES ('stale')", []).unwrap();
        target
            .execute("INSERT INTO sessions (name, sort_order) VALUES ('Extra', 9)", [])
            .unwrap();

        write_all(&mut target, &parse_export(&json).unwrap()).unwrap();

        let stale: i64 = target
            .query_row("SELECT COUNT(*) FROM clipboard_history WHERE content = 'stale'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(stale, 0, "replace must not leave pre-existing history behind");

        let sessions: i64 = target.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0)).unwrap();
        assert_eq!(sessions, 1, "replace must not leave pre-existing sessions behind");
    }

    #[test]
    fn newer_version_is_refused() {
        let json = r#"{"version":99,"exported_at":"","sessions":[{"id":1,"name":"F","is_global":true}]}"#;
        let err = parse_export(json).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        assert!(err.to_string().contains("newer version"));
    }

    #[test]
    fn older_version_is_accepted_with_defaults() {
        let json = r#"{"version":1,"exported_at":"2026-01-01T00:00:00Z",
            "sessions":[{"id":1,"name":"Favorites","is_global":true}],
            "clipboard_pinned":[{"id":1,"content":"x","session_id":1}]}"#;
        let file = parse_export(json).unwrap();
        assert!(file.clipboard_history.is_empty());
        assert_eq!(file.clipboard_pinned[0].sort_order, 0);
        assert!(file.clipboard_pinned[0].description.is_none());
        assert!(!file.clipboard_pinned[0].hidden);
    }

    #[test]
    fn missing_global_session_is_refused() {
        let json = r#"{"version":1,"exported_at":"","sessions":[{"id":1,"name":"F"}]}"#;
        assert!(matches!(parse_export(json), Err(AppError::Validation(_))));
    }

    #[test]
    fn two_global_sessions_are_refused() {
        let json = r#"{"version":1,"exported_at":"","sessions":[
            {"id":1,"name":"A","is_global":true},{"id":2,"name":"B","is_global":true}]}"#;
        assert!(matches!(parse_export(json), Err(AppError::Validation(_))));
    }

    #[test]
    fn dangling_pinned_session_is_refused() {
        let json = r#"{"version":1,"exported_at":"",
            "sessions":[{"id":1,"name":"F","is_global":true}],
            "clipboard_pinned":[{"id":1,"content":"x","session_id":7}]}"#;
        let err = parse_export(json).unwrap_err();
        assert!(err.to_string().contains("references session 7"));
    }

    #[test]
    fn duplicate_pinned_key_is_refused() {
        let json = r#"{"version":1,"exported_at":"",
            "sessions":[{"id":1,"name":"F","is_global":true}],
            "clipboard_pinned":[
                {"id":1,"content":"x","session_id":1},
                {"id":2,"content":"x","session_id":1}]}"#;
        assert!(matches!(parse_export(json), Err(AppError::Validation(_))));
    }

    #[test]
    fn corrupt_json_is_refused() {
        assert!(matches!(parse_export("{not json"), Err(AppError::Json(_))));
    }

    #[test]
    fn failed_write_leaves_database_untouched() {
        let mut target = seeded();

        // session_id passes the file-level checks but the row collides on the
        // primary key, so the insert fails partway through the transaction.
        let mut file = read_all(&target).unwrap();
        file.clipboard_history.push(HistoryRow {
            id: 1,
            content: "collides".into(),
            created_at: String::new(),
        });

        assert!(write_all(&mut target, &file).is_err());

        let rows: Vec<String> = target
            .prepare("SELECT content FROM clipboard_history ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(rows, vec!["one".to_string(), "two".to_string()]);
    }

    #[test]
    fn history_truncates_to_limit_keeping_newest() {
        let mut file = ExportFile {
            version: EXPORT_VERSION,
            exported_at: String::new(),
            clipboard_history: vec![
                HistoryRow { id: 1, content: "oldest".into(), created_at: "2026-01-01 00:00:00".into() },
                HistoryRow { id: 2, content: "middle".into(), created_at: "2026-01-02 00:00:00".into() },
                HistoryRow { id: 3, content: "newest".into(), created_at: "2026-01-03 00:00:00".into() },
            ],
            sessions: vec![],
            clipboard_pinned: vec![],
        };

        truncate_history(&mut file, 2);

        let kept: Vec<&str> = file.clipboard_history.iter().map(|h| h.content.as_str()).collect();
        assert_eq!(kept, vec!["newest", "middle"]);
    }

    #[test]
    fn history_under_limit_is_left_alone() {
        let source = seeded();
        let mut file = read_all(&source).unwrap();
        truncate_history(&mut file, 500);
        assert_eq!(file.clipboard_history.len(), 2);
    }

    #[test]
    fn inactive_sessions_fall_back_to_global() {
        let json = r#"{"version":1,"exported_at":"",
            "sessions":[{"id":1,"name":"Favorites","is_global":true,"is_active":false}]}"#;
        let mut target = setup();
        write_all(&mut target, &parse_export(json).unwrap()).unwrap();

        let active: i64 = target
            .query_row("SELECT COUNT(*) FROM sessions WHERE is_active = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(active, 1);
    }

    #[test]
    fn oversized_content_is_refused() {
        let big = "x".repeat(MAX_CLIP_BYTES + 1);
        let json = format!(
            r#"{{"version":1,"exported_at":"","sessions":[{{"id":1,"name":"F","is_global":true}}],
                "clipboard_history":[{{"id":1,"content":"{big}"}}]}}"#
        );
        assert!(matches!(parse_export(&json), Err(AppError::Validation(_))));
    }
}
