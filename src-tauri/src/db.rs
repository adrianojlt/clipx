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

    conn.execute_batch("PRAGMA journal_mode=WAL;")?;

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
        "CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            is_global INTEGER DEFAULT 0,
            is_active INTEGER DEFAULT 0,
            sort_order INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    tx.execute(
        "INSERT OR IGNORE INTO sessions (id, name, is_global, is_active, sort_order) \
         VALUES (1, 'Favorites', 1, 1, 0)",
        [],
    )?;

    tx.execute(
        "UPDATE sessions SET name = 'Favorites' WHERE is_global = 1 AND name = 'Global'",
        [],
    )?;

    tx.execute(
        "CREATE TABLE IF NOT EXISTS clipboard_pinned (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL UNIQUE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            sort_order INTEGER DEFAULT 0,
            description TEXT,
            hidden INTEGER DEFAULT 0
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

    // Migrate clipboard_pinned to add session_id with UNIQUE(content, session_id)
    if !has("session_id") {

        tx.execute_batch(
            "CREATE TABLE clipboard_pinned_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT NOT NULL,
                session_id INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                sort_order INTEGER DEFAULT 0,
                description TEXT,
                hidden INTEGER DEFAULT 0,
                UNIQUE(content, session_id),
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );
            INSERT INTO clipboard_pinned_new
                SELECT id, content, 1, created_at, sort_order, description, hidden
                FROM clipboard_pinned;
            DROP TABLE clipboard_pinned;
            ALTER TABLE clipboard_pinned_new RENAME TO clipboard_pinned;
            CREATE INDEX idx_pinned_sort ON clipboard_pinned(sort_order ASC);
            CREATE INDEX idx_pinned_session ON clipboard_pinned(session_id);",
        )?;
    }

    tx.commit()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_session_id_migration_preserves_data() {
        let mut conn = Connection::open_in_memory().unwrap();

        conn.execute_batch(
            "CREATE TABLE clipboard_pinned (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT NOT NULL UNIQUE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                sort_order INTEGER DEFAULT 0,
                description TEXT,
                hidden INTEGER DEFAULT 0
            );
            INSERT INTO clipboard_pinned (content) VALUES ('hello');
            INSERT INTO clipboard_pinned (content) VALUES ('world');",
        )
        .unwrap();

        init_db(&mut conn).unwrap();

        let rows: Vec<(String, i64)> = conn
            .prepare("SELECT content, session_id FROM clipboard_pinned ORDER BY content")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], ("hello".to_string(), 1));
        assert_eq!(rows[1], ("world".to_string(), 1));

        let result = conn.execute(
            "INSERT INTO clipboard_pinned (content, session_id) VALUES ('hello', 1)",
            [],
        );
        assert!(result.is_err(), "duplicate (content, session_id) must be rejected");
    }

    #[test]
    fn test_session_id_migration_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        init_db(&mut conn).unwrap();
        init_db(&mut conn).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM clipboard_pinned", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
