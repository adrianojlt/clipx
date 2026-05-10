mod commands;
mod db;
mod error;
mod logging;
mod monitor;
mod settings;
mod window;

pub use crate::error::AppError;

pub(crate) const MAX_CLIP_BYTES: usize = 1_048_576;

use std::sync::Mutex;

use rusqlite::Connection;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_window(app, "main");
        }))
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
            commands::logging::log_frontend_error,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| AppError::Path(format!("Cannot resolve app data dir: {e}")))?;

            if let Err(e) = crate::logging::init_logging(&data_dir) {
                eprintln!("logging init failed: {e}");
            }

            if let Err(e) = crate::settings::migrate_legacy_settings(app.handle()) {
                log::warn!("settings: legacy migration failed: {e}");
            }

            init_app_state(app)?;
            crate::monitor::start_clipboard_monitor(app.handle().clone());
            register_initial_shortcut(app)?;
            build_tray(app)?;

            for label in ["main", "settings", "about"] {
                hide_on_close(app.handle(), label);
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_app_state(app: &mut tauri::App) -> Result<(), AppError> {

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Path(format!("Cannot resolve app data dir: {e}")))?;
        
    std::fs::create_dir_all(&data_dir)?;

    let settings = crate::settings::load_settings(app.handle());
    let initial_limit = settings.history_limit;
    let initial_hotkey = settings.hotkey.clone();

    let mut conn = Connection::open(crate::db::db_path(app.handle())?)?;
    crate::db::init_db(&mut conn)?;

    app.manage(AppState {
        current_shortcut: Mutex::new(initial_hotkey),
        history_limit: Mutex::new(initial_limit),
        db: Mutex::new(conn),
    });

    Ok(())
}

fn register_initial_shortcut(app: &tauri::App) -> Result<(), AppError> {

    let settings = crate::settings::load_settings(app.handle());
    let hotkey_str = settings.hotkey;

    let normalized = crate::settings::normalize_shortcut(&hotkey_str);

    let shortcut = normalized
        .parse::<Shortcut>()
        .map_err(|e| AppError::Shortcut(format!("Failed to parse '{}': {}", hotkey_str, e)))?;

    app.global_shortcut()
        .on_shortcut(shortcut, crate::window::shortcut_handler)
        .map_err(|e| AppError::Shortcut(e.to_string()))?;

    Ok(())
}

fn build_tray(app: &tauri::App) -> Result<(), AppError> {

    let open_i = MenuItem::with_id(app, "open", "Open", true, None::<&str>)
        .map_err(|e| AppError::Window(e.to_string()))?;
    let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)
        .map_err(|e| AppError::Window(e.to_string()))?;
    let about_i = MenuItem::with_id(app, "about", "About", true, None::<&str>)
        .map_err(|e| AppError::Window(e.to_string()))?;
    let sep = tauri::menu::PredefinedMenuItem::separator(app)
        .map_err(|e| AppError::Window(e.to_string()))?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)
        .map_err(|e| AppError::Window(e.to_string()))?;
    let menu = Menu::with_items(app, &[&open_i, &settings_i, &about_i, &sep, &quit_i])
        .map_err(|e| AppError::Window(e.to_string()))?;

    TrayIconBuilder::new()
        .icon(tauri::include_image!("icons/32x32.png"))
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_window(app, "main"),
            "settings" => show_window(app, "settings"),
            "about" => show_window(app, "about"),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)
        .map_err(|e| AppError::Window(e.to_string()))?;
    Ok(())
}

fn show_window(app: &AppHandle, label: &str) {
    if let Some(win) = app.get_webview_window(label) {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn hide_on_close(app: &AppHandle, label: &str) {
    if let Some(win) = app.get_webview_window(label) {
        let win_clone = win.clone();
        win.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = win_clone.hide();
            }
        });
    }
}
