# Task 13 - Define Event Name Constants

**Severity:** Low
**Category:** React-Tauri Boundary
**Depends on:** Nothing - independent, very small change

## Why This Is a Problem

The event name `"clipboard-changed"` is a raw string in two places:

- Rust: `app.emit("clipboard-changed", ())` in `monitor.rs`
- React: `listen("clipboard-changed", ...)` in `App.jsx`

If you rename the event in Rust but forget to update the React listener, the app silently stops updating when clipboard content changes. There is no error message.

For the React side, defining constants prevents typos and makes the string easy to find.

Note: The Rust side cannot import from JavaScript. The fix on the Rust side is a Rust constant. They must match manually - there is no automatic way to share them without code generation tools.

## Files to Touch

- `src/constants/events.js` (create this new file)
- `src/App.jsx` (use the constant)
- `src-tauri/src/monitor.rs` (add a Rust constant)

## Changes

### Create src/constants/events.js

```js
export const EVENTS = {
    CLIPBOARD_CHANGED: "clipboard-changed",
};
```

### Update App.jsx

Add import:
```js
import { EVENTS } from "./constants/events";
```

Replace:
```js
// Before:
const unlisten = await listen("clipboard-changed", () => { ... });

// After:
const unlisten = await listen(EVENTS.CLIPBOARD_CHANGED, () => { ... });
```

### Update monitor.rs (Rust side)

Near the top of `monitor.rs` (just below the existing `use` statements), add:
```rust
const EVENT_CLIPBOARD_CHANGED: &str = "clipboard-changed";
```

Replace the string literal in `start_clipboard_monitor`:
```rust
// Before:
let _ = app.emit("clipboard-changed", ());

// After:
let _ = app.emit(EVENT_CLIPBOARD_CHANGED, ());
```

## How to Verify

```bash
cd src-tauri && cargo build 2>&1
pnpm dev
```

Copy something to the clipboard. The app's history list must update automatically. No visual changes expected.
