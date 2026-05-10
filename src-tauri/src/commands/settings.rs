use crate::error::AppError;
use crate::settings::{load_settings, load_window_size, normalize_shortcut, save_settings, Settings};
use crate::window::{clamp_to_monitor, monitor_under_point, shortcut_handler};
use crate::AppState;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

fn apply_field(s: &mut Settings, key: &str, value: &str) -> Result<(), AppError> {

    match key {
        "hotkey" => s.hotkey = value.to_string(),
        "history_limit" => {
            s.history_limit = value
                .parse::<u32>()
                .map_err(|_| AppError::Validation(format!("Invalid history_limit: {value}")))?;
        }
        "window_width" => {
            s.window_width = value
                .parse::<f64>()
                .map_err(|_| AppError::Validation(format!("Invalid window_width: {value}")))?;
        }
        "window_height" => {
            s.window_height = value
                .parse::<f64>()
                .map_err(|_| AppError::Validation(format!("Invalid window_height: {value}")))?;
        }
        "tab_shortcut_pinned" => s.tab_shortcut_pinned = value.to_string(),
        "tab_shortcut_history" => s.tab_shortcut_history = value.to_string(),
        _ => return Err(AppError::Settings(format!("Unknown setting: {key}"))),
    }

    Ok(())
}

#[tauri::command]
pub fn get_setting(key: String, app: AppHandle) -> Result<String, AppError> {

    let settings = load_settings(&app);

    let val = serde_json::to_value(&settings)?;

    match val.get(&key) {
        Some(serde_json::Value::String(s)) => Ok(s.clone()),
        Some(v) => Ok(v.to_string()),
        None => Err(AppError::Settings(format!("Unknown setting: {key}"))),
    }
}

#[tauri::command]
pub fn set_setting(key: String, value: String, state: State<AppState>, app: AppHandle) -> Result<(), AppError> {

    let mut settings = load_settings(&app);
    apply_field(&mut settings, &key, &value)?;
    settings.validate();
    save_settings(&app, &settings)?;

    if key == "history_limit" {
        if let Ok(mut cached) = state.history_limit.lock() {
            *cached = settings.history_limit;
        }
    }

    if key == "tab_shortcut_pinned" || key == "tab_shortcut_history" {
        let _ = app.emit("settings-changed", &key);
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

    let normalized_new = normalize_shortcut(&shortcut);
    let normalized_old = normalize_shortcut(&old_shortcut_str);

    if normalized_new != normalized_old {

        let new_shortcut = normalized_new
            .parse::<Shortcut>()
            .map_err(|e| AppError::Shortcut(e.to_string()))?;

        app.global_shortcut()
            .on_shortcut(new_shortcut, shortcut_handler)
            .map_err(|e| AppError::Shortcut(e.to_string()))?;

        if let Ok(old) = normalized_old.parse::<Shortcut>() {
            let _ = app.global_shortcut().unregister(old);
        }
    }

    {
        let mut current = state
            .current_shortcut
            .lock()
            .map_err(|e| AppError::State(format!("current_shortcut mutex poisoned: {e}")))?;
        *current = shortcut.clone();
    }

    let mut settings = load_settings(&app);
    settings.hotkey = shortcut;
    save_settings(&app, &settings)
}

#[tauri::command]
pub fn apply_window_size(app: AppHandle) -> Result<(), AppError> {
    let (width, height) = load_window_size(&load_settings(&app));
    let Some(win) = app.get_webview_window("main") else {
        return Ok(());
    };

    win.set_size(tauri::Size::Logical(tauri::LogicalSize { width, height }))
        .map_err(|e| AppError::Window(e.to_string()))?;

    // If the window is currently visible, re clamp its position so the resize
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
