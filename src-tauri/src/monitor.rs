use crate::{AppState, MAX_CLIP_BYTES};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const EVENT_CLIPBOARD_CHANGED: &str = "clipboard-changed";

pub fn start_clipboard_monitor(app: AppHandle) {

    let (tx, rx) = mpsc::channel::<String>();

    let state = app.state::<AppState>();
    let shutdown = state.shutdown.clone();

    {
        let mut guard = state.monitor_tx.lock().unwrap();
        *guard = Some(tx.clone());
    }

    let shutdown_poll = shutdown.clone();
    let shutdown_writer = shutdown.clone();

    let app_poll = app.clone();
    let app_writer = app.clone();
    let tx_poll = tx;

    let poll_handle = thread::spawn(move || {
        let mut last_text = String::new();

        loop {
            thread::sleep(Duration::from_millis(500));

            if shutdown_poll.load(Ordering::SeqCst) {
                break;
            }

            let text = match crate::commands::clipboard::read_clipboard_on_main_thread(&app_poll) {
                Ok(t) => t,
                Err(_) => continue,
            };

            if text.is_empty() || text == last_text || text.len() > MAX_CLIP_BYTES {
                continue;
            }

            last_text = text.clone();
            let _ = tx_poll.send(text);
        }
    });

    let writer_handle = thread::spawn(move || {
        while let Ok(text) = rx.recv() {
            if shutdown_writer.load(Ordering::SeqCst) {
                break;
            }

            let state = app_writer.state::<AppState>();

            let limit = state
                .settings
                .lock()
                .map(|s| s.history_limit)
                .unwrap_or(20) as i64;

            let result: rusqlite::Result<()> = match state.db_monitor.lock() {
                Err(e) => {
                    log::error!("clipboard monitor: db_monitor mutex poisoned: {e}");
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

                    // keep the most recent, exclude the others ...
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
                let _ = app_writer.emit(EVENT_CLIPBOARD_CHANGED, ());
            }
        }
    });

    let state = app.state::<AppState>();
    {
        let mut guard = state.monitor_handles.lock().unwrap();
        *guard = Some((poll_handle, writer_handle));
    }
}
