# Task 15 - Set a Real Content Security Policy

**Severity:** Medium
**Category:** Security / Tauri Config
**Depends on:** Nothing - independent, tiny change

## Why This Is a Problem

`tauri.conf.json` currently has:
```json
"security": {
    "csp": null
}
```

`null` completely disables the Content Security Policy. CSP is a browser security feature that prevents injected scripts from executing. For a local desktop app this is lower risk than a web app, but disabling it entirely removes a defensive layer for free.

If the app ever loads remote content or an attacker can inject clipboard content that gets rendered as HTML (unlikely now but possible in the future), a missing CSP offers no protection.

## Files to Touch

- `src-tauri/tauri.conf.json`

## Exact Change

```json
"security": {
    "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: asset: https://asset.localhost"
}
```

**What each part means:**
- `default-src 'self'` - by default only load resources from the app itself
- `script-src 'self'` - only execute scripts bundled in the app
- `style-src 'self' 'unsafe-inline'` - allow inline styles (Tauri and Vite use these)
- `img-src 'self' data: asset: https://asset.localhost` - allow the Tauri asset protocol for icons

## How to Verify

```bash
pnpm tauri dev
```

The app must open and function normally. If you see any console errors about CSP violations, you will need to add the blocked resource's origin to the policy. The browser DevTools (right-click -> Inspect in the Tauri window) will show CSP errors in the console.
