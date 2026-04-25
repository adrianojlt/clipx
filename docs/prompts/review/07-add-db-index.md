# Task 07 - Add Database Index on clipboard_history.created_at

**Severity:** Medium
**Category:** Rust / Database
**Depends on:** Nothing - independent

## Why This Is a Problem

The most frequent query in the app is:
```sql
SELECT id, content, created_at FROM clipboard_history ORDER BY created_at DESC LIMIT N
```

This runs every time the user opens the app and every time the clipboard changes. Without an index on `created_at`, SQLite scans every row in the table to sort and find the top N results. As history grows, this gets slower.

SQLite indexes make this query O(log n) instead of O(n).

## Files to Touch

- `src-tauri/src/lib.rs` (or `src-tauri/src/db.rs` if Task 05 was done)

## Exact Changes

In the `init_db()` function, add the index creation after the table creation statements:

```rust
conn.execute_batch("
    CREATE INDEX IF NOT EXISTS idx_history_created
    ON clipboard_history(created_at DESC);
")?;
```

The `IF NOT EXISTS` means this is safe to run every time the app starts - it only creates the index if it does not exist yet.

Also consider adding one for the pinned table's sort_order since reorder queries use it:
```rust
conn.execute_batch("
    CREATE INDEX IF NOT EXISTS idx_pinned_sort
    ON clipboard_pinned(sort_order ASC);
")?;
```

## How to Verify

```bash
cd src-tauri && cargo build 2>&1
```

Delete the existing database file (found at `~/Library/Application Support/com.adriano.clipboard-manager/clipx.db` on macOS) to force a fresh init, then run the app. Open the database with a SQLite browser and confirm the indexes exist:

```sql
SELECT name FROM sqlite_master WHERE type='index';
```

Should show `idx_history_created` and `idx_pinned_sort`.
