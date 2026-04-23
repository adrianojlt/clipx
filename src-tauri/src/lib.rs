use arboard::Clipboard;
use rusqlite::Connection;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tauri::{menu::{Menu, MenuItem}, tray::TrayIconBuilder, AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

struct AppState {
    current_shortcut: Mutex<String>,
}

#[derive(serde::Serialize, Clone)]
struct ClipboardItem {
    id: i64,
    content: String,
    created_at: String,
}

#[derive(serde::Serialize, Clone)]
struct PinnedItem {
    id: i64,
    content: String,
    description: String,
    created_at: String,
}

// --- Settings helpers ---

fn settings_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string()))
    } else {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".config")
    }
    .join("clipboard-manager")
}

fn settings_path() -> PathBuf {
    settings_dir().join("settings.json")
}

fn load_settings() -> HashMap<String, String> {
    let path = settings_path();
    if let Ok(content) = fs::read_to_string(&path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    }
}

fn save_settings(settings: &HashMap<String, String>) -> Result<(), String> {
    let dir = settings_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("settings.json");
    let content = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}

fn normalize_shortcut(s: &str) -> String {
    s.to_uppercase()
        .replace("OPTION", "ALT")
        .replace("META", "SUPER")
        .replace("COMMAND", "SUPER")
        .replace("CMD", "SUPER")
        .replace("CONTROL", "CTRL")
}

fn shortcut_handler(app: &tauri::AppHandle, _shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state() == ShortcutState::Pressed {
        if let Some(win) = app.get_webview_window("main") {
            if let Ok(pos) = app.cursor_position() {
                let _ = win.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                    x: pos.x as i32,
                    y: pos.y as i32,
                }));
            }
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

// --- Database helpers ---

fn db_path(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap().join("clipx.db")
}

fn init_db(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS clipboard_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS clipboard_pinned (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL UNIQUE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            sort_order INTEGER DEFAULT 0
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    // Migration: add sort_order if missing
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(clipboard_pinned)")
        .map_err(|e| e.to_string())?
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    if !cols.contains(&"sort_order".to_string()) {

        conn.execute("ALTER TABLE clipboard_pinned ADD COLUMN sort_order INTEGER DEFAULT 0", []).map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare("SELECT id FROM clipboard_pinned ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;
        let ids: Vec<i64> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        drop(stmt);

        for (index, id) in ids.iter().enumerate() {
            conn.execute("UPDATE clipboard_pinned SET sort_order = ? WHERE id = ?", [index as i64, *id]).map_err(|e| e.to_string())?;
        }
    }

    // Migration: add description if missing
    if !cols.contains(&"description".to_string()) {
        conn.execute("ALTER TABLE clipboard_pinned ADD COLUMN description TEXT", []).map_err(|e| e.to_string())?;
        conn.execute("UPDATE clipboard_pinned SET description = content WHERE description IS NULL", []).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn start_clipboard_monitor(app: AppHandle) {

    thread::spawn(move || {

        let db_file = db_path(&app);

        let mut clipboard = match Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to initialize clipboard: {}", e);
                return;
            }
        };

        let mut last_text = String::new();

        loop {

            thread::sleep(Duration::from_millis(500));

            let text = match clipboard.get_text() {
                Ok(t) => t,
                Err(_) => continue,
            };

            if text.is_empty() || text == last_text {
                continue;
            }

            last_text = text.clone();

            let conn = match Connection::open(&db_file) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Remove duplicate if it exists so the text appears only once
            let _ = conn.execute("DELETE FROM clipboard_history WHERE content = ?", [&text]);
            let _ = conn.execute( "INSERT INTO clipboard_history (content) VALUES (?)", [&text]);
            let _ = conn.execute(
                "DELETE FROM clipboard_history WHERE id NOT IN (
                    SELECT id FROM clipboard_history ORDER BY created_at DESC LIMIT 10
                )",
                [],
            );

            let _ = app.emit("clipboard-changed", ());
        }
    });
}

// --- Commands ---

#[tauri::command]
fn get_history(app: AppHandle) -> Result<Vec<ClipboardItem>, String> {
    let conn = Connection::open(db_path(&app)).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, content, created_at FROM clipboard_history ORDER BY created_at DESC LIMIT 10"
    ).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ClipboardItem {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for item in rows {
        items.push(item.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
fn get_pinned(app: AppHandle) -> Result<Vec<PinnedItem>, String> {
    let conn = Connection::open(db_path(&app)).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, content, COALESCE(description, content), created_at FROM clipboard_pinned ORDER BY sort_order ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(PinnedItem {
                id: row.get(0)?,
                content: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for item in rows {
        items.push(item.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
fn pin_item(content: String, app: AppHandle) -> Result<(), String> {

    let conn = Connection::open(db_path(&app)).map_err(|e| e.to_string())?;

    conn.execute( "UPDATE clipboard_pinned SET sort_order = sort_order + 1", [],).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR IGNORE INTO clipboard_pinned (content, description, sort_order) VALUES (?, ?, 0)", rusqlite::params![content, content]).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn reorder_pinned(items: Vec<i64>, app: AppHandle) -> Result<(), String> {

    let mut conn = Connection::open(db_path(&app)).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    for (index, id) in items.iter().enumerate() {
        tx.execute( "UPDATE clipboard_pinned SET sort_order = ? WHERE id = ?", [index as i64, *id],).map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn update_pinned_description(id: i64, description: String, app: AppHandle) -> Result<(), String> {
    let conn = Connection::open(db_path(&app)).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE clipboard_pinned SET description = ? WHERE id = ?",
        rusqlite::params![description, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn unpin_item(id: i64, app: AppHandle) -> Result<(), String> {
    let conn = Connection::open(db_path(&app)).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM clipboard_pinned WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_setting(key: String) -> Result<String, String> {
    let settings = load_settings();
    settings
        .get(&key)
        .cloned()
        .ok_or_else(|| "Setting not found".to_string())
}

#[tauri::command]
fn set_setting(key: String, value: String) -> Result<(), String> {
    let mut settings = load_settings();
    settings.insert(key, value);
    save_settings(&settings)
}

#[tauri::command]
fn update_shortcut(
    shortcut: String,
    state: State<AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {

    let old_shortcut_str = {
        let current = state.current_shortcut.lock().map_err(|e| e.to_string())?;
        current.clone()
    };

    let normalized_old = normalize_shortcut(&old_shortcut_str);
    if let Ok(old) = normalized_old.parse::<Shortcut>() {
        let _ = app.global_shortcut().unregister(old);
    }

    let normalized_new = normalize_shortcut(&shortcut);
    let new_shortcut = normalized_new
        .parse::<Shortcut>()
        .map_err(|e| e.to_string())?;
    app.global_shortcut()
        .on_shortcut(new_shortcut, shortcut_handler)
        .map_err(|e| e.to_string())?;

    {
        let mut current = state.current_shortcut.lock().map_err(|e| e.to_string())?;
        *current = shortcut.clone();
    }

    let mut settings = load_settings();
    settings.insert("hotkey".to_string(), shortcut);
    save_settings(&settings)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState {
            current_shortcut: Mutex::new(String::new()),
        })
        .invoke_handler(tauri::generate_handler![
            get_setting,
            set_setting,
            update_shortcut,
            get_history,
            get_pinned,
            pin_item,
            unpin_item,
            update_pinned_description,
            reorder_pinned,
        ])
        .setup(|app| {

            // Ensure app data dir exists
            let _ = std::fs::create_dir_all(app.path().app_data_dir().unwrap());

            // Init DB
            let conn = Connection::open(db_path(&app.handle())).map_err(|e| e.to_string())?;
            init_db(&conn).map_err(|e| e.to_string())?;
            drop(conn);

            // Start clipboard monitor
            start_clipboard_monitor(app.handle().clone());

            // Load and register initial shortcut
            let settings = load_settings();
            let hotkey_str = settings
                .get("hotkey")
                .cloned()
                .unwrap_or_else(|| "Option+Command+1".to_string());

            let normalized = normalize_shortcut(&hotkey_str);
            let shortcut = normalized
                .parse::<Shortcut>()
                .map_err(|e| format!("Failed to parse shortcut '{}': {}", hotkey_str, e))?;
            app.global_shortcut()
                .on_shortcut(shortcut, shortcut_handler)?;

            {
                let state = app.state::<AppState>();
                let mut current = state.current_shortcut.lock().unwrap();
                *current = hotkey_str;
            }

            // Tray menu
            let open_i = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
            let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let about_i = MenuItem::with_id(app, "about", "About", true, None::<&str>)?;
            let sep = tauri::menu::PredefinedMenuItem::separator(app)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_i, &settings_i, &about_i, &sep, &quit_i])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "settings" => {
                        if let Some(win) = app.get_webview_window("settings") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "about" => {
                        if let Some(win) = app.get_webview_window("about") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // Intercept close on main window and hide instead
            if let Some(win) = app.get_webview_window("main") {
                let win_clone = win.clone();
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_clone.hide();
                    }
                });
            }

            // Intercept close on settings window and hide instead
            if let Some(win) = app.get_webview_window("settings") {
                let win_clone = win.clone();
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_clone.hide();
                    }
                });
            }

            // Intercept close on about window and hide instead
            if let Some(win) = app.get_webview_window("about") {
                let win_clone = win.clone();
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_clone.hide();
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
