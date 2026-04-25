use crate::AppState;
use arboard::Clipboard;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const EVENT_CLIPBOARD_CHANGED: &str = "clipboard-changed";

pub fn start_clipboard_monitor(app: AppHandle) {
    thread::spawn(move || {
        let mut clipboard = match Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to initialize clipboard: {}", e);
                return;
            }
        };

        let mut last_text = String::new();

        loop {
            thread::sleep(Duration::from_millis(500));

            let text = match clipboard.get_text() {
                Ok(t) => t,
                Err(_) => continue,
            };

            if text.is_empty() || text == last_text {
                continue;
            }

            last_text = text.clone();

            let state = app.state::<AppState>();
            let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());

            if let Ok(mut stmt) =
                conn.prepare_cached("DELETE FROM clipboard_history WHERE content = ?1")
            {
                let _ = stmt.execute([&text]);
            }

            if let Ok(mut stmt) =
                conn.prepare_cached("INSERT INTO clipboard_history (content) VALUES (?1)")
            {
                let _ = stmt.execute([&text]);
            }

            let limit = state.history_limit.lock().map(|l| *l).unwrap_or(20) as i64;
            if let Ok(mut stmt) = conn.prepare_cached(
                "DELETE FROM clipboard_history WHERE id NOT IN ( \
                    SELECT id FROM clipboard_history ORDER BY created_at DESC LIMIT ?1 \
                )",
            ) {
                let _ = stmt.execute([limit]);
            }

            let _ = app.emit(EVENT_CLIPBOARD_CHANGED, ());
        }
    });
}
