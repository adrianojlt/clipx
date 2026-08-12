use crate::commands::lock_db;
use crate::error::AppError;
use crate::AppState;

use std::collections::HashMap;
use tauri::State;

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

// Longest process name accepted as a usage key. This is the only path that ever
// creates a row, so it is where junk is kept out of the table.
const MAX_APP_LEN: usize = 256;

fn validate_app(app: &str) -> Result<(), AppError> {

    if app.is_empty() {
        return Err(AppError::Validation("app name is empty".into()));
    }

    if app.len() > MAX_APP_LEN {
        return Err(AppError::Validation(format!(
            "app name too long: {} bytes",
            app.len()
        )));
    }

    Ok(())
}

/// Record one switch to `app`, validating the key on the way in: this is the
/// only path that ever creates a row.
fn record_use(state: &State<'_, AppState>, app: &str) -> Result<(), AppError> {

    validate_app(app)?;

    let conn = lock_db(state)?;

    usage::bump(&conn, app, usage::now())
}

// async so Tauri runs these off the main thread; the platform helpers spawn a
// blocking subprocess and must not freeze the UI event loop.
#[tauri::command]
pub async fn list_open_apps(state: State<'_, AppState>) -> Result<OpenAppsResult, AppError> {

    let mut result = platform::list_open_apps()?;

    // The ordering is a nicety and the list is the feature, so a database
    // failure degrades to the platform's alphabetical order rather than leaving
    // the popup empty. Scoped so the guard, which is not Send, cannot outlive
    // the block and make this future non-Send.
    let scores = match lock_db(&state).and_then(|conn| usage::load_scores(&conn)) {
        Ok(scores) => scores,
        Err(e) => {
            log::warn!("list_open_apps: could not load usage scores: {e}");
            HashMap::new()
        }
    };

    usage::apply_order(&mut result.apps, &scores, usage::now());

    Ok(result)
}

#[tauri::command]
pub async fn focus_app(
    id: String,
    app: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {

    platform::focus_app(&id)?;

    // The switch already happened and the window is up. Nothing about recording
    // it, not even a rejected name, may report a failure for an action that
    // visibly succeeded.
    if let Err(e) = record_use(&state, &app) {
        log::warn!("focus_app: could not record usage for '{app}': {e}");
    }

    Ok(())
}

/// Frecency ordering for the open-apps list.
///
/// Keyed on `app`, the process name, never on `id`: `id` carries the window
/// title on macOS and the window handle on Windows, so neither survives a
/// restart, while the process name is also the icon key and the level at which
/// the owner thinks about an app. Every window of an app inherits its score.
///
/// Sits above the `#[cfg]` platform split, so one implementation covers both and
/// neither platform sort is touched.
mod usage {

    use super::OpenApp;
    use crate::error::AppError;

    use rusqlite::Connection;
    use std::cmp::Ordering;
    use std::collections::HashMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    const HOUR: i64 = 3600;
    const DAY: i64 = 24 * HOUR;

    pub fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    /// Uses, discounted by how long ago the app was last focused.
    ///
    /// Buckets rather than a continuous decay, deliberately: the order only moves
    /// when a boundary is crossed, so the top of the list holds still for hours
    /// and the `1`-`9` keys stay usable from muscle memory.
    pub fn score(uses: i64, last_used: i64, now: i64) -> f64 {

        let age = now - last_used;

        let weight = if age <= HOUR {
            8.0
        } else if age <= DAY {
            4.0
        } else if age <= 7 * DAY {
            2.0
        } else if age <= 30 * DAY {
            1.0
        } else {
            0.5
        };

        uses as f64 * weight
    }

    /// Sort by descending score, stably, on top of whatever order the platform
    /// module produced. An app with no row scores `0.0` and therefore keeps its
    /// alphabetical placement, and the several windows of one app share a score
    /// and are already adjacent, so they stay grouped and in title order.
    pub fn apply_order(apps: &mut [OpenApp], scores: &HashMap<String, (i64, i64)>, now: i64) {

        let score_of = |a: &OpenApp| {
            scores
                .get(&a.app)
                .map_or(0.0, |&(uses, last_used)| score(uses, last_used, now))
        };

        // `partial_cmp` cannot see a NaN here (`uses` is an integer and the
        // weights are literals); the fallback keeps the comparator total rather
        // than risking a panic on a contract violation.
        apps.sort_by(|a, b| score_of(b).partial_cmp(&score_of(a)).unwrap_or(Ordering::Equal));
    }

    pub fn load_scores(conn: &Connection) -> Result<HashMap<String, (i64, i64)>, AppError> {

        let mut stmt = conn.prepare("SELECT app, uses, last_used FROM app_usage")?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, (row.get(1)?, row.get(2)?)))
        })?;

        Ok(rows.collect::<Result<HashMap<_, _>, _>>()?)
    }

    pub fn bump(conn: &Connection, app: &str, now: i64) -> Result<(), AppError> {

        conn.execute(
            "INSERT INTO app_usage (app, uses, last_used) VALUES (?1, 1, ?2) \
             ON CONFLICT(app) DO UPDATE SET uses = uses + 1, last_used = ?2",
            rusqlite::params![app, now],
        )?;

        Ok(())
    }
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
    // Output lines: "<pid>\t<process name>\t<window title>" (title empty if none).
    pub fn list_open_apps() -> Result<OpenAppsResult, AppError> {

        let script = "set output to \"\"\n\
            tell application \"System Events\"\n\
            repeat with p in (every process whose background only is false)\n\
            set pname to name of p\n\
            set upid to unix id of p\n\
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
            set output to output & upid & tab & pname & tab & (t as text) & linefeed\n\
            set emitted to true\n\
            end if\n\
            end repeat\n\
            end if\n\
            end try\n\
            if not emitted then\n\
            set output to output & upid & tab & pname & tab & \"\" & linefeed\n\
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

            // "<pid>\t<process name>\t<window title>". The pid is what keeps two
            // instances of one app apart: their process names, and often their
            // window titles too, are identical.
            let mut fields = line.splitn(3, '\t');

            let (Some(pid), Some(app)) = (fields.next(), fields.next()) else {
                continue;
            };

            let pid = pid.trim();
            let app = app.trim();
            let title = fields.next().unwrap_or("").trim();

            if pid.is_empty() || app.is_empty() {
                continue;
            }

            let id = format!("{pid}{SEP}{app}{SEP}{title}");
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

    /// The running application with process id `pid`.
    ///
    /// Keyed on the pid the listing packed into the id rather than on the
    /// process name, which two instances of one app share.
    fn running_app(pid: i32) -> Option<objc2::rc::Retained<objc2_app_kit::NSRunningApplication>> {

        use objc2_app_kit::NSWorkspace;

        NSWorkspace::sharedWorkspace()
            .runningApplications()
            .iter()
            .find(|app| app.processIdentifier() == pid)
    }

    fn activate(app: &objc2_app_kit::NSRunningApplication) -> bool {
        use objc2_app_kit::NSApplicationActivationOptions;
        app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows)
    }

    /// Click the Window-menu entry named `title` in `app`'s own menu bar.
    ///
    /// The same tree System Events walks, reached directly instead: spawning
    /// `osascript` and opening an Apple Event connection to System Events costs
    /// about 180 ms, and that was the whole of the delay in switching apps.
    /// In-process the same click lands in a few milliseconds.
    ///
    /// Requires ClipX itself to hold Accessibility permission, where the scripted
    /// path borrowed System Events'. Returns false if the permission is missing
    /// or the menu entry is not found, leaving the caller its fallback.
    fn press_window_menu_item(pid: i32, title: &str) -> bool {

        let Some(app) = ax::Element::application(pid) else {
            return false;
        };

        let Some(menu_bar) = app.element_attribute("AXMenuBar") else {
            return false;
        };

        let Some(window_menu) = menu_bar
            .children()
            .into_iter()
            .find(|item| item.title().as_deref() == Some("Window"))
            .and_then(|item| item.children().into_iter().next())
        else {
            return false;
        };

        let items = window_menu.children();

        // Exact title first, then substring: Chrome decorates its titles.
        let target = items
            .iter()
            .find(|item| item.title().as_deref() == Some(title))
            .or_else(|| {
                items
                    .iter()
                    .find(|item| item.title().is_some_and(|t| t.contains(title)))
            });

        target.is_some_and(|item| item.press())
    }

    /// The slice of the Accessibility API needed to click one menu item.
    mod ax {

        use core_foundation::array::CFArray;
        use core_foundation::base::{CFRelease, CFRetain, CFTypeRef, TCFType};
        use core_foundation::string::{CFString, CFStringRef};

        use std::ffi::c_void;

        type AXUIElementRef = *const c_void;

        const AX_SUCCESS: i32 = 0;

        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
            fn AXUIElementCopyAttributeValue(
                element: AXUIElementRef,
                attribute: CFStringRef,
                value: *mut CFTypeRef,
            ) -> i32;
            fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> i32;
        }

        /// An owned accessibility element, released with the value it wraps.
        pub(super) struct Element(AXUIElementRef);

        impl Drop for Element {
            fn drop(&mut self) {
                unsafe { CFRelease(self.0) };
            }
        }

        impl Element {

            pub(super) fn application(pid: i32) -> Option<Self> {
                let element = unsafe { AXUIElementCreateApplication(pid) };
                (!element.is_null()).then_some(Self(element))
            }

            /// Copy an attribute whose value is another element.
            pub(super) fn element_attribute(&self, name: &str) -> Option<Self> {
                self.copy_attribute(name).map(Self)
            }

            pub(super) fn children(&self) -> Vec<Self> {

                let Some(value) = self.copy_attribute("AXChildren") else {
                    return Vec::new();
                };

                let array = unsafe { CFArray::<*const c_void>::wrap_under_create_rule(value as _) };

                // The array owns its elements, so each one handed out is retained
                // to outlive it.
                array
                    .iter()
                    .map(|item| {
                        let element = *item;
                        unsafe { CFRetain(element) };
                        Self(element)
                    })
                    .collect()
            }

            pub(super) fn title(&self) -> Option<String> {
                let value = self.copy_attribute("AXTitle")?;
                let title = unsafe { CFString::wrap_under_create_rule(value as _) };
                Some(title.to_string())
            }

            pub(super) fn press(&self) -> bool {
                let action = CFString::new("AXPress");
                AX_SUCCESS
                    == unsafe { AXUIElementPerformAction(self.0, action.as_concrete_TypeRef()) }
            }

            /// Copy an attribute value, owned by the caller. `None` when the
            /// attribute is absent or Accessibility is not permitted.
            fn copy_attribute(&self, name: &str) -> Option<CFTypeRef> {

                let key = CFString::new(name);
                let mut value: CFTypeRef = std::ptr::null();

                let status = unsafe {
                    AXUIElementCopyAttributeValue(self.0, key.as_concrete_TypeRef(), &mut value)
                };

                (status == AX_SUCCESS && !value.is_null()).then_some(value)
            }
        }
    }

    // id == "<pid>\u{1f}<process name>\u{1f}<window title>". Bring that exact
    // process to the front, then raise the specific window if a title is present.
    pub fn focus_app(id: &str) -> Result<(), AppError> {

        let mut fields = id.splitn(3, SEP);

        let (Some(pid), Some(app)) = (fields.next(), fields.next()) else {
            return Err(AppError::Validation("invalid app id".into()));
        };

        let title = fields.next().unwrap_or("");

        let pid: i32 = pid
            .parse()
            .map_err(|_| AppError::Validation(format!("invalid pid in app id: {pid}")))?;

        // The title is interpolated into the fallback script below; the pid is a
        // number by now and the process name is no longer interpolated at all.
        if title.chars().any(|c| matches!(c, '"' | '\\' | '\0' | '\r' | '\n' | '\t')) {
            return Err(AppError::Validation("invalid app id".into()));
        }

        // Fast path: activate the process directly, and reach into its menu bar
        // in-process when a specific window is wanted. Both cost single-digit
        // milliseconds, against about 180 ms for the scripted equivalent below,
        // which is why the switch used to lag behind the release of the keys.
        //
        // Selecting the window before activating keeps the app from coming
        // forward on the previously-active window first, same as the script.
        if let Some(running) = running_app(pid) {

            let window_ready = title.is_empty() || press_window_menu_item(pid, title);

            if window_ready && activate(&running) {
                return Ok(());
            }
        }

        // Reached when ClipX holds no Accessibility permission of its own, or
        // the window is not in the menu. Costs the ~180 ms the fast path saves.
        log::info!("focus_app: falling back to osascript for {app:?} (pid {pid})");

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
            tell (first process whose unix id is {pid})\n\
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

// Row building, icon extraction scripting, and icon caching for Windows.
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

    /// One top-level window as the enumerator found it, before validation.
    ///
    /// `handle` is the window's own handle, not its process's main window: an app
    /// contributes one of these per window it has open.
    pub struct RawWindow {
        pub handle: isize,
        pub process: String,
        pub path: String,
        pub title: String,
    }

    /// Turn enumerated windows into listed rows, sorted by `(app, title)`.
    ///
    /// The handle becomes `id`, which `focus_app` hands straight to Win32, so a
    /// row with a null handle is dropped here rather than listed as something
    /// that cannot be switched to. A titleless window has nothing to render as
    /// its label and is dropped for the same reason.
    pub fn build_rows(windows: Vec<RawWindow>) -> Vec<Row> {

        let mut rows: Vec<Row> = windows
            .into_iter()
            .filter_map(|w| {
                let title = w.title.trim();

                if title.is_empty() || w.handle == 0 {
                    return None;
                }

                Some(Row {
                    app: OpenApp {
                        name: title.to_string(),
                        id: w.handle.to_string(),
                        app: w.process.trim().to_string(),
                    },
                    path: w.path.trim().to_string(),
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

        fn win(handle: isize, process: &str, path: &str, title: &str) -> RawWindow {
            RawWindow {
                handle,
                process: process.to_string(),
                path: path.to_string(),
                title: title.to_string(),
            }
        }

        #[test]
        fn builds_rows_and_sorts_by_app_then_title() {

            let rows = build_rows(vec![
                win(300, "explorer", r"C:\Windows\explorer.exe", "Downloads"),
                win(100, "chrome", r"C:\chrome.exe", "Inbox"),
                win(101, "chrome", r"C:\chrome.exe", "Docs"),
            ]);

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

            assert_eq!(rows[0].app.id, "101", "id stays the window handle");
            assert_eq!(rows[0].path, r"C:\chrome.exe");
        }

        // The bug this listing exists to avoid: one row per window, never one row
        // per process, so every window of a multi-window app is reachable.
        #[test]
        fn lists_every_window_of_one_process() {

            let rows = build_rows(vec![
                win(10, "explorer", r"C:\Windows\explorer.exe", "Downloads"),
                win(11, "explorer", r"C:\Windows\explorer.exe", "Documents"),
                win(12, "explorer", r"C:\Windows\explorer.exe", "Pictures"),
            ]);

            assert_eq!(
                rows.iter().map(|r| r.app.id.as_str()).collect::<Vec<_>>(),
                vec!["11", "10", "12"],
                "three windows, three rows, each with its own handle"
            );
        }

        // Two instances of one app share a process name, and often a title too.
        // The handle is what keeps their rows apart.
        #[test]
        fn keeps_identically_titled_windows_apart() {

            let rows = build_rows(vec![
                win(20, "notepad", r"C:\notepad.exe", "Untitled - Notepad"),
                win(21, "notepad", r"C:\notepad.exe", "Untitled - Notepad"),
            ]);

            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].app.id, "20");
            assert_eq!(rows[1].app.id, "21");
        }

        #[test]
        fn keeps_rows_whose_path_is_unreadable() {

            // Access Denied on an elevated process: empty path, row still listed.
            let rows = build_rows(vec![win(42, "elevated", "", "Admin Console")]);

            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].app.app, "elevated");
            assert_eq!(rows[0].app.name, "Admin Console");
            assert!(rows[0].path.is_empty());
        }

        #[test]
        fn drops_titleless_rows() {

            let rows = build_rows(vec![
                win(10, "chrome", r"C:\chrome.exe", ""),
                win(11, "chrome", r"C:\chrome.exe", "   "),
                win(12, "chrome", r"C:\chrome.exe", "Real"),
            ]);

            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].app.name, "Real");
        }

        // `id` goes straight to Win32, so a row that could never be raised must
        // not reach the list in the first place.
        #[test]
        fn drops_rows_without_a_usable_window_handle() {

            let rows = build_rows(vec![
                win(0, "service", r"C:\svc.exe", "Null Handle"),
                win(500, "chrome", r"C:\chrome.exe", "Real"),
            ]);

            assert_eq!(
                rows.iter().map(|r| r.app.name.as_str()).collect::<Vec<_>>(),
                vec!["Real"]
            );
        }

        // 64-bit handles must survive the round trip that `focus_app` parses back
        // with `isize::from_str`.
        #[test]
        fn keeps_a_wide_window_handle_intact() {

            let rows = build_rows(vec![win(4_295_032_833, "editor", r"C:\editor.exe", "draft.txt")]);

            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].app.id, "4295032833");
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

            let rows = build_rows(vec![
                win(1, "chrome", r"C:\chrome.exe", "Inbox"),
                win(2, "chrome", r"C:\chrome.exe", "Docs"),
                win(3, "elevated", "", "Admin"),
            ]);

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
        build_rows, extraction_script, icon_cache, icons_by_app, parse_icon_pairs, split_cached,
        store, RawWindow,
    };
    use super::{OpenApp, OpenAppsResult};

    use crate::error::AppError;

    use std::collections::HashMap;
    use std::ffi::c_void;
    use std::os::windows::process::CommandExt;
    use std::path::Path;
    use std::process::Command;

    use windows::core::{BOOL, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, FALSE, HWND, LPARAM, TRUE};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{
        AttachThreadInput, GetCurrentProcessId, GetCurrentThreadId, OpenProcess,
        QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, EnumChildWindows, EnumWindows, GetAncestor, GetClassNameW,
        GetForegroundWindow,
        GetWindowLongW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
        IsWindow, IsWindowVisible, SetForegroundWindow, ShowWindow, GA_ROOTOWNER, GWL_EXSTYLE,
        SW_RESTORE, WS_EX_TOOLWINDOW,
    };

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    // DWMWA_CLOAKED values. A window the app itself cloaked is a suspended UWP
    // ghost and must not be listed; one the shell cloaked is a real window
    // sitting on another virtual desktop, which `SetForegroundWindow` switches
    // to, so it stays.
    const DWM_CLOAKED_APP: u32 = 1;
    const DWM_CLOAKED_INHERITED: u32 = 4;

    // Visible, titled, unowned top-level windows that are still not somewhere to
    // switch to: desktop furniture, and the raw UWP core window, which is either
    // system chrome ("Windows Input Experience") or the inner half of a UWP app
    // already listed through its ApplicationFrameWindow.
    const SKIP_CLASSES: [&str; 5] = [
        "Progman",
        "WorkerW",
        "Shell_TrayWnd",
        "Button",
        "Windows.UI.Core.CoreWindow",
    ];

    // The shell's host window for a UWP app. The app itself runs in a different
    // process, as a child of this window.
    const UWP_FRAME_CLASS: &str = "ApplicationFrameWindow";

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

    /// Every top-level window the user could alt-tab to.
    ///
    /// Replaces the old `Get-Process | Where MainWindowTitle -ne ''` listing,
    /// which was one row per *process* and therefore showed only a process's
    /// main window: a second Explorer window, a second Word document, a second
    /// window of any app that keeps them in one process was simply missing from
    /// the list. `EnumWindows` sees each of them.
    ///
    /// The filter is the standard alt-tab one: visible, titled, not owned by
    /// another window, not a tool window, not an app-cloaked ghost, and not one
    /// of ClipX's own windows, since the popup itself is on screen while this
    /// runs.
    fn enumerate_windows() -> Vec<RawWindow> {

        let mut handles: Vec<HWND> = Vec::new();

        // EnumWindows itself only fails if the callback stops the enumeration,
        // which this one never does; a partial list is still worth rendering.
        unsafe {
            let _ = EnumWindows(
                Some(collect_window),
                LPARAM(&mut handles as *mut Vec<HWND> as isize),
            );
        }

        let names = process_names();
        let mut paths: HashMap<u32, String> = HashMap::new();

        handles
            .into_iter()
            .map(|hwnd| {
                let owner = window_pid(hwnd).unwrap_or(0);

                // A UWP app is listed through its frame window, which belongs to
                // ApplicationFrameHost; name and icon it after the app inside.
                let pid = hosted_pid(hwnd, owner).unwrap_or(owner);

                // Several windows of one app share a process, so the executable
                // is looked up once per pid rather than once per window.
                let path = paths
                    .entry(pid)
                    .or_insert_with(|| process_path(pid))
                    .clone();

                RawWindow {
                    handle: hwnd.0 as isize,
                    process: names.get(&pid).cloned().unwrap_or_default(),
                    path,
                    title: window_title(hwnd),
                }
            })
            .collect()
    }

    unsafe extern "system" fn collect_window(hwnd: HWND, lparam: LPARAM) -> BOOL {

        if is_switchable(hwnd) {
            // The pointer is the `handles` vec above, alive for the whole
            // enumeration, and EnumWindows calls this synchronously on this
            // thread, so there is no other reference to alias.
            let handles = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
            handles.push(hwnd);
        }

        TRUE
    }

    /// The process actually behind a UWP frame window, or `None` for an ordinary
    /// window.
    ///
    /// Without this, every UWP app on the list would be called
    /// "ApplicationFrameHost" and share one icon. The frame keeps being the
    /// handle that is raised; only the identity comes from the child.
    ///
    /// A suspended UWP app has no core window parented to its frame, so it falls
    /// back to the host's identity. The row still carries the app's own window
    /// title, which is what the popup displays.
    fn hosted_pid(hwnd: HWND, owner: u32) -> Option<u32> {

        if class_name(hwnd) != UWP_FRAME_CLASS {
            return None;
        }

        // (frame's own pid, hosted pid once found).
        let mut found = (owner, 0u32);

        unsafe {
            let _ = EnumChildWindows(
                Some(hwnd),
                Some(collect_hosted),
                LPARAM(&mut found as *mut (u32, u32) as isize),
            );
        }

        (found.1 != 0).then_some(found.1)
    }

    unsafe extern "system" fn collect_hosted(hwnd: HWND, lparam: LPARAM) -> BOOL {

        let found = unsafe { &mut *(lparam.0 as *mut (u32, u32)) };

        match window_pid(hwnd) {
            // The first child living outside the host process is the app.
            Some(pid) if pid != found.0 => {
                found.1 = pid;
                FALSE
            }
            _ => TRUE,
        }
    }

    fn is_switchable(hwnd: HWND) -> bool {

        unsafe {

            if !IsWindowVisible(hwnd).as_bool() || GetWindowTextLengthW(hwnd) == 0 {
                return false;
            }

            // Owned windows (dialogs, palettes) are reached through their owner,
            // which the enumeration lists separately.
            if GetAncestor(hwnd, GA_ROOTOWNER) != hwnd {
                return false;
            }

            if GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 != 0 {
                return false;
            }

            if is_app_cloaked(hwnd) || SKIP_CLASSES.contains(&class_name(hwnd).as_str()) {
                return false;
            }

            // ClipX's popup is up while the list is being built, and its settings
            // and about windows may be too.
            window_pid(hwnd) != Some(GetCurrentProcessId())
        }
    }

    fn window_title(hwnd: HWND) -> String {

        let len = unsafe { GetWindowTextLengthW(hwnd) };

        if len <= 0 {
            return String::new();
        }

        // The title can grow between the two calls; the buffer only has to be
        // large enough for whatever the second one copies.
        let mut buf = vec![0u16; len as usize + 1];
        let copied = unsafe { GetWindowTextW(hwnd, &mut buf) };

        String::from_utf16_lossy(&buf[..copied.max(0) as usize])
    }

    fn class_name(hwnd: HWND) -> String {

        // Window class names are capped at 256 characters by the atom table.
        let mut buf = [0u16; 257];
        let copied = unsafe { GetClassNameW(hwnd, &mut buf) };

        String::from_utf16_lossy(&buf[..copied.max(0) as usize])
    }

    /// Whether the window is cloaked by its own app: a suspended UWP view that
    /// exists but is not on screen. Shell-cloaked windows (another virtual
    /// desktop) are real and stay in the list.
    fn is_app_cloaked(hwnd: HWND) -> bool {

        let mut cloaked = 0u32;

        let ok = unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED,
                &mut cloaked as *mut u32 as *mut c_void,
                std::mem::size_of::<u32>() as u32,
            )
        };

        ok.is_ok() && matches!(cloaked, DWM_CLOAKED_APP | DWM_CLOAKED_INHERITED)
    }

    /// pid -> process name, without the `.exe`, matching what `Get-Process` used
    /// to report and what the usage table and icon map are keyed by.
    ///
    /// A snapshot rather than `QueryFullProcessImageNameW` because it needs no
    /// handle on the process, so an elevated app still gets a name.
    fn process_names() -> HashMap<u32, String> {

        let mut names = HashMap::new();

        unsafe {
            
            let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
                log::warn!("list_open_apps: could not snapshot processes");
                return names;
            };

            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };

            if Process32FirstW(snapshot, &mut entry).is_ok() {
                loop {
                    let end = entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len());

                    let file = String::from_utf16_lossy(&entry.szExeFile[..end]);

                    let name = Path::new(&file)
                        .file_stem()
                        .map(|stem| stem.to_string_lossy().into_owned())
                        .unwrap_or(file);

                    names.insert(entry.th32ProcessID, name);

                    if Process32NextW(snapshot, &mut entry).is_err() {
                        break;
                    }
                }
            }

            let _ = CloseHandle(snapshot);
        }

        names
    }

    /// The executable behind `pid`, or empty when it cannot be read.
    ///
    /// This is only the icon cache key, and opening a process of higher
    /// integrity fails for an unelevated ClipX, so an elevated app lists without
    /// an icon exactly as it did when `$_.Path` threw.
    fn process_path(pid: u32) -> String {

        if pid == 0 {
            return String::new();
        }

        unsafe {
            let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                return String::new();
            };

            let mut buf = vec![0u16; 1024];
            let mut len = buf.len() as u32;

            let path = QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
            .map(|()| String::from_utf16_lossy(&buf[..len as usize]))
            .unwrap_or_default();

            let _ = CloseHandle(process);

            path
        }
    }

    // One row per window: id = that window's handle, name = its title, app = the
    // owning process name.
    pub fn list_open_apps() -> Result<OpenAppsResult, AppError> {

        let rows = build_rows(enumerate_windows());

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

    /// Bring `hwnd` to the foreground, in-process.
    ///
    /// `SetForegroundWindow` is granted only to the process owning the current
    /// foreground window or the one that received the last input event. The
    /// popup hides itself before this runs, so ClipX has just given up the
    /// first; attaching this thread to the foreground window's input queue is
    /// how `AppActivate` borrows the right internally, and is why the plain call
    /// is retried rather than trusted once.
    fn raise_window(hwnd: HWND) -> bool {

        unsafe {

            // The list is a snapshot: the window may have closed between the
            // popup opening and the user picking a row.
            if !IsWindow(Some(hwnd)).as_bool() {
                return false;
            }

            // A minimized window takes foreground status without ever becoming
            // visible, so restore it before raising.
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }

            if SetForegroundWindow(hwnd).as_bool() {
                return true;
            }

            let foreground = GetWindowThreadProcessId(GetForegroundWindow(), None);
            let current = GetCurrentThreadId();

            // Nothing to borrow from: no foreground window, or it is already
            // ours and the call above still failed.
            if foreground == 0 || foreground == current {
                return false;
            }

            if !AttachThreadInput(current, foreground, true).as_bool() {
                return false;
            }

            let raised = SetForegroundWindow(hwnd).as_bool();
            let _ = BringWindowToTop(hwnd);

            // Detach unconditionally: leaving this thread's input queue wired to
            // another app's would outlive the switch.
            let _ = AttachThreadInput(current, foreground, false);

            raised
        }
    }

    /// The process owning `hwnd`, used both to name and to key the listing and
    /// by the fallback below. `None` once the window is gone.
    fn window_pid(hwnd: HWND) -> Option<u32> {

        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };

        (pid != 0).then_some(pid)
    }

    // id == that window's own handle on Windows.
    pub fn focus_app(id: &str) -> Result<(), AppError> {

        let handle: isize = id
            .parse()
            .map_err(|_| AppError::Validation(format!("invalid app id: {id}")))?;

        let hwnd = HWND(handle as *mut c_void);

        // Fast path: raise the window through Win32 directly. Spawning
        // `powershell.exe` for the scripted equivalent below costs about 800 ms
        // of interpreter startup and COM activation before it does any work,
        // and that was the whole of the delay in switching apps.
        if raise_window(hwnd) {
            return Ok(());
        }

        // Reached when the foreground lock refuses us even attached. Costs the
        // ~800 ms the fast path saves.
        log::info!("focus_app: falling back to powershell for window {handle}");

        let pid = window_pid(hwnd)
            .ok_or_else(|| AppError::State(format!("window {handle} no longer exists")))?;

        // AppActivate accepts a PID, but raises that process's main window
        // rather than the one picked: the best a fallback can do once Win32 has
        // refused the exact handle, and still the right app.
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

    #[cfg(test)]
    mod tests {

        use std::collections::{HashMap, HashSet};

        // Acceptance evidence against the real desktop: prints what the popup
        // would show. A headless runner has no windows to enumerate, so an empty
        // listing skips rather than fails.
        #[test]
        fn lists_each_window_separately() {

            let started = std::time::Instant::now();
            let result = super::list_open_apps().expect("listing must not fail");
            let cold = started.elapsed();

            if result.apps.is_empty() {
                eprintln!("skipped: no top-level windows on this desktop");
                return;
            }

            // Second call has every icon cached, so it spawns no subprocess at
            // all: the enumeration itself is pure Win32.
            let started = std::time::Instant::now();
            let _ = super::list_open_apps();

            eprintln!(
                "{} rows, {} icon keys, {cold:?} cold / {:?} warm",
                result.apps.len(),
                result.icons.len(),
                started.elapsed()
            );

            let mut ids = HashSet::new();
            let mut per_app: HashMap<&str, usize> = HashMap::new();

            for row in &result.apps {
                assert!(!row.app.is_empty(), "empty app field for {:?}", row.name);
                assert!(!row.name.is_empty(), "empty title for {:?}", row.app);
                assert!(
                    row.id.parse::<isize>().is_ok_and(|h| h != 0),
                    "id {:?} is not a usable window handle",
                    row.id
                );
                assert!(ids.insert(row.id.clone()), "duplicate id {:?}", row.id);

                *per_app.entry(row.app.as_str()).or_default() += 1;

                eprintln!(
                    "  app={:?} icon={} title={:?}",
                    row.app,
                    result.icons.contains_key(&row.app),
                    row.name
                );
            }

            // Not an assertion: whether any app happens to have two windows open
            // is up to whoever is running the test.
            for (app, windows) in per_app.iter().filter(|(_, &n)| n > 1) {
                eprintln!("  {app:?} contributed {windows} windows");
            }
        }
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


#[cfg(test)]
mod tests {

    use super::usage::{apply_order, bump, load_scores, score};
    use super::{validate_app, OpenApp};

    use crate::db::init_db;
    use crate::error::AppError;

    use rusqlite::Connection;
    use std::collections::HashMap;

    const HOUR: i64 = 3600;
    const DAY: i64 = 24 * HOUR;
    const NOW: i64 = 1_700_000_000;

    fn app(name: &str, title: &str) -> OpenApp {
        OpenApp {
            name: format!("{name} - {title}"),
            id: format!("{name}\u{1f}{title}"),
            app: name.to_string(),
        }
    }

    fn names(apps: &[OpenApp]) -> Vec<&str> {
        apps.iter().map(|a| a.name.as_str()).collect()
    }

    fn db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        init_db(&mut conn).unwrap();
        conn
    }

    #[test]
    fn score_weights_each_bucket() {
        assert_eq!(score(1, NOW - 60, NOW), 8.0);
        assert_eq!(score(1, NOW - 2 * HOUR, NOW), 4.0);
        assert_eq!(score(1, NOW - 3 * DAY, NOW), 2.0);
        assert_eq!(score(1, NOW - 10 * DAY, NOW), 1.0);
        assert_eq!(score(1, NOW - 90 * DAY, NOW), 0.5);
    }

    // Pins the comparison operator by test rather than by reading: an age landing
    // exactly on a boundary belongs to the nearer bucket.
    #[test]
    fn score_boundaries_are_inclusive() {
        assert_eq!(score(1, NOW - HOUR, NOW), 8.0);
        assert_eq!(score(1, NOW - HOUR - 1, NOW), 4.0);
        assert_eq!(score(1, NOW - DAY, NOW), 4.0);
        assert_eq!(score(1, NOW - DAY - 1, NOW), 2.0);
        assert_eq!(score(1, NOW - 7 * DAY, NOW), 2.0);
        assert_eq!(score(1, NOW - 7 * DAY - 1, NOW), 1.0);
        assert_eq!(score(1, NOW - 30 * DAY, NOW), 1.0);
        assert_eq!(score(1, NOW - 30 * DAY - 1, NOW), 0.5);
    }

    #[test]
    fn score_multiplies_by_uses() {
        assert_eq!(score(5, NOW - 60, NOW), 40.0);
    }

    #[test]
    fn an_app_with_no_row_scores_zero() {
        let mut apps = vec![app("Zed", "main"), app("Chrome", "Inbox")];
        let scores = HashMap::new();

        apply_order(&mut apps, &scores, NOW);

        // No row anywhere means no reordering at all.
        assert_eq!(names(&apps), vec!["Zed - main", "Chrome - Inbox"]);
    }

    #[test]
    fn a_used_app_rises_above_an_alphabetically_earlier_one() {
        let mut apps = vec![app("Chrome", "Inbox"), app("Zed", "main")];
        let scores = HashMap::from([("Zed".to_string(), (3, NOW - 60))]);

        apply_order(&mut apps, &scores, NOW);

        assert_eq!(names(&apps), vec!["Zed - main", "Chrome - Inbox"]);
    }

    #[test]
    fn windows_of_one_app_stay_adjacent_and_in_order() {
        let mut apps = vec![
            app("Chrome", "Docs"),
            app("Chrome", "Inbox"),
            app("Slack", "general"),
            app("Zed", "main"),
        ];
        let scores = HashMap::from([("Chrome".to_string(), (10, NOW - 60))]);

        apply_order(&mut apps, &scores, NOW);

        assert_eq!(
            names(&apps),
            vec!["Chrome - Docs", "Chrome - Inbox", "Slack - general", "Zed - main"]
        );
    }

    // The "nothing learned yet, behave exactly as today" guarantee.
    #[test]
    fn a_list_with_no_scores_is_left_identical() {
        let original = vec![app("Chrome", "Inbox"), app("Slack", "general"), app("Zed", "main")];
        let mut apps = original.clone();

        apply_order(&mut apps, &HashMap::new(), NOW);

        assert_eq!(names(&apps), names(&original));
        assert_eq!(
            apps.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
            original.iter().map(|a| a.id.as_str()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn first_bump_inserts_and_second_increments() {
        let conn = db();

        bump(&conn, "Zed", NOW).unwrap();
        assert_eq!(load_scores(&conn).unwrap()["Zed"], (1, NOW));

        bump(&conn, "Zed", NOW + 500).unwrap();
        assert_eq!(load_scores(&conn).unwrap()["Zed"], (2, NOW + 500));
    }

    #[test]
    fn load_scores_is_empty_on_a_fresh_db() {
        assert!(load_scores(&db()).unwrap().is_empty());
    }

    #[test]
    fn validate_app_rejects_empty_and_over_long_names() {
        assert!(matches!(validate_app(""), Err(AppError::Validation(_))));
        assert!(matches!(
            validate_app(&"a".repeat(257)),
            Err(AppError::Validation(_))
        ));
        assert!(validate_app(&"a".repeat(256)).is_ok());
        assert!(validate_app("Zed").is_ok());
    }

    #[test]
    fn init_db_prunes_stale_rows_and_spares_recent_ones() {
        let mut conn = db();

        conn.execute(
            "INSERT INTO app_usage (app, uses, last_used) VALUES \
             ('Stale', 9, strftime('%s','now') - 15552001), \
             ('Fresh', 1, strftime('%s','now') - 86400)",
            [],
        )
        .unwrap();

        init_db(&mut conn).unwrap();

        let scores = load_scores(&conn).unwrap();
        assert!(!scores.contains_key("Stale"));
        assert!(scores.contains_key("Fresh"));
    }
}
