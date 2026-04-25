use crate::db::db_path;
use crate::error::AppError;
use crate::{AppState, ClipboardItem};
use rusqlite::Connection;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn get_history(app: AppHandle) -> Result<Vec<ClipboardItem>, AppError> {
    let limit = app
        .state::<AppState>()
        .history_limit
        .lock()
        .map(|l| *l)
        .unwrap_or(20) as i64;

    let conn = Connection::open(db_path(&app)?)?;

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
pub fn delete_history_item(id: i64, app: AppHandle) -> Result<(), AppError> {
    let conn = Connection::open(db_path(&app)?)?;
    conn.execute("DELETE FROM clipboard_history WHERE id = ?", [id])?;
    Ok(())
}
