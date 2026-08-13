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

Windows shortcuts a shell already owns cannot be taken: the plugin calls `RegisterHotKey`, and Alt+Tab fails there with `ERROR_HOTKEY_ALREADY_REGISTERED` (1409). Alt+Esc is not held by anything and registers cleanly, which is what `cycle_windows_hotkey` binds by default; once ClipX holds it, Windows no longer cycles windows with it. Check a candidate with `RegisterHotKey` before designing around it rather than assuming the system reserves everything.

Holding a registered chord makes Windows auto-repeat `WM_HOTKEY` about thirty times a second. An action that steps through something, rather than toggling a window, has to debounce or one held key runs the whole sequence: see `CYCLE_DEBOUNCE` in `commands/apps/mod.rs`.

## Cycling an App's Windows

`cycle_windows` in `commands/apps/windows.rs` rotates the focused app's own windows. Two things it depends on, both easy to break:

`EnumWindows` returns windows in Z-order, and the rotation is that order. It therefore cannot be built on `list_open_apps`, which sorts by app and then by frecency; `switchable_windows_in_z_order` shares the enumeration but not the sorting.

Raising the next window is not enough on its own. The outgoing window stays second in Z-order, so the next press raises it straight back and a third window is never reached. The outgoing window is also pushed to `HWND_BOTTOM`, which is what Alt+Esc does, and what makes the cycle actually reach every window.

Windows of one app are matched by the owning executable's path, never by pid: Chromium and Electron apps spread windows over several processes, and some apps start one process per window.

## Platform Integration

Prefer native platform APIs over shelling out to `osascript` or `powershell` on any path the user waits on. The cost is the subprocess, not the work. Measured on this project:

| Path | Subprocess cost | Native equivalent |
| --- | --- | --- |
| macOS, AppleScript driving System Events | ~180 ms | Accessibility API in-process, single-digit ms |
| Windows, `powershell.exe -NoProfile -Command` | ~700 ms bare, ~800-950 ms once `New-Object -ComObject WScript.Shell` is added | Win32 call in-process, ~0.01 ms |

Windows PowerShell 5.1 is by far the worse of the two: it pays CLR and engine startup on every invocation, so a per-action `powershell` call is never acceptable on an interactive path. Keep the script path as a fallback and log when it is taken, so a silent regression is visible.

The platform halves live in separate files under `commands/apps/`: `macos.rs` and `windows.rs` behind `#[cfg]`, `other.rs` for everything else, with the shared types, the frecency ordering and the Tauri commands in `mod.rs`. `windows_icons.rs` and `window_cycle.rs` hold the pure Windows logic that must stay unit-testable on a mac, so they compile under `test` on every host.

Note the permission shift this implies on macOS: driving System Events borrows its Accessibility grant, while calling the Accessibility API directly requires the ClipX binary itself to be granted Accessibility, and a bundled build is a different identity than the dev binary.

Windows has no permission shift, but `SetForegroundWindow` has a foreground lock: only the process owning the foreground window or the one that received the last input event may call it. Since the popup hides itself before raising the target, borrow the right by attaching to the foreground window's input queue with `AttachThreadInput`, which is what `WScript.Shell.AppActivate` does internally. See `raise_window` in `commands/apps/windows.rs`.

## Listing an App's Windows on macOS

`list_open_apps` in `commands/apps/macos.rs` reads `AXWindows` off each `NSRunningApplication` with a Regular activation policy, one row per titled window, and `focus_app` raises the chosen window with `AXRaise` before activating its process.

It used to ask System Events for each process's Window menu instead. That failed three ways, all worth remembering:

- Reading another app's menus through System Events requires the *calling* app to be allowed assistive access, not just System Events. Without it the read errors inside the script's own `try`, which swallowed it, and every app silently collapsed to a single titleless row - the exact symptom of the missing grant, with nothing in the log to say so. `list_open_apps` now logs when `AXIsProcessTrusted()` is false.
- `repeat with p in (every process whose background only is false)` re-resolves the list on each step, so an app quitting mid-loop failed the whole script with `-1719 Invalid index`. The listing returned an error and the popup kept stale rows.
- The subprocess cost about 1.8 s with a dozen apps open, against 3 ms for the in-process enumeration.

`AXRaise` is honored by Chromium and Electron windows (verified on Chrome and VS Code), so clicking the Window-menu entry is not needed for them. Two details it does need: a minimized window has to have `AXMinimized` cleared first, and window identity is the pid plus the position in `AXWindows`, because two windows of one app routinely share a title. Positions shift as windows are raised, so `focus_app` matches on the title first and uses the index only to break ties.

`AXTitle` is not the Window-menu string: Chrome appends the profile and tab group, editors append the folder. Anything matching one against the other has to try both containment directions.

## Backend Commands

A new Tauri command must be (1) defined in a file under `src-tauri/src/commands/`, (2) re-exported via `pub mod <name>;` in `src-tauri/src/commands/mod.rs` if the file is new, and (3) listed in the `invoke_handler!` macro inside `run()` in `src-tauri/src/lib.rs`. Missing step 3 silently breaks the command at the frontend boundary. The frontend wrapper for every command lives in a per-feature service module under `src/services/` (e.g. `clipboardService.js`, `updateService.js`); components must call those wrappers, never `invoke()` directly.

Service modules import `invoke` from `src/services/invoke.js`, never from `@tauri-apps/api/core`. Tauri creates the windows declared in `tauri.conf.json` before the `setup` hook finishes, so a command fired from a mount effect can beat `app.manage(AppState)` and be rejected at the command boundary with "state not managed". Every command taking `State<AppState>` shares that gap; the wrapper retries just that error, briefly and a bounded number of times.
