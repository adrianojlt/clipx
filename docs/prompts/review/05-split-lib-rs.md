# Task 05 - Split lib.rs Into Modules

**Severity:** High
**Category:** Rust / Project Structure
**Depends on:** Recommended to do Task 01 and Task 04 first so the split starts from a cleaner base

## Why This Is a Problem

The entire backend - 621 lines - lives in one file. This includes database setup, migrations, clipboard monitoring, 13 command handlers, settings file management, and app initialization. Every change touches the same file, making merge conflicts and bugs more likely. Rust has a module system specifically to prevent this.

## Files to Touch

New files to create:
- `src-tauri/src/error.rs`
- `src-tauri/src/db.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/monitor.rs`
- `src-tauri/src/commands/clipboard.rs`
- `src-tauri/src/commands/pinned.rs`
- `src-tauri/src/commands/settings.rs`

Modify:
- `src-tauri/src/lib.rs` (becomes thin wiring file only)

## Target Structure

```
src-tauri/src/
  main.rs               (untouched, 7 lines)
  lib.rs                (only: pub mod declarations + run() + AppState)
  error.rs              (AppError enum from Task 04)
  db.rs                 (db_path, init_db, migrations)
  settings.rs           (settings_dir, settings_path, load_settings, save_settings, normalize_shortcut)
  monitor.rs            (start_clipboard_monitor)
  commands/
    mod.rs              (re-exports all commands)
    clipboard.rs        (get_history, get_clipboard, delete_history_item)
    pinned.rs           (get_pinned, pin_item, reorder_pinned, update_pinned_description, unpin_item, toggle_pinned_hidden)
    settings.rs         (get_setting, set_setting, update_shortcut, apply_window_size)
```

## What Goes Where

### error.rs
Move the `AppError` enum (from Task 04).

### db.rs
Move:
- `db_path()` function
- `init_db()` function (including all SQL CREATE TABLE and migration statements)

### settings.rs
Move:
- `settings_dir()`
- `settings_path()`
- `load_settings()`
- `save_settings()`
- `normalize_shortcut()`

### monitor.rs
Move:
- `start_clipboard_monitor()`

### commands/clipboard.rs
Move:
- `get_history`
- `get_clipboard`
- `delete_history_item`

### commands/pinned.rs
Move:
- `get_pinned`
- `pin_item`
- `reorder_pinned`
- `update_pinned_description`
- `unpin_item`
- `toggle_pinned_hidden`

### commands/settings.rs
Move:
- `get_setting`
- `set_setting`
- `update_shortcut`
- `apply_window_size`

### lib.rs (what remains)
```rust
mod error;
mod db;
mod settings;
mod monitor;
mod commands;

use tauri::Manager;
// AppState struct
// run() function with invoke_handler and setup
```

## How to Verify

```bash
cd src-tauri && cargo build 2>&1
```

Zero errors. Run the app and exercise every feature: copy items, pin items, reorder, edit descriptions, open settings, change hotkey. Everything must work identically to before.

## Tips

- Add `pub use` in each module for the items that `lib.rs` needs to see
- The `commands/mod.rs` can re-export all handlers: `pub use clipboard::*; pub use pinned::*; pub use settings::*;`
- Use `use crate::db::db_path;` inside command files to reference sibling modules
- Do one module at a time and run `cargo build` after each move to catch problems early
