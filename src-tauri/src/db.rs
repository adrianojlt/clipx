use crate::error::AppError;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub fn db_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Path(format!("Cannot resolve app data dir: {e}")))?
        .join("clipx.db"))
}

pub fn init_db(conn: &mut rusqlite::Connection) -> Result<(), AppError> {
    let tx = conn.transaction()?;

    tx.execute(
        "CREATE TABLE IF NOT EXISTS clipboard_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    tx.execute(
        "CREATE TABLE IF NOT EXISTS clipboard_pinned (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL UNIQUE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            sort_order INTEGER DEFAULT 0
        )",
        [],
    )?;

    tx.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_history_created \
            ON clipboard_history(created_at DESC); \
         CREATE INDEX IF NOT EXISTS idx_pinned_sort \
            ON clipboard_pinned(sort_order ASC);",
    )?;

    let cols: Vec<String> = tx
        .prepare("PRAGMA table_info(clipboard_pinned)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    let has = |name: &str| cols.iter().any(|c| c == name);

    if !has("sort_order") {
        tx.execute(
            "ALTER TABLE clipboard_pinned ADD COLUMN sort_order INTEGER DEFAULT 0",
            [],
        )?;

        let ids: Vec<i64> = tx
            .prepare("SELECT id FROM clipboard_pinned ORDER BY created_at DESC")?
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        for (index, id) in ids.iter().enumerate() {
            tx.execute(
                "UPDATE clipboard_pinned SET sort_order = ?1 WHERE id = ?2",
                [index as i64, *id],
            )?;
        }
    }

    if !has("description") {
        tx.execute(
            "ALTER TABLE clipboard_pinned ADD COLUMN description TEXT",
            [],
        )?;
        tx.execute(
            "UPDATE clipboard_pinned SET description = content WHERE description IS NULL",
            [],
        )?;
    }

    if !has("hidden") {
        tx.execute(
            "ALTER TABLE clipboard_pinned ADD COLUMN hidden INTEGER DEFAULT 0",
            [],
        )?;
    }

    tx.commit()?;
    Ok(())
}
