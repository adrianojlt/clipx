# ClipX

A lightweight clipboard manager for desktop. It keeps track of text you copy so you can recall it later without redoing the work.

## What it does

- **Background presence** - Lives in your system tray / menu bar while you work.
- **Global hotkey** - Summon the history popup instantly with a keyboard shortcut (near your mouse cursor).
- **Quick dismiss** - Press Escape to hide the popup and get back to work.
- **Tray menu** - Open the app or quit from the tray icon.
- **Local only** - All clipboard history stays on your machine.

## Running in development

```bash
pnpm tauri dev
```

This starts the app with live reload for both the frontend and the Rust backend.

## Building for production

```bash
pnpm tauri build
```

The compiled app will be available under `src-tauri/target/release/`.

## Tech stack

- Tauri v2
- React + Vite
- SQLite (local storage)

---

Built for keeping things simple and local.
