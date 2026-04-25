use crate::commands::lock_db;
use crate::error::AppError;
use crate::{AppState, PinnedItem};
use rusqlite::OptionalExtension;
use tauri::State;

#[tauri::command]
pub fn get_pinned(state: State<AppState>) -> Result<Vec<PinnedItem>, AppError> {
    let conn = lock_db(&state)?;

    let mut stmt = conn.prepare(
        "SELECT id, content, COALESCE(description, content), COALESCE(hidden, 0), created_at \
         FROM clipboard_pinned ORDER BY sort_order ASC",
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
pub fn pin_item(content: String, state: State<AppState>) -> Result<(), AppError> {
    let mut conn = lock_db(&state)?;
    let tx = conn.transaction()?;

    let exists = tx
        .query_row(
            "SELECT 1 FROM clipboard_pinned WHERE content = ?1",
            [&content],
            |_| Ok(()),
        )
        .optional()?
        .is_some();

    if !exists {
        tx.execute(
            "UPDATE clipboard_pinned SET sort_order = sort_order + 1",
            [],
        )?;
        tx.execute(
            "INSERT INTO clipboard_pinned (content, description, sort_order) VALUES (?1, ?1, 0)",
            rusqlite::params![content],
        )?;
    }

    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn reorder_pinned(items: Vec<i64>, state: State<AppState>) -> Result<(), AppError> {
    let mut conn = lock_db(&state)?;
    let tx = conn.transaction()?;

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
    let conn = lock_db(&state)?;
    conn.execute(
        "UPDATE clipboard_pinned SET description = ?1 WHERE id = ?2",
        rusqlite::params![description, id],
    )?;
    Ok(())
}

#[tauri::command]
pub fn unpin_item(id: i64, state: State<AppState>) -> Result<(), AppError> {
    let conn = lock_db(&state)?;
    conn.execute("DELETE FROM clipboard_pinned WHERE id = ?1", [id])?;
    Ok(())
}

#[tauri::command]
pub fn toggle_pinned_hidden(id: i64, state: State<AppState>) -> Result<bool, AppError> {
    let conn = lock_db(&state)?;

    let current: bool = conn.query_row(
        "SELECT COALESCE(hidden, 0) FROM clipboard_pinned WHERE id = ?1",
        [id],
        |row| row.get::<_, i64>(0).map(|v| v != 0),
    )?;

    let new_val = i64::from(!current);

    conn.execute(
        "UPDATE clipboard_pinned SET hidden = ?1 WHERE id = ?2",
        rusqlite::params![new_val, id],
    )?;

    Ok(!current)
}
