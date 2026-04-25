# Task 08 - Extract Utility Functions from App.jsx

**Severity:** Low
**Category:** React / Structure
**Depends on:** Nothing - independent, easy first step into React refactoring

## Why This Is a Problem

`parseShortcut()` and `matchesShortcut()` are pure JavaScript functions defined inside the `App` component function body. They:
- Get recreated on every render (wasted work)
- Cannot be unit tested without rendering the whole component
- Have no relationship to the component's state or props - they are plain utilities

Moving them out is zero-risk and makes the component smaller.

## Files to Touch

- `src/App.jsx` (remove the functions)
- `src/utils/shortcuts.js` (create this new file)

## Exact Changes

### Step 1 - Create src/utils/shortcuts.js

```js
export function parseShortcut(shortcut) {
    if (!shortcut) return null;
    const parts = shortcut.toLowerCase().split("+");
    return {
        meta: parts.includes("super") || parts.includes("meta"),
        ctrl: parts.includes("ctrl") || parts.includes("control"),
        alt: parts.includes("alt") || parts.includes("option"),
        shift: parts.includes("shift"),
        key: parts[parts.length - 1],
    };
}

export function matchesShortcut(event, shortcut) {
    const parsed = parseShortcut(shortcut);
    if (!parsed) return false;
    return (
        event.metaKey === parsed.meta &&
        event.ctrlKey === parsed.ctrl &&
        event.altKey === parsed.alt &&
        event.shiftKey === parsed.shift &&
        event.key.toLowerCase() === parsed.key
    );
}
```

Copy the exact logic from App.jsx - do not change behavior.

### Step 2 - Update App.jsx

Remove the two function definitions from inside the `App` component.

Add the import at the top of App.jsx:
```js
import { parseShortcut, matchesShortcut } from "./utils/shortcuts";
```

## How to Verify

```bash
pnpm dev
```

Open the app. Tab switching with keyboard shortcuts must work exactly as before. No visual changes expected.
