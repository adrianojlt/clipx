use crate::commands::lock_db;
use crate::error::AppError;
use crate::{AppState, ClipboardItem};
use tauri::State;

#[tauri::command]
pub fn get_history(state: State<AppState>) -> Result<Vec<ClipboardItem>, AppError> {
    let limit = state
        .history_limit
        .lock()
        .map(|l| *l)
        .map_err(|e| AppError::State(format!("history_limit mutex poisoned: {e}")))?
        as i64;

    let conn = lock_db(&state)?;

    let mut stmt = conn.prepare(
        "SELECT id, content, created_at FROM clipboard_history ORDER BY created_at DESC LIMIT ?1",
    )?;

    let items = stmt
        .query_map([limit], |row| {
            Ok(ClipboardItem {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

#[tauri::command]
pub fn get_clipboard() -> Result<String, AppError> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| AppError::Window(format!("clipboard init: {e}")))?;
    clipboard
        .get_text()
        .map_err(|e| AppError::Window(format!("clipboard read: {e}")))
}

#[tauri::command]
pub fn delete_history_item(id: i64, state: State<AppState>) -> Result<(), AppError> {
    let conn = lock_db(&state)?;
    conn.execute("DELETE FROM clipboard_history WHERE id = ?1", [id])?;
    Ok(())
}
