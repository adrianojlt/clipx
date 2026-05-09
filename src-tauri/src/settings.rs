use crate::error::AppError;
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

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

fn read_settings_file(path: &Path) -> HashMap<String, String> {
    match fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str::<HashMap<String, String>>(&content) {
            Ok(s) => s,
            Err(e) => {
                let backup = corrupt_backup_path(path);
                match fs::rename(path, &backup) {
                    Ok(_) => log::warn!(
                        "settings: failed to parse {} ({e}); backed up to {}, using empty settings",
                        path.display(),
                        backup.display()
                    ),
                    Err(rename_err) => log::warn!(
                        "settings: failed to parse {} ({e}); backup rename failed ({rename_err}), using empty settings",
                        path.display()
                    ),
                }
                HashMap::new()
            }
        },
        Err(e) if e.kind() == ErrorKind::NotFound => HashMap::new(),
        Err(e) => {
            log::warn!(
                "settings: failed to read {} ({e}), using empty settings",
                path.display()
            );
            HashMap::new()
        }
    }
}

fn write_settings_file(path: &Path, settings: &HashMap<String, String>) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(settings)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn load_settings(app: &AppHandle) -> HashMap<String, String> {
    let path = match settings_path(app) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("settings: cannot resolve path ({e}), using empty settings");
            return HashMap::new();
        }
    };
    read_settings_file(&path)
}

pub fn save_settings(app: &AppHandle, settings: &HashMap<String, String>) -> Result<(), AppError> {
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

pub fn load_window_size(settings: &HashMap<String, String>) -> (f64, f64) {
    let width = settings
        .get("window_width")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(400.0)
        .clamp(300.0, 800.0);
    let height = settings
        .get("window_height")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(600.0)
        .clamp(400.0, 900.0);
    (width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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
    fn load_window_size_defaults() {
        let s = HashMap::new();
        assert_eq!(load_window_size(&s), (400.0, 600.0));
    }

    #[test]
    fn load_window_size_clamps_upper() {
        let mut s = HashMap::new();
        s.insert("window_width".to_string(), "9999".to_string());
        s.insert("window_height".to_string(), "9999".to_string());
        assert_eq!(load_window_size(&s), (800.0, 900.0));
    }

    #[test]
    fn load_window_size_clamps_lower() {
        let mut s = HashMap::new();
        s.insert("window_width".to_string(), "10".to_string());
        s.insert("window_height".to_string(), "10".to_string());
        assert_eq!(load_window_size(&s), (300.0, 400.0));
    }

    #[test]
    fn load_window_size_garbage_falls_back_to_defaults() {
        let mut s = HashMap::new();
        s.insert("window_width".to_string(), "not-a-number".to_string());
        assert_eq!(load_window_size(&s), (400.0, 600.0));
    }

    #[test]
    fn settings_round_trip() {
        use std::fs;
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");

        let mut original = HashMap::new();
        original.insert("hotkey".to_string(), "ALT+SPACE".to_string());
        original.insert("history_limit".to_string(), "15".to_string());

        let json = serde_json::to_string(&original).unwrap();
        fs::write(&path, &json).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let loaded: HashMap<String, String> = serde_json::from_str(&content).unwrap();

        assert_eq!(loaded.get("hotkey").unwrap(), "ALT+SPACE");
        assert_eq!(loaded.get("history_limit").unwrap(), "15");
    }

    #[test]
    fn write_settings_file_is_atomic() {
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");
        let mut original = HashMap::new();
        original.insert("hotkey".to_string(), "ALT+SPACE".to_string());

        write_settings_file(&path, &original).unwrap();
        assert!(path.exists());
        assert!(!dir.path().join("settings.json.tmp").exists());

        let loaded = read_settings_file(&path);
        assert_eq!(loaded.get("hotkey").unwrap(), "ALT+SPACE");
    }

    #[test]
    fn read_settings_file_backs_up_corrupt_content() {
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, "{not valid json").unwrap();

        let loaded = read_settings_file(&path);
        assert!(loaded.is_empty());
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
}
