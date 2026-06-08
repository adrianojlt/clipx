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
    use std::collections::HashSet;
    use std::process::Command;

    // Separator packed into `id` to carry both process name and window title.
    const SEP: char = '\u{1f}';

    // List windows via each app's Window menu (Accessibility). An app's open
    // windows are the last N items of its Window menu (N = AX window count),
    // which are exactly the strings focus_app clicks - so list and focus always
    // agree, including for Chromium/Electron apps. Needs Accessibility only.
    // Output lines: "<process name>\t<window title>" (title empty if none).
    pub fn list_open_apps() -> Result<Vec<OpenApp>, AppError> {
        let script = "set output to \"\"\n\
            tell application \"System Events\"\n\
            repeat with p in (every process whose background only is false)\n\
            set pname to name of p\n\
            set emitted to false\n\
            try\n\
            set allItems to name of every menu item of menu 1 of (menu bar item \"Window\" of menu bar 1 of p)\n\
            set total to count of allItems\n\
            set sepIndex to 0\n\
            repeat with i from 1 to total\n\
            if item i of allItems is missing value then set sepIndex to i\n\
            end repeat\n\
            if sepIndex > 0 and sepIndex < total then\n\
            repeat with i from (sepIndex + 1) to total\n\
            set t to item i of allItems\n\
            if t is not missing value then\n\
            set output to output & pname & tab & (t as text) & linefeed\n\
            set emitted to true\n\
            end if\n\
            end repeat\n\
            end if\n\
            end try\n\
            if not emitted then\n\
            set output to output & pname & tab & \"\" & linefeed\n\
            end if\n\
            end repeat\n\
            end tell\n\
            return output";

        let out = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| AppError::State(format!("osascript failed: {e}")))?;

        if !out.status.success() {
            return Err(AppError::State(format!(
                "osascript error: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }

        let stdout = String::from_utf8_lossy(&out.stdout);
        let mut seen: HashSet<String> = HashSet::new();
        let mut apps: Vec<OpenApp> = Vec::new();

        for line in stdout.lines() {
            let (app, title) = line.split_once('\t').unwrap_or((line, ""));
            let app = app.trim();
            let title = title.trim();
            if app.is_empty() {
                continue;
            }
            let id = format!("{app}{SEP}{title}");
            if !seen.insert(id.clone()) {
                continue;
            }
            let name = if title.is_empty() {
                app.to_string()
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

        // Raise the window via the app's own Window menu item. AXRaise/AXMain
        // are ignored by Chromium/Electron apps (Chrome, VS Code) for real
        // focus, but clicking the Window-menu entry is honored everywhere.
        // Exact title match first, then substring (Chrome decorates titles).
        // Select the target window via the Window menu BEFORE activating the
        // app, so the app comes forward already showing the right window (no
        // flash of the previously-active window).
        let inner = if title.is_empty() {
            "set frontmost to true".to_string()
        } else {
            format!(
                "try\n\
                click (first menu item of menu 1 of menu bar item \"Window\" of menu bar 1 whose name is \"{title}\")\n\
                on error\n\
                try\n\
                click (first menu item of menu 1 of menu bar item \"Window\" of menu bar 1 whose name contains \"{title}\")\n\
                end try\n\
                end try\n\
                set frontmost to true"
            )
        };

        let script = format!(
            "tell application \"System Events\"\n\
            tell process \"{app}\"\n\
            {inner}\n\
            end tell\n\
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
