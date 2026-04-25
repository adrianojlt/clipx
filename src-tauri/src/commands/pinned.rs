use crate::error::AppError;
use crate::{AppState, PinnedItem};
use tauri::State;

#[tauri::command]
pub fn get_pinned(state: State<AppState>) -> Result<Vec<PinnedItem>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Settings(e.to_string()))?;

    let mut stmt = conn
        .prepare("SELECT id, content, COALESCE(description, content), COALESCE(hidden, 0), created_at FROM clipboard_pinned ORDER BY sort_order ASC")?;

    let rows = stmt.query_map([], |row| {
        Ok(PinnedItem {
            id: row.get(0)?,
            content: row.get(1)?,
            description: row.get(2)?,
            hidden: row.get::<_, i64>(3)? != 0,
            created_at: row.get(4)?,
        })
    })?;

    let mut items = Vec::new();

    for item in rows {
        items.push(item?);
    }

    Ok(items)
}

#[tauri::command]
pub fn pin_item(content: String, state: State<AppState>) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Settings(e.to_string()))?;

    conn.execute(
        "UPDATE clipboard_pinned SET sort_order = sort_order + 1",
        [],
    )?;

    conn.execute("INSERT OR IGNORE INTO clipboard_pinned (content, description, sort_order) VALUES (?, ?, 0)", rusqlite::params![content, content])?;

    Ok(())
}

#[tauri::command]
pub fn reorder_pinned(items: Vec<i64>, state: State<AppState>) -> Result<(), AppError> {
    let mut conn = state
        .db
        .lock()
        .map_err(|e| AppError::Settings(e.to_string()))?;
    let tx = conn.transaction()?;

    for (index, id) in items.iter().enumerate() {
        tx.execute(
            "UPDATE clipboard_pinned SET sort_order = ? WHERE id = ?",
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
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Settings(e.to_string()))?;

    conn.execute(
        "UPDATE clipboard_pinned SET description = ? WHERE id = ?",
        rusqlite::params![description, id],
    )?;

    Ok(())
}

#[tauri::command]
pub fn unpin_item(id: i64, state: State<AppState>) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Settings(e.to_string()))?;

    conn.execute("DELETE FROM clipboard_pinned WHERE id = ?", [id])?;

    Ok(())
}

#[tauri::command]
pub fn toggle_pinned_hidden(id: i64, state: State<AppState>) -> Result<bool, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Settings(e.to_string()))?;

    let current: bool = conn.query_row(
        "SELECT COALESCE(hidden, 0) FROM clipboard_pinned WHERE id = ?",
        [id],
        |row| row.get::<_, i64>(0).map(|v| v != 0),
    )?;

    let new_val = if current { 0 } else { 1 };

    conn.execute(
        "UPDATE clipboard_pinned SET hidden = ? WHERE id = ?",
        rusqlite::params![new_val, id],
    )?;

    Ok(!current)
}
