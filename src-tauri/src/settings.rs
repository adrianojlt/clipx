use crate::error::AppError;
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
pub fn settings_dir() -> Result<PathBuf, AppError> {
    let base = std::env::var("APPDATA")
        .map_err(|_| AppError::Settings("APPDATA env var is not set".to_string()))?;
    Ok(PathBuf::from(base).join("clipboard-manager"))
}

#[cfg(not(target_os = "windows"))]
pub fn settings_dir() -> Result<PathBuf, AppError> {
    let home = std::env::var("HOME")
        .map_err(|_| AppError::Settings("HOME env var is not set".to_string()))?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("clipboard-manager"))
}

pub fn settings_path() -> Result<PathBuf, AppError> {
    Ok(settings_dir()?.join("settings.json"))
}

pub fn load_settings() -> HashMap<String, String> {
    let path = match settings_path() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("settings: cannot resolve path ({e}), using empty settings");
            return HashMap::new();
        }
    };
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            eprintln!(
                "settings: failed to parse {} ({e}), using empty settings",
                path.display()
            );
            HashMap::new()
        }),
        Err(e) if e.kind() == ErrorKind::NotFound => HashMap::new(),
        Err(e) => {
            eprintln!(
                "settings: failed to read {} ({e}), using empty settings",
                path.display()
            );
            HashMap::new()
        }
    }
}

pub fn save_settings(settings: &HashMap<String, String>) -> Result<(), AppError> {
    let dir = settings_dir()?;
    fs::create_dir_all(&dir)?;
    let path = dir.join("settings.json");
    let content = serde_json::to_string_pretty(settings)?;
    fs::write(&path, content)?;
    Ok(())
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
}
