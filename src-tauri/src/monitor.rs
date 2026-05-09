use crate::{AppState, MAX_CLIP_BYTES};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

const EVENT_CLIPBOARD_CHANGED: &str = "clipboard-changed";

pub fn start_clipboard_monitor(app: AppHandle) {
    thread::spawn(move || {
        let mut last_text = String::new();

        loop {
            thread::sleep(Duration::from_millis(500));

            let text = match app.clipboard().read_text() {
                Ok(t) => t,
                Err(_) => continue,
            };

            if text.is_empty() || text == last_text {
                continue;
            }

            if text.len() > MAX_CLIP_BYTES {
                last_text = text;
                continue;
            }

            last_text = text.clone();

            let state = app.state::<AppState>();
            let limit = state.history_limit.lock().map(|l| *l).unwrap_or(20) as i64;
            let mut conn = state.db.lock().unwrap_or_else(|e| e.into_inner());

            let result: rusqlite::Result<()> = (|| {

                let tx = conn.transaction()?;

                tx.execute(
                    "DELETE FROM clipboard_history WHERE content = ?1",
                    [&text],
                )?;

                tx.execute(
                    "INSERT INTO clipboard_history (content) VALUES (?1)",
                    [&text],
                )?;

                tx.execute(
                    "DELETE FROM clipboard_history WHERE id NOT IN ( \
                        SELECT id FROM clipboard_history ORDER BY created_at DESC LIMIT ?1 \
                    )",
                    [limit],
                )?;

                tx.commit()
            })();

            if let Err(e) = result {
                log::warn!("clipboard monitor: transaction failed: {e}");
                continue;
            }

            drop(conn);

            let _ = app.emit(EVENT_CLIPBOARD_CHANGED, ());
        }
    });
}
