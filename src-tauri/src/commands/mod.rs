pub mod clipboard;
pub mod pinned;
pub mod settings;

use crate::error::AppError;
use crate::AppState;
use rusqlite::Connection;
use std::sync::MutexGuard;
use tauri::State;

pub(crate) fn lock_db<'a>(
    state: &'a State<'_, AppState>,
) -> Result<MutexGuard<'a, Connection>, AppError> {
    state
        .db
        .lock()
        .map_err(|e| AppError::State(format!("db mutex poisoned: {e}")))
}
