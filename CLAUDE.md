# ClipX - Clipboard Manager

## Overview
A desktop clipboard manager built with Tauri v2 and JavaScript.
Records text copied to clipboard and saves it for later use.
A global hotkey triggers a popup window near the mouse cursor showing clipboard history.

## Tech Stack
- Framework: Tauri v2
- Frontend: JavaScript (React + Vite)
- Storage: SQLite (via rusqlite crate, bundled)
- Package manager: pnpm (never use npm)

## Key Conventions
- Always use async/await, never .then()
- Tauri v2 API only, never use v1 APIs
- All clipboard data stored locally, never sent anywhere. The only network call in the app is the update check (read-only, to the GitHub Releases API); no clipboard content is ever transmitted.

## Error Handling

All commands and internal helpers use the structured `AppError` enum defined in `error.rs`. Never return `Result<T, String>` in new code, use `Result<T, AppError>`. Several variants implement `From` for common upstream error types, so `?` works without manual mapping; pick a matching variant for everything else. `AppError` implements `serde::Serialize`, so Tauri delivers readable error messages to the frontend.

## Database Architecture

`AppState` holds two `rusqlite::Connection` instances, each behind a `Mutex`:
- `db` - used by all Tauri commands via `lock_db()`
- `db_monitor` - dedicated to the clipboard monitor background thread, avoiding lock contention with commands

Both are opened once in `init_app_state()`. Never open a new `Connection` from a command; always go through the shared state.

## Global Shortcuts

Never treat `ShortcutState::Released` from `tauri-plugin-global-shortcut` as the moment the user let go of the keys. On macOS it fires about 100 ms after the press, when the popup takes focus, however long the chord is actually held. Detect a real release from a `keyup` in the webview instead, which the page receives once the window has focus.

## Platform Integration

Prefer native platform APIs over shelling out to `osascript` or `powershell` on any path the user waits on. The cost is the subprocess, not the work: an AppleScript that drives System Events spends about 180 ms launching the interpreter and opening the Apple Event connection, and only single-digit milliseconds on the action itself. Keep the script path as a fallback and log when it is taken, so a silent regression is visible.

Note the permission shift this implies on macOS: driving System Events borrows its Accessibility grant, while calling the Accessibility API directly requires the ClipX binary itself to be granted Accessibility, and a bundled build is a different identity than the dev binary.

## Backend Commands

A new Tauri command must be (1) defined in a file under `src-tauri/src/commands/`, (2) re-exported via `pub mod <name>;` in `src-tauri/src/commands/mod.rs` if the file is new, and (3) listed in the `invoke_handler!` macro inside `run()` in `src-tauri/src/lib.rs`. Missing step 3 silently breaks the command at the frontend boundary. The frontend wrapper for every command lives in a per-feature service module under `src/services/` (e.g. `clipboardService.js`, `updateService.js`); components must call those wrappers, never `invoke()` directly.
