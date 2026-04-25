# Task 01 - Fix unwrap() Panics in Rust Backend

**Severity:** Critical
**Category:** Rust
**Depends on:** Nothing - do this first

## Why This Is a Problem

In Rust, `unwrap()` means "get the value or crash the entire program immediately with no recovery." The app has four places where this can crash for real users:

- If the OS cannot give the app its data directory (can happen in restricted environments, Docker, sandboxes)
- If the internal mutex gets corrupted (can happen if a thread crashes while holding a lock)

These are not hypothetical. They are time bombs.

## Files to Touch

- `src-tauri/src/lib.rs`

## Exact Changes

### Line 111 - db_path function

**Current:**
```rust
fn db_path(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap().join("clipx.db")
}
```

**Fix:** Change the return type to `Result<PathBuf, String>` and use `?`:
```rust
fn db_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_data_dir()
        .map_err(|e| format!("Cannot resolve app data dir: {e}"))?
        .join("clipx.db"))
}
```

Then update every call to `db_path(&app)` to `db_path(&app)?`. There are roughly 10 call sites - each is inside a function that already returns `Result<_, String>`, so `?` works directly.

### Line 517 - setup() function

**Current:**
```rust
std::fs::create_dir_all(app.path().app_data_dir().unwrap())?;
```

**Fix:**
```rust
let data_dir = app.path().app_data_dir()
    .map_err(|e| format!("Cannot resolve app data dir: {e}"))?;
std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
```

### Line 543 - update_shortcut command

**Current:**
```rust
let mut current = state.current_shortcut.lock().unwrap();
```

**Fix:**
```rust
let mut current = state.current_shortcut.lock().map_err(|e| e.to_string())?;
```

### Line 619 - run() function

**Current:**
```rust
.expect("error while running tauri application")
```

This one is in `run()` which returns `()`, so you cannot use `?`. Leave `.expect()` here - it is acceptable at the top-level entry point where there is nothing else to do if Tauri fails to start.

## How to Verify

```bash
cd src-tauri && cargo build 2>&1
```

The build must succeed with zero errors. No behavior changes - this is purely defensive.
