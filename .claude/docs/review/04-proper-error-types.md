# Task 04 - Add Proper Rust Error Types

**Severity:** High
**Category:** Rust / Error Handling
**Depends on:** Optionally do Task 05 (split lib.rs) first, but can be done standalone

## Why This Is a Problem

Every Rust function in this app returns `Result<T, String>`. Every error is converted with `.map_err(|e| e.to_string())`. This works but loses all information:

- You cannot tell "database error" from "file not found" programmatically
- Error context is discarded - you get a message but not what caused it
- The frontend receives raw Rust error text with no structure to act on
- This is not idiomatic Rust - it is the "I don't know what else to do" error pattern

The `thiserror` crate lets you define structured errors in about 10 lines.

## Files to Touch

- `src-tauri/Cargo.toml`
- `src-tauri/src/lib.rs`

## Exact Changes

### Step 1 - Add thiserror to Cargo.toml

```toml
thiserror = "1"
```

### Step 2 - Define AppError enum in lib.rs

Add near the top of `lib.rs` (after imports, before the structs):

```rust
#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path error: {0}")]
    Path(String),
    #[error("Settings error: {0}")]
    Settings(String),
    #[error("Shortcut error: {0}")]
    Shortcut(String),
}

// This makes AppError serializable for Tauri commands
impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
```

### Step 3 - Update command return types

Change all command signatures from:
```rust
fn get_history(app: AppHandle) -> Result<Vec<ClipboardItem>, String>
```
to:
```rust
fn get_history(app: AppHandle) -> Result<Vec<ClipboardItem>, AppError>
```

Do this for all 13 commands.

### Step 4 - Update internal helper function return types

Change functions like `init_db`, `db_path`, `save_settings`, etc.:
```rust
fn db_path(app: &AppHandle) -> Result<PathBuf, AppError>
fn init_db(app: &AppHandle) -> Result<(), AppError>
fn save_settings(settings: &HashMap<String, String>) -> Result<(), AppError>
```

### Step 5 - Remove most `.map_err(|e| e.to_string())`

With `#[from]` on each variant, the `?` operator handles conversion automatically:

```rust
// Before:
let conn = Connection::open(db_path(&app)?).map_err(|e| e.to_string())?;

// After:
let conn = Connection::open(db_path(&app)?)?;  // rusqlite::Error converts via #[from]
```

The `Path` and `Settings` and `Shortcut` variants still need manual mapping for cases without a `#[from]` source.

## How to Verify

```bash
cd src-tauri && cargo build 2>&1
```

All 13 commands must compile. Test a few commands from the UI to confirm error messages still appear when something goes wrong. The error text seen in the frontend should not change - only the internal structure changes.
