//! Stubs for platforms with no app-switcher support.

use super::OpenAppsResult;
use crate::error::AppError;

pub fn list_open_apps() -> Result<OpenAppsResult, AppError> {
    Err(AppError::State("listing apps not supported on this platform".into()))
}

pub fn focus_app(_id: &str) -> Result<(), AppError> {
    Err(AppError::State("focusing apps not supported on this platform".into()))
}

pub fn cycle_active_app_windows() -> Result<bool, AppError> {
    Err(AppError::State("cycling app windows not supported on this platform".into()))
}
