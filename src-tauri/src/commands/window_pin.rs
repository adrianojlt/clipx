use crate::error::AppError;

/// Toggle whether the main window floats above all other windows.
#[tauri::command]
pub fn set_always_on_top(window: tauri::WebviewWindow, enable: bool) -> Result<(), AppError> {
    window
        .set_always_on_top(enable)
        .map_err(|e| AppError::Window(e.to_string()))
}

/// Toggle "soft pin": whether the app appears in the OS app switcher.
///
/// Pin state is ephemeral and driven entirely by the frontend; this command is a
/// stateless toggle. Platform behavior:
/// - macOS: switch the activation policy between `Regular` (visible in Cmd+Tab)
///   and `Accessory` (ghost mode, the app's default).
/// - Windows/Linux: implemented in later tasks; currently a no-op returning `Ok`.
#[tauri::command]
pub fn set_soft_pin(app: tauri::AppHandle, enable: bool) -> Result<(), AppError> {
    #[cfg(target_os = "macos")]
    {
        use tauri::Manager;

        let policy = if enable {
            tauri::ActivationPolicy::Regular
        } else {
            tauri::ActivationPolicy::Accessory
        };

        // Capture visibility before switching: returning to Accessory while the
        // app is frontmost makes macOS deactivate it and drop the window.
        let was_visible = app
            .get_webview_window("main")
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false);

        app.set_activation_policy(policy)
            .map_err(|e| AppError::Window(e.to_string()))?;

        if enable {
            // Now `Regular`, the app shows in the dock / Cmd+Tab; apply the ClipX
            // icon on the main thread (an unbundled dev binary otherwise shows a
            // generic icon).
            app.run_on_main_thread(crate::set_dock_icon).map_err(|e| AppError::Window(e.to_string()))?;
        } else if was_visible {
            // Re-assert the window after returning to Accessory so unpinning a
            // visible window keeps it open and focused (matching Windows) instead
            // of vanishing. Skipped at startup, when the window is still hidden.
            let app_handle = app.clone();
            app.run_on_main_thread(move || {
                if let Some(win) = app_handle.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            })
            .map_err(|e| AppError::Window(e.to_string()))?;
        }
    }

    #[cfg(target_os = "windows")]
    {
        use tauri::Manager;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
        };

        let window = app
            .get_webview_window("main")
            .ok_or_else(|| AppError::Window("main window not found".into()))?;
        let hwnd = window
            .hwnd()
            .map_err(|e| AppError::Window(e.to_string()))?;

        // Soft pin shows the window in Alt+Tab via WS_EX_APPWINDOW; ghost mode
        // uses WS_EX_TOOLWINDOW. The two styles are mutually exclusive here.
        let app_window = WS_EX_APPWINDOW.0 as isize;
        let tool_window = WS_EX_TOOLWINDOW.0 as isize;
        unsafe {
            let mut ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            if enable {
                ex_style = (ex_style | app_window) & !tool_window;
            } else {
                ex_style = (ex_style | tool_window) & !app_window;
            }
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style);
        }
    }

    // Linux: there is no reliable cross-compositor way to toggle app-switcher
    // visibility (Wayland exposes none, and X11 hints are not honored
    // consistently). Soft pin is a documented no-op here; the window keeps its
    // default behavior. Hide-on-focus-loss is handled on the frontend on every
    // platform, so the feature still degrades gracefully. Known limitation.
    #[cfg(target_os = "linux")]
    {
        let _ = (&app, enable);
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = (&app, enable);
    }

    Ok(())
}
