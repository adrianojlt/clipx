use crate::error::AppError;
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub hotkey: String,
    pub history_limit: u32,
    pub window_width: f64,
    pub window_height: f64,
    pub tab_shortcut_apps: String,
    pub tab_shortcut_pinned: String,
    pub tab_shortcut_history: String,
    pub tab_shortcut_sessions: String,
    pub tab_shortcut_find: String,
}

impl Default for Settings {
    fn default() -> Self {
        let tab_mod = if cfg!(target_os = "windows") { "Alt" } else { "Command" };
        Self {
            hotkey: "Option+Space".to_string(),
            history_limit: 20,
            window_width: 400.0,
            window_height: 600.0,
            tab_shortcut_apps: format!("{tab_mod}+1"),
            tab_shortcut_pinned: format!("{tab_mod}+2"),
            tab_shortcut_history: format!("{tab_mod}+3"),
            tab_shortcut_sessions: format!("{tab_mod}+4"),
            tab_shortcut_find: format!("{tab_mod}+F"),
        }
    }
}

impl Settings {

    pub fn validate(&mut self) {
        self.history_limit = self.history_limit.clamp(1, 500);
        self.window_width = self.window_width.clamp(300.0, 800.0);
        self.window_height = self.window_height.clamp(400.0, 900.0);
    }

    fn from_map(map: HashMap<String, String>) -> Self {

        let mut s = Self::default();

        if let Some(v) = map.get("hotkey") {
            s.hotkey = v.clone();
        }
        if let Some(raw) = map.get("history_limit") {
            match raw.parse() {
                Ok(n) => s.history_limit = n,
                Err(_) => log::warn!("settings migration: invalid history_limit {:?}, using default", raw),
            }
        }
        if let Some(raw) = map.get("window_width") {
            match raw.parse() {
                Ok(n) => s.window_width = n,
                Err(_) => log::warn!("settings migration: invalid window_width {:?}, using default", raw),
            }
        }
        if let Some(raw) = map.get("window_height") {
            match raw.parse() {
                Ok(n) => s.window_height = n,
                Err(_) => log::warn!("settings migration: invalid window_height {:?}, using default", raw),
            }
        }
        if let Some(v) = map.get("tab_shortcut_apps") {
            s.tab_shortcut_apps = v.clone();
        }
        if let Some(v) = map.get("tab_shortcut_pinned") {
            s.tab_shortcut_pinned = v.clone();
        }
        if let Some(v) = map.get("tab_shortcut_history") {
            s.tab_shortcut_history = v.clone();
        }
        if let Some(v) = map.get("tab_shortcut_sessions") {
            s.tab_shortcut_sessions = v.clone();
        }
        if let Some(v) = map.get("tab_shortcut_find") {
            s.tab_shortcut_find = v.clone();
        }
        s
    }
}

pub fn settings_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Path(format!("Cannot resolve app data dir: {e}")))?
        .join("settings.json"))
}

#[cfg(target_os = "windows")]
fn legacy_settings_path() -> Result<PathBuf, AppError> {
    let base = std::env::var("APPDATA")
        .map_err(|_| AppError::Settings("APPDATA env var is not set".to_string()))?;
    Ok(PathBuf::from(base)
        .join("clipboard-manager")
        .join("settings.json"))
}

#[cfg(not(target_os = "windows"))]
fn legacy_settings_path() -> Result<PathBuf, AppError> {
    let home = std::env::var("HOME")
        .map_err(|_| AppError::Settings("HOME env var is not set".to_string()))?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("clipboard-manager")
        .join("settings.json"))
}

pub fn migrate_legacy_settings(app: &AppHandle) -> Result<(), AppError> {

    let new_path = settings_path(app)?;

    if new_path.exists() {
        return Ok(());
    }

    let legacy = match legacy_settings_path() {
        Ok(p) => p,
        Err(_) => return Ok(()),
    };

    if !legacy.exists() {
        return Ok(());
    }

    if let Some(parent) = new_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(&legacy, &new_path)?;
    let renamed = legacy.with_extension("json.legacy");
    fs::rename(&legacy, &renamed)?;

    log::info!(
        "settings: migrated {} -> {} (legacy renamed to {})",
        legacy.display(),
        new_path.display(),
        renamed.display()
    );

    Ok(())
}

fn corrupt_backup_path(path: &Path) -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    path.with_extension(format!("json.corrupt-{ts}"))
}

fn read_settings_file(path: &Path) -> Settings {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == ErrorKind::NotFound => return Settings::default(),
        Err(e) => {
            log::warn!(
                "settings: failed to read {} ({e}), using defaults",
                path.display()
            );
            return Settings::default();
        }
    };

    // Try new typed format first.
    if let Ok(mut s) = serde_json::from_str::<Settings>(&content) {
        s.validate();
        return s;
    }

    // Fall back to old string-map format (migrates existing installs).
    if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&content) {
        let mut s = Settings::from_map(map);
        s.validate();
        if let Err(e) = write_settings_file(path, &s) {
            log::warn!("settings: failed to upgrade old format: {e}");
        }
        return s;
    }

    // Truly corrupt, back up and use defaults.
    let backup = corrupt_backup_path(path);
    match fs::rename(path, &backup) {
        Ok(_) => log::warn!(
            "settings: failed to parse {}; backed up to {}, using defaults",
            path.display(),
            backup.display()
        ),
        Err(rename_err) => log::warn!(
            "settings: failed to parse {}; backup rename failed ({rename_err}), using defaults",
            path.display()
        ),
    }
    Settings::default()
}

fn write_settings_file(path: &Path, settings: &Settings) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(settings)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn load_settings(app: &AppHandle) -> Settings {
    let path = match settings_path(app) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("settings: cannot resolve path ({e}), using defaults");
            return Settings::default();
        }
    };
    read_settings_file(&path)
}

pub fn save_settings(app: &AppHandle, settings: &Settings) -> Result<(), AppError> {
    let path = settings_path(app)?;
    write_settings_file(&path, settings)
}

pub fn normalize_shortcut(s: &str) -> String {
    s.split('+')
        .map(|tok| match tok.trim().to_uppercase().as_str() {
            "OPTION" => "ALT".to_string(),
            "META" | "COMMAND" | "CMD" => "SUPER".to_string(),
            "CONTROL" => "CTRL".to_string(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join("+")
}


#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn normalize_shortcut_handles_option() {
        assert_eq!(normalize_shortcut("option+space"), "ALT+SPACE");
    }

    #[test]
    fn normalize_shortcut_maps_command_variants() {
        assert_eq!(normalize_shortcut("Cmd+K"), "SUPER+K");
        assert_eq!(normalize_shortcut("Command+K"), "SUPER+K");
        assert_eq!(normalize_shortcut("Meta+K"), "SUPER+K");
    }

    #[test]
    fn normalize_shortcut_passes_through_unknown_tokens() {
        assert_eq!(normalize_shortcut("Shift+F1"), "SHIFT+F1");
    }

    #[test]
    fn normalize_shortcut_empty_returns_empty() {
        assert_eq!(normalize_shortcut(""), "");
    }

    #[test]
    fn settings_round_trip() {
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");

        let mut original = Settings::default();
        original.hotkey = "ALT+SPACE".to_string();
        original.history_limit = 15;

        write_settings_file(&path, &original).unwrap();
        let loaded = read_settings_file(&path);

        assert_eq!(loaded.hotkey, "ALT+SPACE");
        assert_eq!(loaded.history_limit, 15);
    }

    #[test]
    fn write_settings_file_is_atomic() {
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");
        let mut s = Settings::default();
        s.hotkey = "ALT+SPACE".to_string();

        write_settings_file(&path, &s).unwrap();
        assert!(path.exists());
        assert!(!dir.path().join("settings.json.tmp").exists());

        let loaded = read_settings_file(&path);
        assert_eq!(loaded.hotkey, "ALT+SPACE");
    }

    #[test]
    fn read_settings_file_backs_up_corrupt_content() {
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, "{not valid json").unwrap();

        let loaded = read_settings_file(&path);
        assert_eq!(loaded.hotkey, Settings::default().hotkey);
        assert!(!path.exists(), "corrupt file should have been renamed");

        let backups: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("settings.json.corrupt-")
            })
            .collect();
        assert_eq!(backups.len(), 1, "expected exactly one corrupt backup");
    }

    #[test]
    fn read_settings_file_migrates_old_string_map_format() {

        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");

        // Old format: all values are JSON strings
        fs::write(
            &path,
            r#"{"hotkey":"ALT+SPACE","history_limit":"15","window_width":"500","window_height":"700","tab_shortcut_pinned":"Command+1","tab_shortcut_history":"Command+2"}"#,
        )
        .unwrap();

        let loaded = read_settings_file(&path);

        assert_eq!(loaded.hotkey, "ALT+SPACE");
        assert_eq!(loaded.history_limit, 15);
        assert_eq!(loaded.window_width, 500.0);
        assert!(!path.with_extension("json.corrupt-0").exists());
    }

    #[test]
    fn validate_clamps_all_fields() {

        let mut s = Settings {
            hotkey: "X".to_string(),
            history_limit: 200,
            window_width: 50.0,
            window_height: 5000.0,
            tab_shortcut_apps: "E".to_string(),
            tab_shortcut_pinned: "A".to_string(),
            tab_shortcut_history: "B".to_string(),
            tab_shortcut_sessions: "D".to_string(),
            tab_shortcut_find: "C".to_string(),
        };

        s.validate();
        assert_eq!(s.history_limit, 200);
        assert_eq!(s.window_width, 300.0);
        assert_eq!(s.window_height, 900.0);
    }
}
