# Task 09 - Create an invoke() Service Layer

**Severity:** High
**Category:** React-Tauri Boundary
**Depends on:** Nothing - independent, but do this before extracting React components

## Why This Is a Problem

There are 13 different `invoke("command_name", { ... })` calls scattered directly inside component event handlers and useEffect hooks. Each command name is a raw string. Problems:

- A typo in a command name fails silently at runtime (no build-time error)
- If a Rust command is renamed, you must grep the entire frontend to find all call sites
- No single place to add consistent error handling or logging
- Impossible to mock backend calls for future testing

A service layer is a thin file of named functions that wrap each invoke. The component imports the function, never the command string.

## Files to Touch

- `src/services/clipboardService.js` (create this new file)
- `src/App.jsx` (replace all invoke calls)
- `src/components/Settings.jsx` (replace all invoke calls)

## Step 1 - Create src/services/clipboardService.js

```js
import { invoke } from "@tauri-apps/api/core";

export const getHistory = () =>
    invoke("get_history");

export const getPinned = () =>
    invoke("get_pinned");

export const getClipboard = () =>
    invoke("get_clipboard");

export const getSetting = (key) =>
    invoke("get_setting", { key });

export const setSetting = (key, value) =>
    invoke("set_setting", { key, value });

export const pinItem = (content) =>
    invoke("pin_item", { content });

export const deleteHistoryItem = (id) =>
    invoke("delete_history_item", { id });

export const unpinItem = (id) =>
    invoke("unpin_item", { id });

export const updatePinnedDescription = (id, description) =>
    invoke("update_pinned_description", { id, description });

export const togglePinnedHidden = (id) =>
    invoke("toggle_pinned_hidden", { id });

export const reorderPinned = (items) =>
    invoke("reorder_pinned", { items });

export const updateShortcut = (shortcut) =>
    invoke("update_shortcut", { shortcut });

export const applyWindowSize = () =>
    invoke("apply_window_size");
```

## Step 2 - Update App.jsx

Remove the `invoke` import:
```js
// Remove: import { invoke } from "@tauri-apps/api/core";
```

Add service imports:
```js
import {
    getHistory, getPinned, getClipboard, getSetting,
    pinItem, deleteHistoryItem, unpinItem,
    updatePinnedDescription, togglePinnedHidden, reorderPinned,
} from "./services/clipboardService";
```

Replace each `invoke("command_name", { ... })` call with the matching function. Example:
```js
// Before:
const data = await invoke("get_history");

// After:
const data = await getHistory();
```

## Step 3 - Update Settings.jsx

Remove `invoke` import. Add:
```js
import { getSetting, setSetting, updateShortcut, applyWindowSize } from "../services/clipboardService";
```

Replace each invoke call with the matching function.

## How to Verify

```bash
pnpm dev
```

Every feature must work: copy, pin, delete, reorder, edit description, toggle hidden, open settings, change hotkey, save settings. No change in behavior - this is purely structural.
