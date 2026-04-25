# Task 02 - Remove Unused Dependencies

**Severity:** Low
**Category:** Tooling
**Depends on:** Nothing - safe to do anytime

## Why This Is a Problem

Two dependencies are declared but never used. They increase build times, bundle size, and confuse anyone reading the project for the first time.

- `tauri-plugin-clipboard-manager` is in `Cargo.toml` but all clipboard work in Rust is done via the `arboard` crate.
- `@tauri-apps/plugin-sql` is in `package.json` but all SQLite work is done in Rust via `rusqlite`. The frontend never touches SQL directly.

Note: `@tauri-apps/plugin-clipboard-manager` should STAY in `package.json` because `App.jsx` uses `writeText` from it to write to clipboard on copy.

## Files to Touch

- `src-tauri/Cargo.toml`
- `package.json`

## Exact Changes

### src-tauri/Cargo.toml

Remove this line:
```toml
tauri-plugin-clipboard-manager = "2.3.2"
```

### package.json

Remove this line from `dependencies`:
```json
"@tauri-apps/plugin-sql": "^2.3.2",
```

## How to Verify

```bash
cd src-tauri && cargo build 2>&1
pnpm install && pnpm build 2>&1
```

Both must succeed. Search the codebase for any import of `tauri-plugin-clipboard-manager` in Rust (should find none) and any import of `@tauri-apps/plugin-sql` in JS (should find none).

```bash
grep -r "plugin-sql" src/
grep -r "tauri-plugin-clipboard-manager" src-tauri/src/
```

Both should return empty.
