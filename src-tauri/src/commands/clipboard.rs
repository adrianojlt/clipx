use crate::error::AppError;
use crate::{AppState, ClipboardItem};
use tauri::State;

#[tauri::command]
pub fn get_history(state: State<AppState>) -> Result<Vec<ClipboardItem>, AppError> {
    let limit = state.history_limit.lock().map(|l| *l).unwrap_or(20) as i64;

    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Settings(e.to_string()))?;

    let mut stmt = conn.prepare(&format!(
        "SELECT id, content, created_at FROM clipboard_history ORDER BY created_at DESC LIMIT {}",
        limit
    ))?;

    let rows = stmt.query_map([], |row| {
        Ok(ClipboardItem {
            id: row.get(0)?,
            content: row.get(1)?,
            created_at: row.get(2)?,
        })
    })?;

    let mut items = Vec::new();

    for item in rows {
        items.push(item?);
    }

    Ok(items)
}

#[tauri::command]
pub fn get_clipboard() -> Result<String, AppError> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| AppError::Settings(e.to_string()))?;
    clipboard
        .get_text()
        .map_err(|e| AppError::Settings(e.to_string()))
}

#[tauri::command]
pub fn delete_history_item(id: i64, state: State<AppState>) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Settings(e.to_string()))?;
    conn.execute("DELETE FROM clipboard_history WHERE id = ?", [id])?;
    Ok(())
}
