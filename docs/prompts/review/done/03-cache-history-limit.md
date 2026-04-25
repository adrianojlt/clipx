# Task 03 - Cache history_limit in AppState

**Severity:** High
**Category:** Rust / Performance
**Depends on:** Nothing - independent

## Why This Is a Problem

The clipboard monitor runs in a background thread and polls every 500 milliseconds, forever. On every tick it reads the `history_limit` setting from the JSON settings file on disk. That is 2 disk reads per second, all day, even when the user is not doing anything.

This is wasteful. The setting almost never changes - only when the user opens Settings and saves. Reading from disk on every tick also slows down the clipboard monitor thread unnecessarily.

## Files to Touch

- `src-tauri/src/lib.rs`

## Exact Changes

### Step 1 - Add history_limit to AppState

**Current:**
```rust
struct AppState {
    current_shortcut: Mutex<String>,
}
```

**Fix:**
```rust
struct AppState {
    current_shortcut: Mutex<String>,
    history_limit: Mutex<u32>,
}
```

### Step 2 - Initialize it in run()

Find where `AppState` is created (around line 494):

**Current:**
```rust
.manage(AppState {
    current_shortcut: Mutex::new(String::new()),
})
```

**Fix:**
```rust
let initial_limit = {
    let settings = load_settings();
    settings.get("history_limit")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(20)
        .clamp(1, 50)
};

.manage(AppState {
    current_shortcut: Mutex::new(String::new()),
    history_limit: Mutex::new(initial_limit),
})
```

### Step 3 - Update the history_limit in set_setting

In the `set_setting` command, after saving the setting, update the AppState cache if the key is `history_limit`:

```rust
#[tauri::command]
fn set_setting(key: String, value: String, state: State<AppState>) -> Result<(), String> {
    let mut settings = load_settings();
    settings.insert(key.clone(), value.clone());
    save_settings(&settings)?;

    if key == "history_limit" {
        if let Ok(limit) = value.parse::<u32>() {
            if let Ok(mut cached) = state.history_limit.lock() {
                *cached = limit.clamp(1, 50);
            }
        }
    }

    Ok(())
}
```

Note: `set_setting` needs `state: State<AppState>` added to its parameters. Add it to both the function signature and the `invoke_handler` registration (the handler registration does not need changes - Tauri resolves State automatically).

### Step 4 - Use the cached value in the clipboard monitor

In `start_clipboard_monitor`, pass `AppState` or `AppHandle`. The function already takes `AppHandle`. You can get the state from it:

In the monitor loop, replace the disk read:
```rust
// Remove this disk read:
let limit = { ... load settings from disk ... };

// Replace with:
let limit = app.state::<AppState>().history_limit.lock()
    .map(|l| *l)
    .unwrap_or(20);
```

## How to Verify

```bash
cd src-tauri && cargo build 2>&1
```

Copy several items to clipboard. Open Activity Monitor and confirm no unusual disk I/O from the app process. Changing the history limit in Settings should still take effect on the next clipboard change.
