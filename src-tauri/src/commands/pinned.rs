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

    if content.len() > MAX_CLIP_BYTES {
        return Err(AppError::Validation(format!(
            "Content exceeds {MAX_CLIP_BYTES}-byte limit"
        )));
    }

    let mut conn = lock_db(&state)?;
    let tx = conn.transaction()?;

    tx.execute("UPDATE clipboard_pinned SET sort_order = sort_order + 1", [])?;
    tx.execute(
        "INSERT OR IGNORE INTO clipboard_pinned (content, description, sort_order) VALUES (?1, ?1, 0)",
        rusqlite::params![content],
    )?;

    if tx.changes() == 0 {
        // Already pinned - undo the sort_order bump
        tx.execute("UPDATE clipboard_pinned SET sort_order = sort_order - 1", [])?;
    }

    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn reorder_pinned(items: Vec<i64>, state: State<AppState>) -> Result<(), AppError> {

    if items.len() > 500 {
        return Err(AppError::Validation("Too many items in reorder_pinned".into()));
    }

    let mut conn = lock_db(&state)?;
    let tx = conn.transaction()?;

    let mut expected: Vec<i64> = {
        let mut stmt = tx.prepare("SELECT id FROM clipboard_pinned ORDER BY id")?;
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
    
    if description.len() > MAX_CLIP_BYTES {
        return Err(AppError::Validation("Description too large".into()));
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

    let conn = lock_db(&state)?;
    let n = conn.execute("DELETE FROM clipboard_pinned WHERE id = ?1", [id])?;

    if n == 0 {
        return Err(AppError::NotFound(id));
    }

    Ok(())
}

#[tauri::command]
pub fn toggle_pinned_hidden(id: i64, state: State<AppState>) -> Result<bool, AppError> {

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
