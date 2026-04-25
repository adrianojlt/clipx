# Task 17 - Add Rust Unit Tests

**Severity:** High
**Category:** Tooling / Testing
**Depends on:** Task 04 (proper error types) and Task 05 (module split) - both already done. The settings module exists, so tests go straight there.

## Why This Is a Problem

There are zero Rust tests. The settings parser, shortcut normalizer, and database path logic all have logic that can fail in subtle ways across platforms. Rust has a built-in test runner - adding tests costs nothing in setup.

The safest first targets are pure functions with no Tauri or database dependencies.

## Files to Touch

- `src-tauri/src/settings.rs` (test module for `normalize_shortcut`, `load_window_size`, settings round-trip)
- `src-tauri/src/window.rs` (test module for `clamp_to_monitor`)
- `src-tauri/Cargo.toml` (add `tempfile` dev-dep)

## Test Targets (Start Here)

### normalize_shortcut (in `settings.rs`)

After the recent refactor this function is token-based: it splits on `+`, uppercases each token, and maps aliases (`Option` -> `ALT`, `Cmd`/`Command`/`Meta` -> `SUPER`, `Control` -> `CTRL`). The interesting properties to lock in are: per-token mapping, alias handling for the modifier family, and untouched non-alias tokens.

### load_window_size (in `settings.rs`)

Pure function added in the recent refactor. Reads `window_width` / `window_height` from a settings `HashMap`, falls back to defaults, and clamps to `[300, 800]` x `[400, 900]`. Trivial to test for: defaults, clamp upper bound, clamp lower bound, garbage strings falling back to defaults.

### clamp_to_monitor (in `window.rs`)

Also added in the recent refactor. The `monitor: None` branch is a pure pass-through and is the only branch that can be unit-tested without a Tauri `Monitor` instance. Test that with `monitor = None` the input `(x, y)` is returned unchanged.

### load_settings / save_settings round-trip

These parse and write JSON via serde. A pure serde round-trip test (without calling our internal functions, which read/write a fixed system path) confirms the JSON shape is stable.

## Step 1 - Add a Test Module to settings.rs

At the bottom of `src-tauri/src/settings.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn normalize_shortcut_handles_option() {
        assert_eq!(normalize_shortcut("option+space"), "ALT+SPACE");
    }

    #[test]
    fn normalize_shortcut_maps_command_variants() {
        assert_eq!(normalize_shortcut("Cmd+K"), "SUPER+K");
        assert_eq!(normalize_shortcut("Command+K"), "SUPER+K");
        assert_eq!(normalize_shortcut("Meta+K"), "SUPER+K");
    }

    #[test]
    fn normalize_shortcut_passes_through_unknown_tokens() {
        // Tokens that aren't aliases are kept (uppercased).
        assert_eq!(normalize_shortcut("Shift+F1"), "SHIFT+F1");
    }

    #[test]
    fn normalize_shortcut_empty_returns_empty() {
        assert_eq!(normalize_shortcut(""), "");
    }

    #[test]
    fn load_window_size_defaults() {
        let s = HashMap::new();
        assert_eq!(load_window_size(&s), (400.0, 600.0));
    }

    #[test]
    fn load_window_size_clamps_upper() {
        let mut s = HashMap::new();
        s.insert("window_width".to_string(), "9999".to_string());
        s.insert("window_height".to_string(), "9999".to_string());
        assert_eq!(load_window_size(&s), (800.0, 900.0));
    }

    #[test]
    fn load_window_size_clamps_lower() {
        let mut s = HashMap::new();
        s.insert("window_width".to_string(), "10".to_string());
        s.insert("window_height".to_string(), "10".to_string());
        assert_eq!(load_window_size(&s), (300.0, 400.0));
    }

    #[test]
    fn load_window_size_garbage_falls_back_to_defaults() {
        let mut s = HashMap::new();
        s.insert("window_width".to_string(), "not-a-number".to_string());
        assert_eq!(load_window_size(&s), (400.0, 600.0));
    }

    #[test]
    fn settings_round_trip() {
        use std::fs;
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");

        let mut original = HashMap::new();
        original.insert("hotkey".to_string(), "ALT+SPACE".to_string());
        original.insert("history_limit".to_string(), "15".to_string());

        let json = serde_json::to_string(&original).unwrap();
        fs::write(&path, &json).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let loaded: HashMap<String, String> = serde_json::from_str(&content).unwrap();

        assert_eq!(loaded.get("hotkey").unwrap(), "ALT+SPACE");
        assert_eq!(loaded.get("history_limit").unwrap(), "15");
    }
}
```

## Step 2 - Add a Test Module to window.rs

At the bottom of `src-tauri/src/window.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_to_monitor_without_monitor_is_passthrough() {
        // When we don't know which monitor the cursor is on, we just return the
        // raw cursor coordinates. The Some(monitor) branch needs a Tauri Monitor
        // and is covered manually with the multi-monitor smoke test.
        assert_eq!(clamp_to_monitor(123, 456, 400.0, 600.0, None), (123, 456));
    }
}
```

## Step 3 - tempfile dev dependency

`settings_round_trip` uses `tempfile`. Add to `src-tauri/Cargo.toml`:
```toml
[dev-dependencies]
tempfile = "3"
```

## How to Verify

```bash
cd src-tauri && cargo test 2>&1
```

All tests must pass. Output will show one suite per module:
```
running 9 tests
test settings::tests::normalize_shortcut_handles_option ... ok
test settings::tests::normalize_shortcut_maps_command_variants ... ok
test settings::tests::normalize_shortcut_passes_through_unknown_tokens ... ok
test settings::tests::normalize_shortcut_empty_returns_empty ... ok
test settings::tests::load_window_size_defaults ... ok
test settings::tests::load_window_size_clamps_upper ... ok
test settings::tests::load_window_size_clamps_lower ... ok
test settings::tests::load_window_size_garbage_falls_back_to_defaults ... ok
test settings::tests::settings_round_trip ... ok

running 1 test
test window::tests::clamp_to_monitor_without_monitor_is_passthrough ... ok
```
