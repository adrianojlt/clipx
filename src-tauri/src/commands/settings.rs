use crate::error::AppError;
use crate::settings::{load_settings, normalize_shortcut, save_settings};
use crate::shortcut_handler;
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
            .map_err(|e| AppError::Shortcut(e.to_string()))?;
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
            .map_err(|e| AppError::Shortcut(e.to_string()))?;
        *current = shortcut.clone();
    }

    let mut settings = load_settings();
    settings.insert("hotkey".to_string(), shortcut);
    save_settings(&settings)
}

#[tauri::command]
pub fn apply_window_size(app: AppHandle) -> Result<(), AppError> {
    let settings = load_settings();
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
    if let Some(win) = app.get_webview_window("main") {
        win.set_size(tauri::Size::Logical(tauri::LogicalSize { width, height }))
            .map_err(|e| AppError::Settings(e.to_string()))?;
    }
    Ok(())
}
