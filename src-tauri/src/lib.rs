use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, State,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

struct AppState {
    current_shortcut: Mutex<String>,
}

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
        .replace("CMD", "META")
        .replace("COMMAND", "META")
        .replace("SUPER", "META")
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

    // Unregister current
    let normalized_old = normalize_shortcut(&old_shortcut_str);
    if let Ok(old) = normalized_old.parse::<Shortcut>() {
        let _ = app.global_shortcut().unregister(old);
    }

    // Register new
    let normalized_new = normalize_shortcut(&shortcut);
    let new_shortcut = normalized_new
        .parse::<Shortcut>()
        .map_err(|e| e.to_string())?;
    app.global_shortcut()
        .on_shortcut(new_shortcut, shortcut_handler)
        .map_err(|e| e.to_string())?;

    // Update state
    {
        let mut current = state.current_shortcut.lock().map_err(|e| e.to_string())?;
        *current = shortcut.clone();
    }

    // Save to settings
    let mut settings = load_settings();
    settings.insert("hotkey".to_string(), shortcut);
    save_settings(&settings)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState {
            current_shortcut: Mutex::new(String::new()),
        })
        .invoke_handler(tauri::generate_handler![
            get_setting,
            set_setting,
            update_shortcut
        ])
        .setup(|app| {
            // Load and register initial shortcut
            let settings = load_settings();
            let hotkey_str = settings
                .get("hotkey")
                .cloned()
                .unwrap_or_else(|| "Option+3".to_string());

            let normalized = normalize_shortcut(&hotkey_str);
            let shortcut = normalized
                .parse::<Shortcut>()
                .map_err(|e| format!("Failed to parse shortcut '{}': {}", hotkey_str, e))?;
            app.global_shortcut()
                .on_shortcut(shortcut, shortcut_handler)?;

            // Update state
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
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

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

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
