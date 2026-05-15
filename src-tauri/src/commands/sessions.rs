use crate::commands::lock_db;
use crate::error::AppError;
use crate::{AppState, Session, MAX_CLIP_BYTES};
use tauri::State;

#[tauri::command]
pub fn get_sessions(state: State<AppState>) -> Result<Vec<Session>, AppError> {

    let conn = lock_db(&state)?;

    let mut stmt = conn.prepare(
        "SELECT id, name, is_global, is_active, sort_order FROM sessions ORDER BY sort_order ASC",
    )?;

    let items = stmt
        .query_map([], |row| {
            Ok(Session {
                id: row.get(0)?,
                name: row.get(1)?,
                is_global: row.get::<_, i64>(2)? != 0,
                is_active: row.get::<_, i64>(3)? != 0,
                sort_order: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

#[tauri::command]
pub fn create_session(name: String, state: State<AppState>) -> Result<Session, AppError> {

    if name.trim().is_empty() {
        return Err(AppError::Validation("Session name cannot be empty".into()));
    }

    if name.len() > crate::MAX_DESC_BYTES {
        return Err(AppError::Validation(format!(
            "Session name exceeds {}-byte limit",
            crate::MAX_DESC_BYTES
        )));
    }

    let conn = lock_db(&state)?;

    let max_sort: i64 = conn.query_row(
        "SELECT COALESCE(MAX(sort_order), 0) FROM sessions",
        [],
        |row| row.get(0),
    )?;

    conn.execute(
        "INSERT INTO sessions (name, is_global, is_active, sort_order) VALUES (?1, 0, 0, ?2)",
        rusqlite::params![name.trim(), max_sort + 1],
    )?;

    let id = conn.last_insert_rowid();

    let session = conn.query_row(
        "SELECT id, name, is_global, is_active, sort_order FROM sessions WHERE id = ?1",
        [id],
        |row| {
            Ok(Session {
                id: row.get(0)?,
                name: row.get(1)?,
                is_global: row.get::<_, i64>(2)? != 0,
                is_active: row.get::<_, i64>(3)? != 0,
                sort_order: row.get(4)?,
            })
        },
    )?;

    Ok(session)
}

#[tauri::command]
pub fn delete_session(id: i64, state: State<AppState>) -> Result<(), AppError> {

    if id <= 0 {
        return Err(AppError::Validation("Invalid id".into()));
    }

    let mut conn = lock_db(&state)?;

    let is_global: i64 = conn
        .query_row(
            "SELECT is_global FROM sessions WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .map_err(|_| AppError::NotFound(id))?;

    if is_global != 0 {
        return Err(AppError::Validation("Cannot delete the Global session".into()));
    }

    let was_active: i64 = conn.query_row(
        "SELECT is_active FROM sessions WHERE id = ?1",
        [id],
        |row| row.get(0),
    )?;

    let tx = conn.transaction()?;

    tx.execute("DELETE FROM clipboard_pinned WHERE session_id = ?1", [id])?;
    tx.execute("DELETE FROM sessions WHERE id = ?1", [id])?;

    if was_active != 0 {
        tx.execute("UPDATE sessions SET is_active = 0", [])?;
        tx.execute(
            "UPDATE sessions SET is_active = 1 WHERE is_global = 1",
            [],
        )?;
    }

    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn activate_session(id: i64, state: State<AppState>) -> Result<(), AppError> {

    if id <= 0 {
        return Err(AppError::Validation("Invalid id".into()));
    }

    let mut conn = lock_db(&state)?;
    let tx = conn.transaction()?;

    tx.execute("UPDATE sessions SET is_active = 0", [])?;
    let n = tx.execute("UPDATE sessions SET is_active = 1 WHERE id = ?1", [id])?;

    if n == 0 {
        return Err(AppError::NotFound(id));
    }

    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn reorder_sessions(items: Vec<i64>, state: State<AppState>) -> Result<(), AppError> {

    if items.len() > 500 {
        return Err(AppError::Validation("Too many items in reorder_sessions".into()));
    }

    let mut conn = lock_db(&state)?;
    let tx = conn.transaction()?;

    let mut expected: Vec<i64> = {
        let mut stmt = tx.prepare("SELECT id FROM sessions ORDER BY id")?;
        let rows: Result<Vec<i64>, _> = stmt.query_map([], |row| row.get(0))?.collect();
        rows?
    };

    let mut received = items.clone();
    expected.sort_unstable();
    received.sort_unstable();
    received.dedup();

    if received != expected {
        return Err(AppError::Validation(
            "reorder_sessions: IDs do not match current session set".into(),
        ));
    }

    for (index, id) in items.iter().enumerate() {
        tx.execute(
            "UPDATE sessions SET sort_order = ?1 WHERE id = ?2",
            [index as i64, *id],
        )?;
    }

    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn pin_item_to_session(
    content: String,
    session_id: i64,
    state: State<AppState>,
) -> Result<(), AppError> {

    if content.len() > MAX_CLIP_BYTES {
        return Err(AppError::Validation(format!(
            "Content exceeds {MAX_CLIP_BYTES}-byte limit"
        )));
    }

    if session_id <= 0 {
        return Err(AppError::Validation("Invalid session_id".into()));
    }

    let mut conn = lock_db(&state)?;
    let tx = conn.transaction()?;

    tx.execute(
        "UPDATE clipboard_pinned SET sort_order = sort_order + 1 WHERE session_id = ?1",
        [session_id],
    )?;

    tx.execute(
        "INSERT OR IGNORE INTO clipboard_pinned (content, session_id, description, sort_order) \
         VALUES (?1, ?2, ?1, 0)",
        rusqlite::params![content, session_id],
    )?;

    if tx.changes() == 0 {
        tx.execute(
            "UPDATE clipboard_pinned SET sort_order = sort_order - 1 WHERE session_id = ?1",
            [session_id],
        )?;
    }

    tx.commit()?;
    Ok(())
}
