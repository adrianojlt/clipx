use crate::commands::lock_db;
use crate::error::AppError;
use crate::{AppState, PinnedItem, MAX_CLIP_BYTES};
use rusqlite::OptionalExtension;
use tauri::State;

#[tauri::command]
pub fn get_pinned(state: State<AppState>) -> Result<Vec<PinnedItem>, AppError> {

    let conn = lock_db(&state)?;

    let mut stmt = conn.prepare(
        "SELECT id, content, COALESCE(description, content), COALESCE(hidden, 0), created_at \
         FROM clipboard_pinned \
         WHERE session_id = (SELECT id FROM sessions WHERE is_active = 1 LIMIT 1) \
         ORDER BY sort_order ASC",
    )?;

    let items = stmt
        .query_map([], |row| {
            Ok(PinnedItem {
                id: row.get(0)?,
                content: row.get(1)?,
                description: row.get(2)?,
                hidden: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

pub(crate) fn pin_item_impl(conn: &mut rusqlite::Connection, content: &str) -> Result<(), AppError> {

    let tx = conn.transaction()?;

    tx.execute(
        "UPDATE clipboard_pinned SET sort_order = sort_order + 1 \
         WHERE session_id = (SELECT id FROM sessions WHERE is_global = 1 LIMIT 1)",
        [],
    )?;

    tx.execute(
        "INSERT OR IGNORE INTO clipboard_pinned (content, session_id, description, sort_order) \
         VALUES (?1, (SELECT id FROM sessions WHERE is_global = 1 LIMIT 1), ?1, 0)",
        rusqlite::params![content],
    )?;

    if tx.changes() == 0 {
        tx.execute(
            "UPDATE clipboard_pinned SET sort_order = sort_order - 1 \
             WHERE session_id = (SELECT id FROM sessions WHERE is_global = 1 LIMIT 1)",
            [],
        )?;
    }

    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn pin_item(content: String, state: State<AppState>) -> Result<(), AppError> {

    if content.len() > MAX_CLIP_BYTES {
        return Err(AppError::Validation(format!(
            "Content exceeds {MAX_CLIP_BYTES}-byte limit"
        )));
    }

    let mut conn = lock_db(&state)?;
    pin_item_impl(&mut conn, &content)
}

#[tauri::command]
pub fn get_global_pinned(state: State<AppState>) -> Result<Vec<PinnedItem>, AppError> {

    let conn = lock_db(&state)?;

    let mut stmt = conn.prepare(
        "SELECT id, content, COALESCE(description, content), COALESCE(hidden, 0), created_at \
         FROM clipboard_pinned \
         WHERE session_id = (SELECT id FROM sessions WHERE is_global = 1 LIMIT 1) \
         ORDER BY sort_order ASC",
    )?;

    let items = stmt
        .query_map([], |row| {
            Ok(PinnedItem {
                id: row.get(0)?,
                content: row.get(1)?,
                description: row.get(2)?,
                hidden: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

#[tauri::command]
pub fn reorder_pinned(items: Vec<i64>, state: State<AppState>) -> Result<(), AppError> {

    if items.len() > 500 {
        return Err(AppError::Validation("Too many items in reorder_pinned".into()));
    }

    let mut conn = lock_db(&state)?;
    let tx = conn.transaction()?;

    let mut expected: Vec<i64> = {
        let mut stmt = tx.prepare(
            "SELECT id FROM clipboard_pinned \
             WHERE session_id = (SELECT id FROM sessions WHERE is_active = 1 LIMIT 1) \
             ORDER BY id",
        )?;
        let rows: Result<Vec<i64>, _> = stmt.query_map([], |row| row.get(0))?.collect();
        rows?
    };

    let mut received = items.clone();

    expected.sort_unstable();
    received.sort_unstable();
    received.dedup();

    if received != expected {
        return Err(AppError::Validation(
            "reorder_pinned: IDs do not match current pinned set".into(),
        ));
    }

    for (index, id) in items.iter().enumerate() {
        tx.execute(
            "UPDATE clipboard_pinned SET sort_order = ?1 WHERE id = ?2",
            [index as i64, *id],
        )?;
    }

    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn update_pinned_description(
    id: i64,
    description: String,
    state: State<AppState>,
) -> Result<(), AppError> {

    if id <= 0 {
        return Err(AppError::Validation("Invalid id".into()));
    }

    if description.len() > crate::MAX_DESC_BYTES {
        return Err(AppError::Validation(format!(
            "Description exceeds {}-byte limit",
            crate::MAX_DESC_BYTES
        )));
    }

    let conn = lock_db(&state)?;
    let n = conn.execute(
        "UPDATE clipboard_pinned SET description = ?1 WHERE id = ?2",
        rusqlite::params![description, id],
    )?;

    if n == 0 {
        return Err(AppError::NotFound(id));
    }

    Ok(())
}

#[tauri::command]
pub fn unpin_item(id: i64, state: State<AppState>) -> Result<(), AppError> {

    if id <= 0 {
        return Err(AppError::Validation("Invalid id".into()));
    }

    let conn = lock_db(&state)?;
    let n = conn.execute("DELETE FROM clipboard_pinned WHERE id = ?1", [id])?;

    if n == 0 {
        return Err(AppError::NotFound(id));
    }

    Ok(())
}

#[tauri::command]
pub fn toggle_pinned_hidden(id: i64, state: State<AppState>) -> Result<bool, AppError> {

    if id <= 0 {
        return Err(AppError::Validation("Invalid id".into()));
    }

    let conn = lock_db(&state)?;

    let new_val: Option<bool> = conn
        .query_row(
            "UPDATE clipboard_pinned \
             SET hidden = CASE WHEN COALESCE(hidden, 0) = 0 THEN 1 ELSE 0 END \
             WHERE id = ?1 \
             RETURNING hidden",
            [id],
            |row| row.get::<_, i64>(0).map(|v| v != 0),
        )
        .optional()?;

    match new_val {
        Some(v) => Ok(v),
        None => Err(AppError::NotFound(id)),
    }
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
    fn pin_item_success() {
        let mut conn = setup();
        pin_item_impl(&mut conn, "hello").unwrap();
        let row: (String, i64) = conn
            .query_row(
                "SELECT content, sort_order FROM clipboard_pinned WHERE content = 'hello'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(row.0, "hello");
        assert_eq!(row.1, 0);
    }

    #[test]
    fn pin_item_duplicate_does_not_increment_sort_order() {
        let mut conn = setup();
        pin_item_impl(&mut conn, "hello").unwrap();
        pin_item_impl(&mut conn, "world").unwrap();
        // "world" inserted second: sort_order should be 0 ("hello" bumped to 1)
        let world_order: i64 = conn
            .query_row(
                "SELECT sort_order FROM clipboard_pinned WHERE content = 'world'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(world_order, 0);

        // pin "world" again (duplicate) - sort_order of existing items must not change
        pin_item_impl(&mut conn, "world").unwrap();
        let hello_order: i64 = conn
            .query_row(
                "SELECT sort_order FROM clipboard_pinned WHERE content = 'hello'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // rollback restored sort_order after failed insert, so "hello" stays at 1
        assert_eq!(hello_order, 1);
    }
}
