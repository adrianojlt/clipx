use log::Level;
use tauri::command;

#[command]
pub fn log_frontend_error(level: String, message: String) {
    let level = match level.as_str() {
        "error" => Level::Error,
        "warn" => Level::Warn,
        "info" => Level::Info,
        "debug" => Level::Debug,
        _ => Level::Info,
    };
    log::log!(level, "[frontend] {}", message);
}
