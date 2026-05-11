use crate::commands::lock_db;
use crate::error::AppError;
use crate::settings::{normalize_shortcut, save_settings, Settings};
use crate::window::{clamp_to_monitor, monitor_under_point, shortcut_handler};
use crate::AppState;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

fn apply_field(s: &mut Settings, key: &str, value: &str) -> Result<(), AppError> {

    match key {
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

fn settings_from_state(state: &State<AppState>) -> Result<Settings, AppError> {
    Ok(Settings {
        hotkey: state
            .current_shortcut
            .lock()
            .map_err(|_| AppError::State("current_shortcut poisoned".into()))?
            .clone(),
        history_limit: *state
            .history_limit
            .lock()
            .map_err(|_| AppError::State("history_limit poisoned".into()))?,
        window_width: *state
            .window_width
            .lock()
            .map_err(|_| AppError::State("window_width poisoned".into()))?,
        window_height: *state
            .window_height
            .lock()
            .map_err(|_| AppError::State("window_height poisoned".into()))?,
        tab_shortcut_pinned: state
            .tab_shortcut_pinned
            .lock()
            .map_err(|_| AppError::State("tab_shortcut_pinned poisoned".into()))?
            .clone(),
        tab_shortcut_history: state
            .tab_shortcut_history
            .lock()
            .map_err(|_| AppError::State("tab_shortcut_history poisoned".into()))?
            .clone(),
    })
}

fn apply_settings_to_state(settings: &Settings, state: &State<AppState>) {
    if let Ok(mut v) = state.history_limit.lock()        { *v = settings.history_limit; }
    if let Ok(mut v) = state.window_width.lock()         { *v = settings.window_width; }
    if let Ok(mut v) = state.window_height.lock()        { *v = settings.window_height; }
    if let Ok(mut v) = state.tab_shortcut_pinned.lock()  { *v = settings.tab_shortcut_pinned.clone(); }
    if let Ok(mut v) = state.tab_shortcut_history.lock() { *v = settings.tab_shortcut_history.clone(); }
}

#[tauri::command]
pub fn get_setting(key: String, state: State<AppState>) -> Result<String, AppError> {

    let settings = settings_from_state(&state)?;
    let val = serde_json::to_value(&settings)?;

    match val.get(&key) {
        Some(serde_json::Value::String(s)) => Ok(s.clone()),
        Some(v) => Ok(v.to_string()),
        None => Err(AppError::Settings(format!("Unknown setting: {key}"))),
    }
}

#[tauri::command]
pub fn set_setting(key: String, value: String, state: State<AppState>, app: AppHandle) -> Result<(), AppError> {

    if key == "hotkey" {
        return update_shortcut(value, state, app);
    }

    let mut settings = settings_from_state(&state)?;
    apply_field(&mut settings, &key, &value)?;

    settings.validate();
    apply_settings_to_state(&settings, &state);
    save_settings(&app, &settings)?;

    if key == "tab_shortcut_pinned" || key == "tab_shortcut_history" {
        let _ = app.emit("settings-changed", &key);
    }

    if key == "history_limit" {

        let conn = lock_db(&state)?;

        conn.execute(
            "DELETE FROM clipboard_history WHERE id NOT IN \
             (SELECT id FROM clipboard_history ORDER BY created_at DESC LIMIT ?1)",
            [settings.history_limit as i64],
        )?;
    }

    Ok(())
}

#[tauri::command]
pub fn update_shortcut(
    shortcut: String,
    state: State<AppState>,
    app: tauri::AppHandle,
) -> Result<(), AppError> {

    {
        let mut current = state
            .current_shortcut
            .lock()
            .map_err(|e| AppError::State(format!("current_shortcut mutex poisoned: {e}")))?;

        let normalized_new = normalize_shortcut(&shortcut);
        let normalized_old = normalize_shortcut(&*current);

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

        *current = shortcut.clone();
    } // lock released before settings_from_state re-acquires it

    let mut settings = settings_from_state(&state)?;
    settings.hotkey = shortcut;

    save_settings(&app, &settings)
}

#[tauri::command]
pub fn apply_window_size(state: State<AppState>, app: AppHandle) -> Result<(), AppError> {

    let width  = state.window_width.lock().map(|w| *w).unwrap_or(400.0);
    let height = state.window_height.lock().map(|h| *h).unwrap_or(600.0);

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
