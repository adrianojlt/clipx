mod commands;
mod db;
mod error;
mod monitor;
mod settings;

pub use crate::error::AppError;

use std::sync::Mutex;

use rusqlite::Connection;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub struct AppState {
    pub(crate) current_shortcut: Mutex<String>,
    pub(crate) history_limit: Mutex<u32>,
    pub(crate) db: Mutex<Connection>,
}

#[derive(serde::Serialize, Clone)]
pub struct ClipboardItem {
    pub(crate) id: i64,
    pub(crate) content: String,
    pub(crate) created_at: String,
}

#[derive(serde::Serialize, Clone)]
pub struct PinnedItem {
    pub(crate) id: i64,
    pub(crate) content: String,
    pub(crate) description: String,
    pub(crate) hidden: bool,
    pub(crate) created_at: String,
}

pub(crate) fn shortcut_handler(
    app: &tauri::AppHandle,
    _shortcut: &Shortcut,
    event: tauri_plugin_global_shortcut::ShortcutEvent,
) {
    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.hide();
            if let Ok(pos) = app.cursor_position() {
                let _ = win.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                    x: pos.x as i32,
                    y: pos.y as i32,
                }));
            }
            let settings = crate::settings::load_settings();
            let width = settings
                .get("window_width")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(400.0)
                .max(300.0)
                .min(800.0);
            let height = settings
                .get("window_height")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(600.0)
                .max(400.0)
                .min(900.0);
            let _ = win.set_size(tauri::Size::Logical(tauri::LogicalSize { width, height }));
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_limit = {
        let settings = crate::settings::load_settings();
        settings
            .get("history_limit")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(20)
            .clamp(1, 50)
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::update_shortcut,
            commands::settings::apply_window_size,
            commands::clipboard::get_history,
            commands::pinned::get_pinned,
            commands::pinned::pin_item,
            commands::pinned::unpin_item,
            commands::clipboard::delete_history_item,
            commands::pinned::update_pinned_description,
            commands::pinned::reorder_pinned,
            commands::pinned::toggle_pinned_hidden,
            commands::clipboard::get_clipboard,
        ])
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Ensure app data dir exists
            let data_dir = app.path().app_data_dir().map_err(|e| {
                crate::error::AppError::Path(format!("Cannot resolve app data dir: {e}"))
            })?;
            std::fs::create_dir_all(&data_dir).map_err(|e| crate::error::AppError::Io(e))?;

            // Init DB
            let conn = rusqlite::Connection::open(crate::db::db_path(&app.handle())?)?;
            crate::db::init_db(&conn)?;

            // Manage state
            app.manage(AppState {
                current_shortcut: Mutex::new(String::new()),
                history_limit: Mutex::new(initial_limit),
                db: Mutex::new(conn),
            });

            // Start clipboard monitor
            crate::monitor::start_clipboard_monitor(app.handle().clone());

            // Load and register initial shortcut
            let settings = crate::settings::load_settings();
            let hotkey_str = settings
                .get("hotkey")
                .cloned()
                .unwrap_or_else(|| "Option+Space".to_string());

            let normalized = crate::settings::normalize_shortcut(&hotkey_str);
            let shortcut = normalized.parse::<Shortcut>().map_err(|e| {
                crate::error::AppError::Shortcut(format!(
                    "Failed to parse shortcut '{}': {}",
                    hotkey_str, e
                ))
            })?;
            app.global_shortcut()
                .on_shortcut(shortcut, shortcut_handler)?;

            {
                let state = app.state::<AppState>();
                let mut current = state
                    .current_shortcut
                    .lock()
                    .map_err(|e| crate::error::AppError::Shortcut(e.to_string()))?;
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
                .icon(tauri::include_image!("icons/32x32.png"))
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
