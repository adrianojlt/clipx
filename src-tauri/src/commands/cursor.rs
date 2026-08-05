use crate::error::AppError;

use tauri::Manager;

/// A point in the main window's client coordinates, in CSS pixels.
#[derive(serde::Serialize)]
pub struct CursorPoint {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

/// Where the mouse is, relative to the popup's client area.
///
/// The frontend needs this to hit-test the row under the cursor when the
/// open-apps chord is released. Reading it here rather than from webview pointer
/// events is what makes the lookup work when the mouse never moved after the
/// popup appeared, so no pointer event ever fired.
#[tauri::command]
pub fn get_cursor_client_position(app: tauri::AppHandle) -> Result<CursorPoint, AppError> {

    let win = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::Window("main window not found".into()))?;

    let cursor = app
        .cursor_position()
        .map_err(|e| AppError::Window(format!("cannot read cursor position: {e}")))?;

    let origin = win
        .inner_position()
        .map_err(|e| AppError::Window(format!("cannot read window position: {e}")))?;

    let scale = win
        .scale_factor()
        .map_err(|e| AppError::Window(format!("cannot read scale factor: {e}")))?;

    Ok(to_client_point(
        (cursor.x, cursor.y),
        (origin.x, origin.y),
        scale,
    ))
}

fn to_client_point(cursor: (f64, f64), origin: (i32, i32), scale: f64) -> CursorPoint {
    CursorPoint {
        x: (cursor.0 - origin.0 as f64) / scale,
        y: (cursor.1 - origin.1 as f64) / scale,
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn to_client_point_subtracts_the_origin_then_divides_by_the_scale() {
        let p = to_client_point((1100.0, 640.0), (1000, 400), 2.0);
        assert_eq!((p.x, p.y), (50.0, 120.0));
    }

    #[test]
    fn to_client_point_is_negative_when_the_cursor_left_the_window() {
        let p = to_client_point((900.0, 400.0), (1000, 400), 2.0);
        assert_eq!((p.x, p.y), (-50.0, 0.0));
    }
}
