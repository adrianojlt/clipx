mod commands;
mod db;
mod error;
mod logging;
mod monitor;
mod settings;
mod window;

pub use crate::error::AppError;

pub(crate) const MAX_CLIP_BYTES: usize = 1_048_576;
pub(crate) const MAX_DESC_BYTES: usize = 256;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

use crate::settings::Settings;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub struct AppState {
    pub(crate) settings: Mutex<Settings>,
    pub(crate) db: Mutex<Connection>,
    pub(crate) db_monitor: Mutex<Connection>,
    pub(crate) shutdown: Arc<AtomicBool>,
    pub(crate) monitor_handles: Mutex<Option<(std::thread::JoinHandle<()>, std::thread::JoinHandle<()>)>>,
    pub(crate) monitor_tx: Mutex<Option<std::sync::mpsc::Sender<String>>>,
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

#[derive(serde::Serialize, Clone)]
pub struct Session {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) is_global: bool,
    pub(crate) is_active: bool,
    pub(crate) sort_order: i64,
    pub(crate) item_count: i64,
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
            commands::pinned::get_global_pinned,
            commands::pinned::pin_item,
            commands::pinned::unpin_item,
            commands::clipboard::delete_history_item,
            commands::pinned::update_pinned_description,
            commands::pinned::reorder_pinned,
            commands::pinned::toggle_pinned_hidden,
            commands::clipboard::get_clipboard,
            commands::logging::log_frontend_error,
            commands::sessions::get_sessions,
            commands::sessions::create_session,
            commands::sessions::delete_session,
            commands::sessions::activate_session,
            commands::sessions::reorder_sessions,
            commands::sessions::pin_item_to_session,
            commands::apps::list_open_apps,
            commands::apps::focus_app,
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
            build_tray(app)?;

            for label in ["main", "settings", "about"] {
                hide_on_close(app.handle(), label);
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app, event| match event {

            tauri::RunEvent::Ready => {
                if let Err(e) = register_initial_shortcut(app) {
                    log::error!("failed to register initial global shortcut: {e}");
                }
            }

            tauri::RunEvent::Exit => {

                let state = app.state::<AppState>();

                state.shutdown.store(true, Ordering::SeqCst);

                state.monitor_tx.lock().ok().map(|mut g| g.take());

                let handles = state.monitor_handles.lock().ok().and_then(|mut g| g.take());

                if let Some((h1, h2)) = handles {

                    let (tx, rx) = std::sync::mpsc::channel::<()>();

                    std::thread::spawn(move || {
                        let _ = h1.join();
                        let _ = h2.join();
                        let _ = tx.send(());
                    });

                    if rx.recv_timeout(std::time::Duration::from_secs(2)).is_err() {
                        log::warn!("monitor threads did not exit in 2 s; forcing exit");
                        std::process::exit(0);
                    }
                }
            }
            _ => {}
        });
}

fn init_app_state(app: &mut tauri::App) -> Result<(), AppError> {

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Path(format!("Cannot resolve app data dir: {e}")))?;
        
    std::fs::create_dir_all(&data_dir)?;

    let settings = crate::settings::load_settings(app.handle());

    let mut conn = Connection::open(crate::db::db_path(app.handle())?)?;
    crate::db::init_db(&mut conn)?;

    if let Ok(p) = crate::db::db_path(app.handle()) {
        log::info!("db path: {}", p.display());
    }

    let session_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))
        .unwrap_or(-1);

    let history_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM clipboard_history", [], |r| r.get(0))
        .unwrap_or(-1);

    log::info!("startup row counts: sessions={session_count} history={history_count}");

    let conn_monitor = Connection::open(crate::db::db_path(app.handle())?)?;

    conn_monitor.execute_batch("PRAGMA busy_timeout = 5000;")?;

    app.manage(AppState {
        settings: Mutex::new(settings),
        db: Mutex::new(conn),
        db_monitor: Mutex::new(conn_monitor),
        shutdown: Arc::new(AtomicBool::new(false)),
        monitor_handles: Mutex::new(None),
        monitor_tx: Mutex::new(None),
    });

    Ok(())
}

fn register_initial_shortcut(app: &AppHandle) -> Result<(), AppError> {

    let state = app.state::<AppState>();

    let hotkey_str = state
        .settings
        .lock()
        .map_err(|_| AppError::State("settings poisoned".into()))?
        .hotkey
        .clone();

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
        if label == "main" {
            let _ = app.emit("main-window-shown", ());
        }
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
