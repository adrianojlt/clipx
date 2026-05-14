<p align="center">
  <img src="left.png" alt="ClipX screenshot" width="45%" />
  <img src="right.png" alt="ClipX screenshot" width="45%" />
</p>

A lightweight clipboard manager for desktop. It keeps track of text you copy so you can recall it later without redoing the work.

## What it does

- **Background presence** - Lives in your system tray / menu bar while you work.
- **Global hotkey** - Summon the history popup instantly with a keyboard shortcut (near your mouse cursor).
- **Quick dismiss** - Press Escape to hide the popup. When the search box is focused, the first Escape clears focus and a second Escape hides the popup.
- **Tray menu** - Open the app, change settings, or quit from the tray icon.
- **Configurable hotkey** - Record your own keyboard shortcut to summon the popup via Settings.
- **Pin items** - Pin frequently used entries so they stay at the top, reorder them, and give each one a custom label. Hide sensitive content with the eye toggle.
- **Search** - Filter both pinned items and clipboard history instantly with the search box. Press `Command+F` (configurable) to jump to the search box from anywhere in the popup.
- **Configurable history size** - Choose how many entries to keep (up to 50) in Settings.
- **Tab shortcuts** - Switch between Pinned and History tabs with configurable keyboard shortcuts (default Command+1 / Command+2).
- **Resizable window** - Adjust the popup width and height in Settings.
- **Local only** - All clipboard history stays on your machine.

## Download

Pre-built installers for macOS, Windows, and Linux are available on the [Releases](../../releases) page.

> **macOS note:** If you see "ClipX is damaged and can't be opened", run this in Terminal after installing:
>
> ```bash
> xattr -cr /Applications/ClipX.app
> ```
>
> This removes the quarantine flag macOS adds to downloaded apps that aren't signed with an Apple Developer certificate.

## Testing

Run frontend unit tests:

```bash
pnpm test
```

Run tests in watch mode:

```bash
pnpm test:watch
```

Run Rust backend tests:

```bash
cd src-tauri && cargo test
```

## Code quality

Lint JavaScript and JSX:

```bash
pnpm lint
```

Format code with Prettier:

```bash
pnpm format
```

## Troubleshooting

If something goes wrong, check the log file for details.

**Log file location:**

- macOS: `~/Library/Application Support/com.adriano.clipx/logs/clipx.log`
- Windows: `%APPDATA%/ClipX/logs/clipx.log`
- Linux: `~/.config/ClipX/logs/clipx.log`

**What gets logged:**

- App startup and shutdown
- Database errors
- Clipboard monitor failures
- Settings load/save errors
- Shortcut registration failures

**What is NOT logged:**

- Clipboard content itself (privacy protection)
- Normal day-to-day operations

## Creating a release

1. Bump the version in all three places (replace `0.1.9` with the new version):
   - `package.json` - `"version"` field
   - `src-tauri/tauri.conf.json` - `"version"` field
   - `src-tauri/Cargo.toml` - `version` field

2. Update `Cargo.lock` by running a build:

```bash
cd src-tauri && cargo build
```

3. Commit and tag:

```bash
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "bump version to vX.Y.Z"
git tag vX.Y.Z
git push origin vX.Y.Z
```

Pushing the tag triggers the GitHub Actions workflow, which builds installers for all platforms and publishes them as a new release.

> The artifact filenames (e.g. `ClipX_0.1.9_aarch64.dmg`) come from the version in `tauri.conf.json`, not the git tag - so all three files must be bumped before tagging.

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
