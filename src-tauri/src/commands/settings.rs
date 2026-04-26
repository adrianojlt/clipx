use crate::error::AppError;
use crate::settings::{load_settings, load_window_size, normalize_shortcut, save_settings};
use crate::window::{clamp_to_monitor, monitor_under_point, shortcut_handler};
use crate::AppState;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

#[tauri::command]
pub fn get_setting(key: String) -> Result<String, AppError> {
    let settings = load_settings();
    settings
        .get(&key)
        .cloned()
        .ok_or_else(|| AppError::Settings("Setting not found".to_string()))
}

#[tauri::command]
pub fn set_setting(key: String, value: String, state: State<AppState>) -> Result<(), AppError> {
    let mut settings = load_settings();
    settings.insert(key.clone(), value.clone());
    save_settings(&settings)?;

    if key == "history_limit" {
        if let Ok(limit) = value.parse::<u32>() {
            if let Ok(mut cached) = state.history_limit.lock() {
                *cached = limit.clamp(1, 50);
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub fn update_shortcut(
    shortcut: String,
    state: State<AppState>,
    app: tauri::AppHandle,
) -> Result<(), AppError> {
    let old_shortcut_str = {
        let current = state
            .current_shortcut
            .lock()
            .map_err(|e| AppError::State(format!("current_shortcut mutex poisoned: {e}")))?;
        current.clone()
    };

    let normalized_old = normalize_shortcut(&old_shortcut_str);
    if let Ok(old) = normalized_old.parse::<Shortcut>() {
        let _ = app.global_shortcut().unregister(old);
    }

    let normalized_new = normalize_shortcut(&shortcut);
    let new_shortcut = normalized_new
        .parse::<Shortcut>()
        .map_err(|e| AppError::Shortcut(e.to_string()))?;
    app.global_shortcut()
        .on_shortcut(new_shortcut, shortcut_handler)
        .map_err(|e| AppError::Shortcut(e.to_string()))?;

    {
        let mut current = state
            .current_shortcut
            .lock()
            .map_err(|e| AppError::State(format!("current_shortcut mutex poisoned: {e}")))?;
        *current = shortcut.clone();
    }

    let mut settings = load_settings();
    settings.insert("hotkey".to_string(), shortcut);
    save_settings(&settings)
}

#[tauri::command]
pub fn apply_window_size(app: AppHandle) -> Result<(), AppError> {
    let (width, height) = load_window_size(&load_settings());
    let Some(win) = app.get_webview_window("main") else {
        return Ok(());
    };

    win.set_size(tauri::Size::Logical(tauri::LogicalSize { width, height }))
        .map_err(|e| AppError::Window(e.to_string()))?;

    // If the window is currently visible, re-clamp its position so the resize
    // can't push the right/bottom edge off the current monitor.
    if win.is_visible().unwrap_or(false) {
        if let Ok(pos) = win.outer_position() {
            let monitor = monitor_under_point(&win, pos.x, pos.y);
            let (x, y) = clamp_to_monitor(pos.x, pos.y, width, height, monitor.as_ref());
            if x != pos.x || y != pos.y {
                let _ =
                    win.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
            }
        }
    }

    Ok(())
}
