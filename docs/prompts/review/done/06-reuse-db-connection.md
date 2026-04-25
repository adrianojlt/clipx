# Task 06 - Reuse Database Connection Instead of Opening Per Command

**Severity:** Medium
**Category:** Rust / Performance
**Depends on:** Task 03 (AppState already touched) - do that first

## Why This Is a Problem

Every Tauri command opens a brand-new SQLite connection:
```rust
let conn = Connection::open(db_path(&app)?)?;
```
Then the connection is dropped at the end of the function. This means:
- A new file handle is opened and closed on every user action
- Connection setup overhead adds latency to every command
- The clipboard monitor opens a new connection every 500ms in a loop (2/second, forever)

SQLite supports a single-writer, multiple-reader model. The simplest fix is to share one connection via a Mutex in AppState.

## Files to Touch

- `src-tauri/src/lib.rs` (or `src-tauri/src/db.rs` if Task 05 was done)

## Exact Changes

### Step 1 - Add connection to AppState

```rust
use rusqlite::Connection;
use std::sync::Mutex;

struct AppState {
    current_shortcut: Mutex<String>,
    history_limit: Mutex<u32>,          // from Task 03
    db: Mutex<Connection>,
}
```

### Step 2 - Initialize connection in setup()

In the `setup()` closure (where `.manage(AppState {...})` is called):

```rust
let db_file = db_path(app.handle())?;
init_db(app.handle())?;  // still run migrations first
let conn = Connection::open(&db_file).map_err(|e| e.to_string())?;

app.manage(AppState {
    current_shortcut: Mutex::new(String::new()),
    history_limit: Mutex::new(initial_limit),
    db: Mutex::new(conn),
});
```

### Step 3 - Update command signatures to use state

Commands that hit the database need `state: State<AppState>`. Most already receive `app: AppHandle` - add state next to it:

```rust
#[tauri::command]
fn get_history(app: AppHandle, state: State<AppState>) -> Result<Vec<ClipboardItem>, AppError> {
    let conn = state.db.lock().map_err(|e| AppError::Settings(e.to_string()))?;
    // use conn directly, no Connection::open() call
}
```

### Step 4 - Update clipboard monitor

Pass `AppState` to the monitor or use `app.state::<AppState>()` inside the thread:

```rust
fn start_clipboard_monitor(app: AppHandle) {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(500));
            // ...
            let state = app.state::<AppState>();
            let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
            // use conn
        }
    });
}
```

## Tradeoff to Know

A `Mutex<Connection>` means only one command can use the database at a time. For this app that is fine - user interactions don't overlap. If you ever add heavy concurrent queries, consider `r2d2` or `deadpool` for connection pooling. For now, Mutex is simpler and sufficient.

## How to Verify

```bash
cd src-tauri && cargo build 2>&1
```

Run the app. Copy items, pin, reorder, delete, change settings. All features must work. Check Activity Monitor - the app should show fewer file system events than before.
