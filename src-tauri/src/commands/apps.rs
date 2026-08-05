use crate::error::AppError;

use std::collections::HashMap;

#[derive(serde::Serialize, Clone)]
pub struct OpenApp {
    pub(crate) name: String,
    pub(crate) id: String,
    // Group identity: the owning app, without the window title. The frontend
    // looks its icon up by this, so several windows share one payload entry.
    pub(crate) app: String,
}

/// The app list plus a side map of icons, keyed by app identity rather than
/// carried per row: a 20-window app would otherwise repeat the same base64 PNG
/// twenty times in the payload.
#[derive(serde::Serialize)]
pub struct OpenAppsResult {
    pub(crate) apps: Vec<OpenApp>,
    pub(crate) icons: HashMap<String, String>,
}

// async so Tauri runs these off the main thread; the platform helpers spawn a
// blocking subprocess and must not freeze the UI event loop.
#[tauri::command]
pub async fn list_open_apps() -> Result<OpenAppsResult, AppError> {
    platform::list_open_apps()
}

#[tauri::command]
pub async fn focus_app(id: String) -> Result<(), AppError> {
    platform::focus_app(&id)
}

#[cfg(target_os = "macos")]
mod platform {

    use super::{OpenApp, OpenAppsResult};
    use crate::error::AppError;

    use std::collections::{HashMap, HashSet};
    use std::process::Command;
    use std::sync::{Mutex, OnceLock};

    // Separator packed into `id` to carry both process name and window title.
    const SEP: char = '\u{1f}';

    // Edge length of the extracted icon, in points.
    const ICON_SIZE: f64 = 32.0;

    // Extracted icons for the life of the process, keyed by executable path.
    // Icons rarely change while an app is installed, and an app relaunched under
    // a new pid keeps its path, so a restart is the accepted staleness window.
    static ICON_CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

    fn icon_cache() -> &'static Mutex<HashMap<String, String>> {
        ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
    }

    /// Read `key` from `cache`, falling back to `extract` on a miss and storing
    /// the result.
    ///
    /// A poisoned lock means an earlier extraction panicked mid-critical-section.
    /// Icons are decoration, so give up on them for the rest of the process
    /// rather than propagating the failure into the app list.
    fn cached_icon<F>(
        cache: &Mutex<HashMap<String, String>>,
        key: &str,
        extract: F,
    ) -> Option<String>
    where
        F: FnOnce() -> Option<String>,
    {
        match cache.lock() {
            Ok(map) => {
                if let Some(uri) = map.get(key) {
                    return Some(uri.clone());
                }
            }
            Err(_) => return None,
        }

        let uri = extract()?;

        // Racing callers may both extract the same key; last writer wins and the
        // values are identical, so no coordination is needed.
        if let Ok(mut map) = cache.lock() {
            map.insert(key.to_string(), uri.clone());
        }

        Some(uri)
    }

    /// Icons of every running foreground app, as `data:image/png;base64,...` URIs.
    ///
    /// Keyed twice per app: by `localizedName()` and by the executable stem.
    /// `list_open_apps` reports the AppleScript process name, which is the
    /// executable stem for unbundled binaries and the localized name for
    /// bundled ones, so callers can look up either without knowing which.
    ///
    /// Safe off the main thread: `runningApplications` is documented thread
    /// safe, and the drawing below targets an offscreen `NSBitmapImageRep`
    /// rather than the window server.
    fn icons() -> HashMap<String, String> {
        icons_from(icon_cache())
    }

    // Split from `icons()` so tests can supply their own cache instead of
    // mutating the process-wide one.
    fn icons_from(cache: &Mutex<HashMap<String, String>>) -> HashMap<String, String> {

        use objc2_app_kit::{NSApplicationActivationPolicy, NSWorkspace};

        let mut out: HashMap<String, String> = HashMap::new();

        for app in NSWorkspace::sharedWorkspace().runningApplications().iter() {

            if app.activationPolicy() != NSApplicationActivationPolicy::Regular {
                continue;
            }

            // The executable path is both the cache key and the source of the
            // stem key; an app without one gets no icon at all.
            let Some(path) = app
                .executableURL()
                .and_then(|url| url.path())
                .map(|path| path.to_string())
            else {
                continue;
            };

            let Some(uri) = cached_icon(cache, &path, || {
                let icon = app.icon()?;
                png_data_uri(&icon)
            }) else {
                continue;
            };

            if let Some(name) = app.localizedName() {
                out.insert(name.to_string(), uri.clone());
            }

            if let Some(stem) = std::path::Path::new(&path).file_stem() {
                out.insert(stem.to_string_lossy().into_owned(), uri);
            }
        }

        out
    }

    /// Draw `icon` into an offscreen 32x32 bitmap and encode it as a PNG data URI.
    fn png_data_uri(icon: &objc2_app_kit::NSImage) -> Option<String> {

        use base64::Engine;
        use objc2::AllocAnyThread;
        use objc2_app_kit::{
            NSBitmapImageFileType, NSBitmapImageRep, NSCompositingOperation, NSDeviceRGBColorSpace,
            NSGraphicsContext,
        };
        use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize};

        let size = NSSize::new(ICON_SIZE, ICON_SIZE);

        // Null planes + zero bytesPerRow/bitsPerPixel let AppKit allocate and
        // lay out the backing buffer itself.
        let rep = unsafe {
            NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
                NSBitmapImageRep::alloc(),
                std::ptr::null_mut(),
                ICON_SIZE as isize,
                ICON_SIZE as isize,
                8,
                4,
                true,
                false,
                NSDeviceRGBColorSpace,
                0,
                0,
            )
        }?;

        rep.setSize(size);

        let ctx = NSGraphicsContext::graphicsContextWithBitmapImageRep(&rep)?;

        let previous = NSGraphicsContext::currentContext();
        NSGraphicsContext::setCurrentContext(Some(&ctx));

        // Empty fromRect means "the whole source image".
        icon.drawInRect_fromRect_operation_fraction(
            NSRect::new(NSPoint::new(0.0, 0.0), size),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0)),
            NSCompositingOperation::SourceOver,
            1.0,
        );

        ctx.flushGraphics();
        NSGraphicsContext::setCurrentContext(previous.as_deref());

        let png = unsafe {
            rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
        }?;

        Some(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png.to_vec())
        ))
    }

    // List windows via each app's Window menu (Accessibility). An app's open
    // windows are the last N items of its Window menu (N = AX window count),
    // which are exactly the strings focus_app clicks - so list and focus always
    // agree, including for Chromium/Electron apps. Needs Accessibility only.
    // Output lines: "<process name>\t<window title>" (title empty if none).
    pub fn list_open_apps() -> Result<OpenAppsResult, AppError> {

        let script = "set output to \"\"\n\
            tell application \"System Events\"\n\
            repeat with p in (every process whose background only is false)\n\
            set pname to name of p\n\
            set emitted to false\n\
            try\n\
            set allItems to name of every menu item of menu 1 of (menu bar item \"Window\" of menu bar 1 of p)\n\
            set total to count of allItems\n\
            set sepIndex to 0\n\
            repeat with i from 1 to total\n\
            if item i of allItems is missing value then set sepIndex to i\n\
            end repeat\n\
            if sepIndex > 0 and sepIndex < total then\n\
            repeat with i from (sepIndex + 1) to total\n\
            set t to item i of allItems\n\
            if t is not missing value then\n\
            set output to output & pname & tab & (t as text) & linefeed\n\
            set emitted to true\n\
            end if\n\
            end repeat\n\
            end if\n\
            end try\n\
            if not emitted then\n\
            set output to output & pname & tab & \"\" & linefeed\n\
            end if\n\
            end repeat\n\
            end tell\n\
            return output";

        let out = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| AppError::State(format!("osascript failed: {e}")))?;

        if !out.status.success() {
            return Err(AppError::State(format!(
                "osascript error: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }

        let stdout = String::from_utf8_lossy(&out.stdout);

        let mut seen: HashSet<String> = HashSet::new();
        let mut apps: Vec<OpenApp> = Vec::new();

        for line in stdout.lines() {

            let (app, title) = line.split_once('\t').unwrap_or((line, ""));
            let app = app.trim();
            let title = title.trim();

            if app.is_empty() {
                continue;
            }

            let id = format!("{app}{SEP}{title}");
            if !seen.insert(id.clone()) {
                continue;
            }

            let name = if title.is_empty() {
                app.to_string()
            } else {
                format!("{app} - {title}")
            };

            apps.push(OpenApp {
                name,
                id,
                app: app.to_string(),
            });
        }

        apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        Ok(OpenAppsResult {
            apps,
            icons: icons(),
        })
    }

    // id == "<process name>\u{1f}<window title>". Bring the process to the
    // front, then raise the specific window if a title is present.
    pub fn focus_app(id: &str) -> Result<(), AppError> {

        let (app, title) = id.split_once(SEP).unwrap_or((id, ""));

        if app.chars().any(|c| matches!(c, '"' | '\\' | '\0' | '\r' | '\n' | '\t'))
            || title.chars().any(|c| matches!(c, '"' | '\\' | '\0' | '\r' | '\n' | '\t'))
        {
            return Err(AppError::Validation("invalid app id".into()));
        }

        // Raise the window via the app's own Window menu item. AXRaise/AXMain
        // are ignored by Chromium/Electron apps (Chrome, VS Code) for real
        // focus, but clicking the Window-menu entry is honored everywhere.
        // Exact title match first, then substring (Chrome decorates titles).
        // Select the target window via the Window menu BEFORE activating the
        // app, so the app comes forward already showing the right window (no
        // flash of the previously-active window).
        let inner = if title.is_empty() {
            "set frontmost to true".to_string()
        } else {
            format!(
                "try\n\
                click (first menu item of menu 1 of menu bar item \"Window\" of menu bar 1 whose name is \"{title}\")\n\
                on error\n\
                try\n\
                click (first menu item of menu 1 of menu bar item \"Window\" of menu bar 1 whose name contains \"{title}\")\n\
                end try\n\
                end try\n\
                set frontmost to true"
            )
        };

        let script = format!(
            "tell application \"System Events\"\n\
            tell process \"{app}\"\n\
            {inner}\n\
            end tell\n\
            end tell"
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| AppError::State(format!("osascript failed: {e}")))?;

        if !output.status.success() {
            return Err(AppError::State(format!(
                "failed to focus app: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    #[cfg(test)]
    mod tests {

        use super::{cached_icon, icons};
        use base64::Engine;
        use std::collections::HashMap;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, Mutex};

        const PNG_MAGIC: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

        #[test]
        fn cache_miss_extracts_and_populates() {

            let cache = Mutex::new(HashMap::new());
            let calls = AtomicUsize::new(0);

            let uri = cached_icon(&cache, "/Applications/Foo.app/Contents/MacOS/Foo", || {
                calls.fetch_add(1, Ordering::SeqCst);
                Some("data:image/png;base64,AAA".to_string())
            });

            assert_eq!(uri.as_deref(), Some("data:image/png;base64,AAA"));
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert_eq!(cache.lock().unwrap().len(), 1);
        }

        #[test]
        fn cache_hit_does_not_re_extract() {

            let cache = Mutex::new(HashMap::new());
            let calls = AtomicUsize::new(0);

            let extract = || {
                calls.fetch_add(1, Ordering::SeqCst);
                Some("data:image/png;base64,AAA".to_string())
            };

            let first = cached_icon(&cache, "/bin/foo", extract);
            let second = cached_icon(&cache, "/bin/foo", || {
                calls.fetch_add(1, Ordering::SeqCst);
                Some("data:image/png;base64,DIFFERENT".to_string())
            });

            assert_eq!(first, second, "second call should serve the cached value");
            assert_eq!(calls.load(Ordering::SeqCst), 1, "extractor ran twice");
        }

        #[test]
        fn failed_extraction_is_not_cached() {

            let cache = Mutex::new(HashMap::new());

            assert_eq!(cached_icon(&cache, "/bin/foo", || None), None);
            assert!(cache.lock().unwrap().is_empty());
        }

        #[test]
        fn poisoned_lock_degrades_to_no_icon() {

            let cache: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

            // Poison by panicking while holding the guard, as a panicking
            // extraction inside `cached_icon` would.
            let held = Arc::clone(&cache);
            let hook = std::panic::take_hook();

            std::panic::set_hook(Box::new(|_| {}));
            let _ = std::thread::spawn(move || {
                let _guard = held.lock().unwrap();
                panic!("poison the cache");
            }).join();

            std::panic::set_hook(hook);

            assert!(cache.is_poisoned());

            let calls = AtomicUsize::new(0);
            let uri = cached_icon(&cache, "/bin/foo", || {
                calls.fetch_add(1, Ordering::SeqCst);
                Some("data:image/png;base64,AAA".to_string())
            });

            assert_eq!(uri, None, "poisoned cache must yield no icon");
            assert_eq!(calls.load(Ordering::SeqCst), 0, "should not extract");
        }

        // Acceptance evidence for the new return type. Skips when the test binary
        // lacks Accessibility permission, which no CI runner grants.
        #[test]
        fn list_open_apps_populates_app_field_and_icons() {

            let result = match super::list_open_apps() {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("skipped: list_open_apps failed ({e:?})");
                    return;
                }
            };

            eprintln!(
                "{} rows, {} icon keys",
                result.apps.len(),
                result.icons.len()
            );

            for row in &result.apps {
                assert!(!row.app.is_empty(), "empty app field for {:?}", row.name);
                assert!(
                    row.name == row.app || row.name.starts_with(&format!("{} - ", row.app)),
                    "name {:?} does not lead with app {:?}",
                    row.name,
                    row.app
                );
                eprintln!(
                    "  app={:?} icon={}",
                    row.app,
                    result.icons.contains_key(&row.app)
                );
            }

            assert!(!result.icons.is_empty(), "no icons extracted");
        }

        // Runs on a non-main thread to prove icon extraction does not need one.
        #[test]
        fn extracted_icons_are_decodable_pngs() {

            let map = std::thread::spawn(icons).join().expect("icons() panicked");

            eprintln!("icons(): {} keys", map.len());

            for (name, uri) in &map {

                let b64 = uri
                    .strip_prefix("data:image/png;base64,")
                    .unwrap_or_else(|| panic!("{name}: bad data URI prefix: {uri:.40}"));

                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .unwrap_or_else(|e| panic!("{name}: base64 decode failed: {e}"));

                assert!(
                    bytes.starts_with(PNG_MAGIC),
                    "{name}: decoded bytes are not a PNG"
                );

                // IHDR is the first chunk: 8 magic + 4 length + 4 type, then
                // big-endian width and height.
                let dim = |o: usize| u32::from_be_bytes(bytes[o..o + 4].try_into().unwrap());
                let (w, h) = (dim(16), dim(20));

                assert_eq!((w, h), (32, 32), "{name}: unexpected PNG size {w}x{h}");

                eprintln!("  {name}: {} bytes PNG, {w}x{h}", bytes.len());
            }
        }
    }
}

// PowerShell output parsing, script building, and icon caching for Windows.
//
// Kept outside `platform`, which cannot compile on a non-Windows host because of
// its `std::os::windows` import, so these pure functions stay unit-testable from
// any development machine. Compiled under `test` everywhere for that reason.
#[cfg(any(target_os = "windows", test))]
mod windows_icons {

    use super::OpenApp;

    use std::collections::HashMap;
    use std::sync::Mutex;

    // Extracted icons for the life of the process, keyed by executable path. The
    // macOS module keeps its own; the two platforms fill the cache by different
    // shapes (one call per app vs. one batched call for all misses).
    #[cfg(target_os = "windows")]
    static ICON_CACHE: std::sync::OnceLock<Mutex<HashMap<String, String>>> =
        std::sync::OnceLock::new();

    #[cfg(target_os = "windows")]
    pub fn icon_cache() -> &'static Mutex<HashMap<String, String>> {
        ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
    }

    /// One listed window: the row the frontend renders, plus the owning
    /// executable path used as the icon cache key (empty when unreadable).
    pub struct Row {
        pub app: OpenApp,
        pub path: String,
    }

    /// Parse `<pid>\t<process name>\t<path>\t<window title>` lines, sorted by
    /// `(app, title)`.
    ///
    /// The title comes last because it is the only field that can itself contain
    /// a tab; a Windows path cannot, and a pid and process name will not.
    pub fn parse_rows(stdout: &str) -> Vec<Row> {

        let mut rows: Vec<Row> = stdout
            .lines()
            .filter_map(|line| {
                let mut fields = line.splitn(4, '\t');

                let pid = fields.next()?.trim();
                let process = fields.next()?.trim();
                let path = fields.next()?.trim();
                let title = fields.next()?.trim();

                if title.is_empty() || pid.is_empty() {
                    return None;
                }

                Some(Row {
                    app: OpenApp {
                        name: title.to_string(),
                        id: pid.to_string(),
                        app: process.to_string(),
                    },
                    path: path.to_string(),
                })
            })
            .collect();

        // Keep an app's windows together, matching macOS, where `name` already
        // starts with the app so a name sort groups for free. Ties fall back to
        // the title so the order is stable.
        rows.sort_by(|a, b| {
            let key = |r: &Row| (r.app.app.to_lowercase(), r.app.name.to_lowercase());
            key(a).cmp(&key(b))
        });

        rows
    }

    /// Split `paths` into already-cached icons and the paths still needing
    /// extraction. A poisoned lock reports everything as a hit-less no-op so the
    /// list still renders, without paying for extraction that can never be stored.
    pub fn split_cached(
        cache: &Mutex<HashMap<String, String>>,
        paths: &[String],
    ) -> (HashMap<String, String>, Vec<String>) {

        let Ok(map) = cache.lock() else {
            return (HashMap::new(), Vec::new());
        };

        let mut hits = HashMap::new();
        let mut misses = Vec::new();

        for path in paths {
            match map.get(path) {
                Some(uri) => {
                    hits.insert(path.clone(), uri.clone());
                }
                // Several windows of one app share a path, so guard against
                // asking PowerShell for the same icon twice in one call.
                None if !misses.contains(path) => misses.push(path.clone()),
                None => {}
            }
        }

        (hits, misses)
    }

    pub fn store(cache: &Mutex<HashMap<String, String>>, entries: &[(String, String)]) {
        if let Ok(mut map) = cache.lock() {
            for (path, uri) in entries {
                map.insert(path.clone(), uri.clone());
            }
        }
    }

    /// PowerShell 5.1 script emitting `<path>\t<base64 png>` for each path whose
    /// icon can be read. `ExtractAssociatedIcon` yields the 32x32 shell icon,
    /// matching the size the macOS path draws at.
    pub fn extraction_script(paths: &[String]) -> String {

        // Paths are embedded in single-quoted PowerShell literals, where the only
        // escape is a doubled quote and no interpolation happens. Control
        // characters cannot appear in a Windows path and would break the
        // line-oriented output, so drop those rows instead of quoting them.
        let list = paths
            .iter()
            .filter(|p| !p.is_empty() && !p.chars().any(char::is_control))
            .map(|p| format!("'{}'", p.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(",");

        format!(
            "Add-Type -AssemblyName System.Drawing\n\
            foreach ($p in @({list})) {{\n\
            try {{\n\
            $i = [System.Drawing.Icon]::ExtractAssociatedIcon($p)\n\
            if ($i) {{\n\
            $b = $i.ToBitmap()\n\
            $m = New-Object System.IO.MemoryStream\n\
            $b.Save($m, [System.Drawing.Imaging.ImageFormat]::Png)\n\
            \"$p`t$([Convert]::ToBase64String($m.ToArray()))\"\n\
            $m.Dispose()\n\
            $b.Dispose()\n\
            $i.Dispose()\n\
            }}\n\
            }} catch {{ }}\n\
            }}"
        )
    }

    /// Parse the extraction script's `<path>\t<base64 png>` lines into data URIs.
    pub fn parse_icon_pairs(stdout: &str) -> Vec<(String, String)> {
        stdout
            .lines()
            .filter_map(|line| {
                let (path, b64) = line.split_once('\t')?;
                let (path, b64) = (path.trim(), b64.trim());

                if path.is_empty() || b64.is_empty() {
                    return None;
                }

                Some((
                    path.to_string(),
                    format!("data:image/png;base64,{b64}"),
                ))
            })
            .collect()
    }

    /// Re-key path-indexed icons by app identity, which is what the frontend
    /// looks rows up by.
    pub fn icons_by_app(
        rows: &[Row],
        by_path: &HashMap<String, String>,
    ) -> HashMap<String, String> {

        let mut out = HashMap::new();

        for row in rows {
            if let Some(uri) = by_path.get(&row.path) {
                out.insert(row.app.app.clone(), uri.clone());
            }
        }

        out
    }

    #[cfg(test)]
    mod tests {

        use super::*;
        use std::sync::Arc;

        fn line(pid: &str, process: &str, path: &str, title: &str) -> String {
            format!("{pid}\t{process}\t{path}\t{title}")
        }

        #[test]
        fn parses_rows_and_sorts_by_app_then_title() {

            let stdout = [
                line("300", "explorer", r"C:\Windows\explorer.exe", "Downloads"),
                line("100", "chrome", r"C:\chrome.exe", "Inbox"),
                line("101", "chrome", r"C:\chrome.exe", "Docs"),
            ]
            .join("\n");

            let rows = parse_rows(&stdout);

            assert_eq!(
                rows.iter()
                    .map(|r| (r.app.app.as_str(), r.app.name.as_str()))
                    .collect::<Vec<_>>(),
                vec![
                    ("chrome", "Docs"),
                    ("chrome", "Inbox"),
                    ("explorer", "Downloads"),
                ],
                "same-app rows must end up adjacent"
            );

            assert_eq!(rows[0].app.id, "101", "id stays the pid");
            assert_eq!(rows[0].path, r"C:\chrome.exe");
        }

        #[test]
        fn keeps_rows_whose_path_is_unreadable() {

            // Access Denied on an elevated process: empty path, row still listed.
            let rows = parse_rows(&line("42", "elevated", "", "Admin Console"));

            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].app.app, "elevated");
            assert_eq!(rows[0].app.name, "Admin Console");
            assert!(rows[0].path.is_empty());
        }

        #[test]
        fn keeps_a_tab_inside_the_window_title() {

            // The title is the last field precisely so its tabs survive.
            let rows = parse_rows(&line("7", "editor", r"C:\editor.exe", "draft\tnotes.txt"));

            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].app.name, "draft\tnotes.txt");
            assert_eq!(rows[0].path, r"C:\editor.exe");
        }

        #[test]
        fn drops_short_and_titleless_rows() {

            let stdout = [
                "9\tchrome\tC:\\chrome.exe".to_string(),
                line("10", "chrome", r"C:\chrome.exe", "   "),
                String::new(),
                line("11", "chrome", r"C:\chrome.exe", "Real"),
            ]
            .join("\n");

            let rows = parse_rows(&stdout);

            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].app.name, "Real");
        }

        #[test]
        fn splits_hits_from_misses_and_deduplicates() {

            let cache = Mutex::new(HashMap::from([(
                r"C:\chrome.exe".to_string(),
                "data:image/png;base64,AAA".to_string(),
            )]));

            let paths = vec![
                r"C:\chrome.exe".to_string(),
                r"C:\code.exe".to_string(),
                // Second window of the same uncached app.
                r"C:\code.exe".to_string(),
            ];

            let (hits, misses) = split_cached(&cache, &paths);

            assert_eq!(hits.len(), 1);
            assert_eq!(misses, vec![r"C:\code.exe".to_string()], "asked twice");
        }

        #[test]
        fn warm_cache_reports_no_misses() {

            let cache = Mutex::new(HashMap::new());
            store(
                &cache,
                &[(r"C:\a.exe".to_string(), "data:image/png;base64,AAA".to_string())],
            );

            let (hits, misses) = split_cached(&cache, &[r"C:\a.exe".to_string()]);

            assert_eq!(hits.len(), 1);
            assert!(misses.is_empty(), "a warm cache must skip extraction");
        }

        #[test]
        fn poisoned_cache_degrades_to_no_icons() {

            let cache: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

            let held = Arc::clone(&cache);
            let hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(|_| {}));
            let _ = std::thread::spawn(move || {
                let _guard = held.lock().unwrap();
                panic!("poison the cache");
            })
            .join();
            std::panic::set_hook(hook);

            let (hits, misses) = split_cached(&cache, &[r"C:\a.exe".to_string()]);

            assert!(hits.is_empty());
            assert!(misses.is_empty(), "must not extract what cannot be stored");

            // Storing into a poisoned cache is a no-op, not a panic.
            store(&cache, &[(r"C:\a.exe".to_string(), "x".to_string())]);
        }

        #[test]
        fn extraction_script_quotes_paths_safely() {

            let script = extraction_script(&[
                r"C:\Program Files\app.exe".to_string(),
                r"C:\Bob's Tools\t.exe".to_string(),
                // Dropped: control characters would break the line-based output.
                "C:\\bad\npath.exe".to_string(),
                String::new(),
            ]);

            assert!(script.contains(r"'C:\Program Files\app.exe'"));
            assert!(
                script.contains(r"'C:\Bob''s Tools\t.exe'"),
                "single quote must be doubled, got: {script}"
            );
            assert!(!script.contains("bad\npath"));
            assert!(!script.contains("@(,"), "empty path left a hole in the array");
            assert!(script.contains("ExtractAssociatedIcon"));
        }

        #[test]
        fn parses_icon_pairs_into_data_uris() {

            let stdout = "C:\\a.exe\tQUJD\n\nC:\\b.exe\t\nmalformed\nC:\\c.exe\tWFla";

            assert_eq!(
                parse_icon_pairs(stdout),
                vec![
                    (r"C:\a.exe".to_string(), "data:image/png;base64,QUJD".to_string()),
                    (r"C:\c.exe".to_string(), "data:image/png;base64,WFla".to_string()),
                ]
            );
        }

        #[test]
        fn rekeys_icons_by_process_name() {

            let rows = parse_rows(
                &[
                    line("1", "chrome", r"C:\chrome.exe", "Inbox"),
                    line("2", "chrome", r"C:\chrome.exe", "Docs"),
                    line("3", "elevated", "", "Admin"),
                ]
                .join("\n"),
            );

            let by_path = HashMap::from([(
                r"C:\chrome.exe".to_string(),
                "data:image/png;base64,AAA".to_string(),
            )]);

            let icons = icons_by_app(&rows, &by_path);

            assert_eq!(icons.len(), 1, "one entry per app, not per window");
            assert_eq!(icons.get("chrome").map(String::as_str), Some("data:image/png;base64,AAA"));
            assert!(!icons.contains_key("elevated"), "no path means no icon");
        }
    }
}

#[cfg(target_os = "windows")]
mod platform {

    use super::windows_icons::{
        extraction_script, icon_cache, icons_by_app, parse_icon_pairs, parse_rows, split_cached,
        store,
    };
    use super::{OpenApp, OpenAppsResult};

    use crate::error::AppError;

    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    // Must stay `powershell` (Windows PowerShell 5.1), never `pwsh`: 5.1 is the
    // one shipped with Windows and the only guaranteed source of the
    // System.Drawing assembly the icon extraction loads.
    fn powershell(script: &str) -> Result<String, AppError> {

        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| AppError::State(format!("powershell failed: {e}")))?;

        if !output.status.success() {
            return Err(AppError::State(format!(
                "powershell error: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    // List processes that own a visible main window. id = PID, name = window
    // title, app = process name. `$_.Path` throws for elevated processes, so each
    // row falls back to an empty path rather than failing the whole call.
    pub fn list_open_apps() -> Result<OpenAppsResult, AppError> {

        let script = "Get-Process | Where-Object { $_.MainWindowTitle -ne '' } | \
            ForEach-Object { \
            $p = ''; \
            try { $p = $_.Path } catch { }; \
            if ($null -eq $p) { $p = '' }; \
            \"$($_.Id)`t$($_.ProcessName)`t$p`t$($_.MainWindowTitle)\" }";

        let rows = parse_rows(&powershell(script)?);

        let paths: Vec<String> = rows
            .iter()
            .map(|r| r.path.clone())
            .filter(|p| !p.is_empty())
            .collect();

        let (mut by_path, misses) = split_cached(icon_cache(), &paths);

        // Steady state is one subprocess call: with every path cached there is
        // nothing to extract and the second invocation is skipped entirely.
        if !misses.is_empty() {
            // Icons are decoration, so a failed extraction leaves blank slots
            // rather than failing the list.
            if let Ok(stdout) = powershell(&extraction_script(&misses)) {
                let pairs = parse_icon_pairs(&stdout);
                store(icon_cache(), &pairs);
                by_path.extend(pairs);
            }
        }

        let icons = icons_by_app(&rows, &by_path);
        let apps: Vec<OpenApp> = rows.into_iter().map(|r| r.app).collect();

        Ok(OpenAppsResult { apps, icons })
    }

    // id == PID on Windows. AppActivate accepts a PID and raises the window.
    pub fn focus_app(id: &str) -> Result<(), AppError> {

        let pid: u32 = id
            .parse()
            .map_err(|_| AppError::Validation(format!("invalid app id: {id}")))?;

        let script = format!(
            "(New-Object -ComObject WScript.Shell).AppActivate({pid})"
        );

        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| AppError::State(format!("powershell failed: {e}")))?;

        if !output.status.success() {
            return Err(AppError::State(format!(
                "failed to focus app: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod platform {

    use super::OpenAppsResult;
    use crate::error::AppError;

    pub fn list_open_apps() -> Result<OpenAppsResult, AppError> {
        Err(AppError::State("listing apps not supported on this platform".into()))
    }

    pub fn focus_app(_id: &str) -> Result<(), AppError> {
        Err(AppError::State("focusing apps not supported on this platform".into()))
    }
}

