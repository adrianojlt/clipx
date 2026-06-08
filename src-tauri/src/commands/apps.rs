use crate::error::AppError;

#[derive(serde::Serialize, Clone)]
pub struct OpenApp {
    pub(crate) name: String,
    pub(crate) id: String,
}

// async so Tauri runs these off the main thread; the platform helpers spawn a
// blocking subprocess and must not freeze the UI event loop.
#[tauri::command]
pub async fn list_open_apps() -> Result<Vec<OpenApp>, AppError> {
    platform::list_open_apps()
}

#[tauri::command]
pub async fn focus_app(id: String) -> Result<(), AppError> {
    platform::focus_app(&id)
}

#[cfg(target_os = "macos")]
mod platform {
    use super::OpenApp;
    use crate::error::AppError;
    use core_foundation::array::{CFArray, CFArrayRef};
    use core_foundation::base::{TCFType, ToVoid};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use std::collections::HashSet;
    use std::process::Command;

    // Separator packed into `id` to carry both process name and window title.
    const SEP: char = '\u{1f}';

    // CGWindowListOption flags.
    const ON_SCREEN_ONLY: u32 = 1 << 0;
    const EXCLUDE_DESKTOP_ELEMENTS: u32 = 1 << 4;
    const NULL_WINDOW_ID: u32 = 0;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: u32) -> CFArrayRef;
    }

    fn dict_string(dict: &CFDictionary, key: &CFString) -> Option<String> {
        let raw = dict.find(key.to_void())?;
        let value = unsafe { core_foundation::base::CFType::wrap_under_get_rule(*raw) };
        value.downcast::<CFString>().map(|s| s.to_string())
    }

    fn dict_i64(dict: &CFDictionary, key: &CFString) -> Option<i64> {
        let raw = dict.find(key.to_void())?;
        let value = unsafe { core_foundation::base::CFType::wrap_under_get_rule(*raw) };
        value.downcast::<CFNumber>().and_then(|n| n.to_i64())
    }

    // One entry per on-screen window via CoreGraphics (in-process, fast).
    // Window titles (kCGWindowName) need Screen Recording permission; without
    // it the title is empty and entries collapse to one per app.
    pub fn list_open_apps() -> Result<Vec<OpenApp>, AppError> {

        let key_owner = CFString::from_static_string("kCGWindowOwnerName");
        let key_name = CFString::from_static_string("kCGWindowName");
        let key_layer = CFString::from_static_string("kCGWindowLayer");

        let array_ref = unsafe {
            CGWindowListCopyWindowInfo(
                ON_SCREEN_ONLY | EXCLUDE_DESKTOP_ELEMENTS,
                NULL_WINDOW_ID,
            )
        };

        if array_ref.is_null() {
            return Err(AppError::State(
                "CGWindowListCopyWindowInfo returned null".into(),
            ));
        }

        let windows: CFArray<CFDictionary> = unsafe { CFArray::wrap_under_create_rule(array_ref) };

        let mut seen: HashSet<String> = HashSet::new();
        let mut apps: Vec<OpenApp> = Vec::new();

        for dict in windows.iter() {

            // Layer 0 = normal app windows; skip menubar/dock/overlays.
            if dict_i64(&dict, &key_layer).unwrap_or(-1) != 0 {
                continue;
            }

            let app = match dict_string(&dict, &key_owner) {
                Some(a) if !a.trim().is_empty() => a.trim().to_string(),
                _ => continue,
            };

            let title = dict_string(&dict, &key_name)
                .map(|t| t.trim().to_string())
                .unwrap_or_default();

            let id = format!("{app}{SEP}{title}");

            if !seen.insert(id.clone()) {
                continue;
            }

            let name = if title.is_empty() {
                app
            } else {
                format!("{app} - {title}")
            };

            apps.push(OpenApp { name, id });
        }

        apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(apps)
    }

    // id == "<process name>\u{1f}<window title>". Bring the process to the
    // front, then raise the specific window if a title is present.
    pub fn focus_app(id: &str) -> Result<(), AppError> {
        let (app, title) = id.split_once(SEP).unwrap_or((id, ""));

        if app.contains('"') || title.contains('"') {
            return Err(AppError::Validation("invalid app id".into()));
        }

        let raise = if title.is_empty() {
            String::new()
        } else {
            format!(
                "\nperform action \"AXRaise\" of (first window of process \"{app}\" whose name is \"{title}\")"
            )
        };

        let script = format!(
            "tell application \"System Events\"\n\
            set frontmost of process \"{app}\" to true{raise}\n\
            end tell"
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| AppError::State(format!("osascript failed: {e}")))?;

        if !output.status.success() {
            return Err(AppError::State(format!(
                "failed to focus app: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::OpenApp;
    use crate::error::AppError;
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    // List processes that own a visible main window. id = PID, name = window title.
    pub fn list_open_apps() -> Result<Vec<OpenApp>, AppError> {
        let script = "Get-Process | Where-Object { $_.MainWindowTitle -ne '' } | \
            ForEach-Object { \"$($_.Id)`t$($_.MainWindowTitle)\" }";

        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| AppError::State(format!("powershell failed: {e}")))?;

        if !output.status.success() {
            return Err(AppError::State(format!(
                "powershell error: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut apps: Vec<OpenApp> = stdout
            .lines()
            .filter_map(|l| {
                let (pid, title) = l.split_once('\t')?;
                let title = title.trim();
                if title.is_empty() {
                    return None;
                }
                Some(OpenApp {
                    name: title.to_string(),
                    id: pid.trim().to_string(),
                })
            })
            .collect();

        apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(apps)
    }

    // id == PID on Windows. AppActivate accepts a PID and raises the window.
    pub fn focus_app(id: &str) -> Result<(), AppError> {
        let pid: u32 = id
            .parse()
            .map_err(|_| AppError::Validation(format!("invalid app id: {id}")))?;

        let script = format!(
            "(New-Object -ComObject WScript.Shell).AppActivate({pid})"
        );

        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| AppError::State(format!("powershell failed: {e}")))?;

        if !output.status.success() {
            return Err(AppError::State(format!(
                "failed to focus app: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod platform {
    use super::OpenApp;
    use crate::error::AppError;

    pub fn list_open_apps() -> Result<Vec<OpenApp>, AppError> {
        Err(AppError::State("listing apps not supported on this platform".into()))
    }

    pub fn focus_app(_id: &str) -> Result<(), AppError> {
        Err(AppError::State("focusing apps not supported on this platform".into()))
    }
}
