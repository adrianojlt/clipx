use crate::settings::{load_settings, load_window_size};
use tauri::{Manager, Monitor, PhysicalPosition, WebviewWindow};
use tauri_plugin_global_shortcut::{Shortcut, ShortcutEvent, ShortcutState};

pub(crate) fn shortcut_handler(app: &tauri::AppHandle, _shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state() != ShortcutState::Pressed {
        return;
    }
    let Some(win) = app.get_webview_window("main") else {
        return;
    };

    let (width, height) = load_window_size(&load_settings());

    let Ok(cursor) = app.cursor_position() else {
        return;
    };

    let monitor = monitor_under_point(&win, cursor.x as i32, cursor.y as i32);
    let (x, y) = clamp_to_monitor(
        cursor.x as i32,
        cursor.y as i32,
        width,
        height,
        monitor.as_ref(),
    );

    let _ = win.set_size(tauri::Size::Logical(tauri::LogicalSize { width, height }));
    let _ = win.set_position(tauri::Position::Physical(PhysicalPosition { x, y }));
    let _ = win.show();
    let _ = win.set_focus();
}

pub(crate) fn monitor_under_point(win: &WebviewWindow, x: i32, y: i32) -> Option<Monitor> {
    win.available_monitors()
        .ok()
        .and_then(|mons| {
            mons.into_iter().find(|m| {
                let p = m.position();
                let s = m.size();
                x >= p.x && x < p.x + s.width as i32 && y >= p.y && y < p.y + s.height as i32
            })
        })
        .or_else(|| win.primary_monitor().ok().flatten())
}

pub(crate) fn clamp_to_monitor(
    mut x: i32,
    mut y: i32,
    logical_w: f64,
    logical_h: f64,
    monitor: Option<&Monitor>,
) -> (i32, i32) {
    if let Some(m) = monitor {
        let scale = m.scale_factor();
        let win_w = (logical_w * scale).round() as i32;
        let win_h = (logical_h * scale).round() as i32;
        let p = m.position();
        let s = m.size();
        x = x.min(p.x + s.width as i32 - win_w).max(p.x);
        y = y.min(p.y + s.height as i32 - win_h).max(p.y);
    }
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_to_monitor_without_monitor_is_passthrough() {
        assert_eq!(clamp_to_monitor(123, 456, 400.0, 600.0, None), (123, 456));
    }
}
