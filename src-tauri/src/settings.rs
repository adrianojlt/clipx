use crate::error::AppError;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn settings_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string()))
    } else {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".config")
    }
    .join("clipboard-manager")
}

pub fn settings_path() -> PathBuf {
    settings_dir().join("settings.json")
}

pub fn load_settings() -> HashMap<String, String> {
    let path = settings_path();
    if let Ok(content) = fs::read_to_string(&path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    }
}

pub fn save_settings(settings: &HashMap<String, String>) -> Result<(), AppError> {
    let dir = settings_dir();
    fs::create_dir_all(&dir)?;
    let path = dir.join("settings.json");
    let content =
        serde_json::to_string_pretty(settings).map_err(|e| AppError::Settings(e.to_string()))?;
    fs::write(&path, content)?;
    Ok(())
}

pub fn normalize_shortcut(s: &str) -> String {
    s.to_uppercase()
        .replace("OPTION", "ALT")
        .replace("META", "SUPER")
        .replace("COMMAND", "SUPER")
        .replace("CMD", "SUPER")
        .replace("CONTROL", "CTRL")
}
