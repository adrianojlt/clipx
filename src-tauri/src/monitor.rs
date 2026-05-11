use crate::{AppState, MAX_CLIP_BYTES};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

const EVENT_CLIPBOARD_CHANGED: &str = "clipboard-changed";

// Code responsible to check for changes in clipboard ...
// Polling was chosen because no portable clipboard change event exists,
// and macOS doesn't expose one at all
pub fn start_clipboard_monitor(app: AppHandle) {

    let (tx, rx) = mpsc::channel::<String>();

    let shutdown = app.state::<AppState>().shutdown.clone();
    let shutdown_poll = shutdown.clone();

    // Polling thread: reads clipboard every 500ms, sends new content to the writer
    let app_poll = app.clone();

    thread::spawn(move || {

        let mut last_text = String::new();

        loop {

            // we will check every 500ms for new content in the clipboard
            thread::sleep(Duration::from_millis(500));

            if shutdown_poll.load(Ordering::Relaxed) {
                break;
            }

            let text = match app_poll.clipboard().read_text() {
                Ok(t) => t,
                Err(_) => continue,
            };

            if text.is_empty() || text == last_text {
                continue;
            }

            // content didn't change? do nothing ...
            if text.len() > MAX_CLIP_BYTES {
                continue;
            }

            last_text = text.clone();
            let _ = tx.send(text);
        }
    });

    // Writer thread: persists clipboard changes to DB and emits the changed event
    thread::spawn(move || {

        while let Ok(text) = rx.recv() {

            let state = app.state::<AppState>();

            // Get history limit from state, default to 20 if lock fails
            let limit = state.settings.lock().map(|s| s.history_limit).unwrap_or(20) as i64;

            let result: rusqlite::Result<()> = match state.db.lock() {
                Err(e) => {
                    log::error!("clipboard monitor: db mutex poisoned: {e}");
                    continue;
                }
                Ok(mut conn) => (|| {

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
                })(),
            };

            if let Err(e) = result {
                log::warn!("clipboard monitor: transaction failed: {e}");
            } else {
                let _ = app.emit(EVENT_CLIPBOARD_CHANGED, ());
            }
        }
    });
}
