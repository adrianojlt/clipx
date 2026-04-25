# Task 16 - Add Vitest and First React Unit Tests

**Severity:** High
**Category:** Tooling / Testing
**Depends on:** Task 08 (extract utility functions) - required, tests target those pure functions

## Why This Is a Problem

There are zero frontend tests. Refactoring App.jsx (Tasks 10, 11) or changing keyboard shortcut logic carries no safety net. A test that runs in 50ms will catch regressions before you even open the app.

The best place to start is the utility functions extracted in Task 08 - they are pure functions with no Tauri or browser dependencies, so they are trivial to test.

## Files to Touch

- `package.json` (add devDependencies and test script)
- `vite.config.js` (add test configuration)
- `src/utils/shortcuts.test.js` (create - first test file)

## Step 1 - Install Vitest

```bash
pnpm add -D vitest @vitest/ui
```

## Step 2 - Add test script to package.json

```json
"scripts": {
    "test": "vitest run",
    "test:watch": "vitest",
    "test:ui": "vitest --ui"
}
```

## Step 3 - Configure Vitest in vite.config.js

Add `test` to the existing config:

```js
export default defineConfig({
    plugins: [react()],
    // ... existing config ...
    test: {
        environment: "jsdom",
        globals: true,
    },
});
```

## Step 4 - Create src/utils/shortcuts.test.js

Note: the current `parseShortcut` in `src/utils/shortcuts.js` is **case-sensitive** (matches `"Alt"`, not `"alt"`), uppercases single-letter keys (`"H"`, not `"h"`), and remaps `"Space"` to a literal space (`" "`). Tests below match that behaviour — do not lowercase modifier names.

```js
import { describe, it, expect } from "vitest";
import { parseShortcut, matchesShortcut } from "./shortcuts";

describe("parseShortcut", () => {
    it("parses a simple shortcut", () => {
        const result = parseShortcut("Alt+H");
        expect(result.alt).toBe(true);
        expect(result.key).toBe("H");
        expect(result.meta).toBe(false);
        expect(result.ctrl).toBe(false);
        expect(result.shift).toBe(false);
    });

    it("parses a multi-modifier shortcut", () => {
        const result = parseShortcut("Ctrl+Shift+P");
        expect(result.ctrl).toBe(true);
        expect(result.shift).toBe(true);
        expect(result.key).toBe("P");
    });

    it("normalises the Space key to a literal space", () => {
        const result = parseShortcut("Alt+Space");
        expect(result.alt).toBe(true);
        expect(result.key).toBe(" ");
    });

    it("treats Option as alt", () => {
        const result = parseShortcut("Option+Space");
        expect(result.alt).toBe(true);
        expect(result.key).toBe(" ");
    });

    it("treats Super, Command, and Meta as meta", () => {
        expect(parseShortcut("Super+K").meta).toBe(true);
        expect(parseShortcut("Command+K").meta).toBe(true);
        expect(parseShortcut("Meta+K").meta).toBe(true);
    });
});

describe("matchesShortcut", () => {
    it("returns true when event matches shortcut", () => {
        const event = { metaKey: false, ctrlKey: false, altKey: true, shiftKey: false, key: "H" };
        expect(matchesShortcut(event, "Alt+H")).toBe(true);
    });

    it("returns false when modifier does not match", () => {
        const event = { metaKey: false, ctrlKey: true, altKey: false, shiftKey: false, key: "H" };
        expect(matchesShortcut(event, "Alt+H")).toBe(false);
    });

    it("matches a Space shortcut against a real space key", () => {
        const event = { metaKey: false, ctrlKey: false, altKey: true, shiftKey: false, key: " " };
        expect(matchesShortcut(event, "Alt+Space")).toBe(true);
    });
});
```

## How to Verify

```bash
pnpm test
```

All tests must pass. This confirms the extracted utility functions work correctly before you refactor the components that use them.
