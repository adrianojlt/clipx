use crate::commands::lock_db;
use crate::error::AppError;
use crate::{AppState, Session, MAX_CLIP_BYTES};
use tauri::State;

fn row_to_session(row: &rusqlite::Row) -> rusqlite::Result<Session> {
    Ok(Session {
        id: row.get(0)?,
        name: row.get(1)?,
        is_global: row.get::<_, i64>(2)? != 0,
        is_active: row.get::<_, i64>(3)? != 0,
        sort_order: row.get(4)?,
        item_count: row.get(5)?,
    })
}

#[tauri::command]
pub fn get_sessions(state: State<AppState>) -> Result<Vec<Session>, AppError> {

    let conn = lock_db(&state)?;

    let mut stmt = conn.prepare(
        "SELECT s.id, s.name, s.is_global, s.is_active, s.sort_order, COUNT(p.id) AS item_count \
         FROM sessions s \
         LEFT JOIN clipboard_pinned p ON p.session_id = s.id \
         GROUP BY s.id \
         ORDER BY s.sort_order ASC",
    )?;

    let items = stmt
        .query_map([], row_to_session)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

pub(crate) fn create_session_impl(conn: &rusqlite::Connection, name: &str) -> Result<Session, AppError> {

    if name.trim().is_empty() {
        return Err(AppError::Validation("Session name cannot be empty".into()));
    }

    if name.len() > crate::MAX_DESC_BYTES {
        return Err(AppError::Validation(format!(
            "Session name exceeds {}-byte limit",
            crate::MAX_DESC_BYTES
        )));
    }

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
        "SELECT s.id, s.name, s.is_global, s.is_active, s.sort_order, COUNT(p.id) AS item_count \
         FROM sessions s \
         LEFT JOIN clipboard_pinned p ON p.session_id = s.id \
         WHERE s.id = ?1 \
         GROUP BY s.id",
        [id],
        row_to_session,
    )?;

    Ok(session)
}

#[tauri::command]
pub fn create_session(name: String, state: State<AppState>) -> Result<Session, AppError> {
    let conn = lock_db(&state)?;
    create_session_impl(&conn, &name)
}

pub(crate) fn delete_session_impl(conn: &mut rusqlite::Connection, id: i64) -> Result<(), AppError> {

    if id <= 0 {
        return Err(AppError::Validation("Invalid id".into()));
    }

    let tx = conn.transaction()?;

    let (is_global, was_active): (i64, i64) = tx
        .query_row(
            "SELECT is_global, is_active FROM sessions WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(id),
            other => AppError::Db(other),
        })?;

    if is_global != 0 {
        return Err(AppError::Validation("Cannot delete the Global session".into()));
    }

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
pub fn delete_session(id: i64, state: State<AppState>) -> Result<(), AppError> {
    let mut conn = lock_db(&state)?;
    delete_session_impl(&mut conn, id)
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

pub(crate) fn reorder_sessions_impl(conn: &mut rusqlite::Connection, items: Vec<i64>) -> Result<(), AppError> {

    if items.len() > 500 {
        return Err(AppError::Validation("Too many items in reorder_sessions".into()));
    }

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
pub fn reorder_sessions(items: Vec<i64>, state: State<AppState>) -> Result<(), AppError> {
    let mut conn = lock_db(&state)?;
    reorder_sessions_impl(&mut conn, items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::init_db(&mut conn).unwrap();
        conn
    }

    #[test]
    fn create_session_empty_name_rejected() {
        let conn = setup();
        assert!(matches!(
            create_session_impl(&conn, ""),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            create_session_impl(&conn, "   "),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn create_session_success() {
        let conn = setup();
        let s = create_session_impl(&conn, "Work").unwrap();
        assert_eq!(s.name, "Work");
        assert!(!s.is_global);
        assert!(!s.is_active);
    }

    #[test]
    fn delete_session_global_rejected() {
        let mut conn = setup();
        // global session has id=1 from init_db
        assert!(matches!(
            delete_session_impl(&mut conn, 1),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn delete_session_not_found() {
        let mut conn = setup();
        assert!(matches!(
            delete_session_impl(&mut conn, 999),
            Err(AppError::NotFound(999))
        ));
    }

    #[test]
    fn delete_active_session_activates_global() {
        let mut conn = setup();
        let s = create_session_impl(&conn, "Work").unwrap();
        // activate the new session
        {
            let tx = conn.transaction().unwrap();
            tx.execute("UPDATE sessions SET is_active = 0", []).unwrap();
            tx.execute("UPDATE sessions SET is_active = 1 WHERE id = ?1", [s.id]).unwrap();
            tx.commit().unwrap();
        }
        delete_session_impl(&mut conn, s.id).unwrap();
        let global_active: i64 = conn
            .query_row(
                "SELECT is_active FROM sessions WHERE is_global = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(global_active, 1);
    }

    #[test]
    fn reorder_sessions_id_mismatch_rejected() {
        let mut conn = setup();
        // only global session (id=1) exists; passing [1, 99] is a mismatch
        assert!(matches!(
            reorder_sessions_impl(&mut conn, vec![1, 99]),
            Err(AppError::Validation(_))
        ));
    }
}

#[tauri::command]
pub fn pin_item_to_session(
    content: String,
    session_id: i64,
    description: Option<String>,
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

    let desc = description.unwrap_or_else(|| content.clone());

    let mut conn = lock_db(&state)?;
    let tx = conn.transaction()?;

    tx.execute(
        "UPDATE clipboard_pinned SET sort_order = sort_order + 1 WHERE session_id = ?1",
        [session_id],
    )?;

    tx.execute(
        "INSERT OR IGNORE INTO clipboard_pinned (content, session_id, description, sort_order) \
         VALUES (?1, ?2, ?3, 0)",
        rusqlite::params![content, session_id, desc],
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
