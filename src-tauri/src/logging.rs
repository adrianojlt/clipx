use crate::error::AppError;
use log::LevelFilter;
use simplelog::{Config, WriteLogger};
use std::fs::OpenOptions;

pub fn init_logging(app_data_dir: &std::path::Path) -> Result<(), AppError> {
    let log_dir = app_data_dir.join("logs");
    std::fs::create_dir_all(&log_dir).map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create log directory: {e}"),
        ))
    })?;

    let log_path = log_dir.join("clipx.log");
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| {
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to open log file: {e}"),
            ))
        })?;

    WriteLogger::init(LevelFilter::Info, Config::default(), log_file).map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to init logger: {e}"),
        ))
    })?;

    log::info!("Logging initialized. Log file: {}", log_path.display());
    Ok(())
}
