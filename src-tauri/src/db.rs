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

pub fn init_db(conn: &rusqlite::Connection) -> Result<(), AppError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS clipboard_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS clipboard_pinned (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL UNIQUE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            sort_order INTEGER DEFAULT 0
        )",
        [],
    )?;

    conn.execute_batch(
        "
        CREATE INDEX IF NOT EXISTS idx_history_created
        ON clipboard_history(created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_pinned_sort
        ON clipboard_pinned(sort_order ASC);
    ",
    )?;

    // Migration: add sort_order if missing
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(clipboard_pinned)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;

    if !cols.contains(&"sort_order".to_string()) {
        conn.execute(
            "ALTER TABLE clipboard_pinned ADD COLUMN sort_order INTEGER DEFAULT 0",
            [],
        )?;

        let mut stmt = conn.prepare("SELECT id FROM clipboard_pinned ORDER BY created_at DESC")?;

        let ids: Vec<i64> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        drop(stmt);

        for (index, id) in ids.iter().enumerate() {
            conn.execute(
                "UPDATE clipboard_pinned SET sort_order = ? WHERE id = ?",
                [index as i64, *id],
            )?;
        }
    }

    // Migration: add description if missing
    if !cols.contains(&"description".to_string()) {
        conn.execute(
            "ALTER TABLE clipboard_pinned ADD COLUMN description TEXT",
            [],
        )?;

        conn.execute(
            "UPDATE clipboard_pinned SET description = content WHERE description IS NULL",
            [],
        )?;
    }

    // Migration: add hidden if missing
    if !cols.contains(&"hidden".to_string()) {
        conn.execute(
            "ALTER TABLE clipboard_pinned ADD COLUMN hidden INTEGER DEFAULT 0",
            [],
        )?;
    }

    Ok(())
}
