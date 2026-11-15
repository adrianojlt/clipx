use crate::error::AppError;
use crate::AppState;
use tauri::{AppHandle, State};

/// Time for the app behind the popup to take focus back once the popup hides.
/// Keys sent before then land in ClipX, or nowhere.
#[cfg(any(windows, target_os = "macos"))]
const FOCUS_SETTLE: std::time::Duration = std::time::Duration::from_millis(100);

/// Paste the clipboard into the app in front, when the `auto_paste` setting is on.
///
/// Called by the popup right after it copied a selection and hid itself. Returns
/// at once: the keys go out from a short-lived thread after the focus settles,
/// and a failure leaves the copy in place, logged rather than reported.
#[tauri::command]
pub fn paste_into_active_app(app: AppHandle, state: State<AppState>) -> Result<(), AppError> {

    let enabled = state
        .settings
        .lock()
        .map(|s| s.auto_paste)
        .map_err(|_| AppError::State("settings mutex poisoned".into()))?;

    if enabled {
        schedule_paste(app);
    }

    Ok(())
}

#[cfg(any(windows, target_os = "macos"))]
fn schedule_paste(app: AppHandle) {

    std::thread::spawn(move || {

        std::thread::sleep(FOCUS_SETTLE);

        // On the main thread: on macOS, enigo resolves the V key through the
        // Text Input Sources API, which must not be called from any other.
        let dispatched = app.run_on_main_thread(|| {
            if let Err(e) = send_paste_chord() {
                log::warn!("auto paste: {e}, item left on the clipboard only");
            }
        });

        if let Err(e) = dispatched {
            log::warn!("auto paste: could not reach the main thread: {e}");
        }
    });
}

#[cfg(not(any(windows, target_os = "macos")))]
fn schedule_paste(_app: AppHandle) {
    log::warn!("auto paste: not supported on this platform, item left on the clipboard only");
}

#[cfg(any(windows, target_os = "macos"))]
fn send_paste_chord() -> Result<(), String> {

    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(windows)]
    let modifier = Key::Control;

    // No permission prompt: without Accessibility on macOS this fails, and the
    // selection stays a plain copy.
    let settings = Settings {
        open_prompt_to_get_permissions: false,
        ..Settings::default()
    };

    let mut enigo = Enigo::new(&settings).map_err(|e| e.to_string())?;

    enigo.key(modifier, Direction::Press).map_err(|e| e.to_string())?;
    let clicked = enigo.key(Key::Unicode('v'), Direction::Click);
    let released = enigo.key(modifier, Direction::Release);

    clicked.and(released).map_err(|e| e.to_string())
}
