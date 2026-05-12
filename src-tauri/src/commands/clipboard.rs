use crate::commands::lock_db;
use crate::error::AppError;
use crate::{AppState, ClipboardItem};
use tauri::{AppHandle, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

#[tauri::command]
pub fn get_history(state: State<AppState>) -> Result<Vec<ClipboardItem>, AppError> {

    let limit = state
        .settings
        .lock()
        .map(|s| s.history_limit)
        .map_err(|e| AppError::State(format!("settings mutex poisoned: {e}")))?
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

pub(crate) fn read_clipboard_on_main_thread(app: &AppHandle) -> Result<String, AppError> {

    let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
    let app_inner = app.clone();

    app.run_on_main_thread(move || {
        let _ = tx.send(app_inner.clipboard().read_text().map_err(|e| e.to_string()));
    })
    .map_err(|e| AppError::Window(format!("dispatch to main thread failed: {e}")))?;

    rx.recv_timeout(std::time::Duration::from_secs(2))
        .map_err(|_| AppError::Window("clipboard read timed out".into()))?
        .map_err(|e| AppError::Window(format!("clipboard read: {e}")))
}

#[tauri::command]
pub fn get_clipboard(app: AppHandle) -> Result<String, AppError> {
    read_clipboard_on_main_thread(&app)
}

#[tauri::command]
pub fn delete_history_item(id: i64, state: State<AppState>) -> Result<(), AppError> {

    if id <= 0 {
        return Err(AppError::Validation("Invalid id".into()));
    }

    let conn = lock_db(&state)?;
    let n = conn.execute("DELETE FROM clipboard_history WHERE id = ?1", [id])?;

    if n == 0 {
        return Err(AppError::NotFound(id));
    }

    Ok(())
}
