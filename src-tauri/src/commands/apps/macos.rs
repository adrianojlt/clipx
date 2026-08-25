//! Listing, focusing and icon extraction for macOS.

use super::{OpenApp, OpenAppsResult};
use crate::error::AppError;

use std::collections::HashMap;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

// Separator packed into `id` to carry the pid, the window index and the title.
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

        if let Some(stem) = path_stem(&path) {
            out.insert(stem, uri);
        }
    }

    out
}

/// The file name of `path` without its extension, the second of the two keys
/// the icon map carries: the name an unbundled binary is listed under.
fn path_stem(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
}

/// The name an app is listed and keyed under: its localized name, falling back
/// to the executable stem. Matches how `icons_from` keys the icon map, so every
/// row finds its icon.
fn app_name(app: &objc2_app_kit::NSRunningApplication) -> Option<String> {

    if let Some(name) = app.localizedName() {
        return Some(name.to_string());
    }

    app.executableURL()
        .and_then(|url| url.path())
        .and_then(|path| path_stem(&path.to_string()))
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

/// One row per titled window, read straight out of the Accessibility API.
///
/// Each process's Window menu, walked through System Events, used to be the
/// source. It cost a subprocess (about 1.8 s with a dozen apps open), raced the
/// process list (`-1719 Invalid index` when an app quit mid-iteration), and,
/// worst, returned no menu at all unless the calling app is itself allowed
/// assistive access - which the enclosing `try` swallowed, silently collapsing
/// every app to a single row. `AXWindows` in-process removes all three.
pub fn list_open_apps() -> Result<OpenAppsResult, AppError> {

    use objc2_app_kit::{NSApplicationActivationPolicy, NSWorkspace};

    // The permission is the one thing that cannot be worked around here, and
    // its symptom - one row per app, no windows - looks like a listing bug
    // rather than a missing grant, so name it in the log.
    if !ax::is_trusted() {
        // The popup lists on every open, and the grant cannot change without a
        // restart, so one line per process is all this is worth.
        static WARNED: std::sync::Once = std::sync::Once::new();
        WARNED.call_once(|| log::warn!(
            "list_open_apps: ClipX is not trusted for Accessibility, so no window \
             titles can be read and every app collapses to one row. Grant it under \
             System Settings > Privacy & Security > Accessibility."
        ));
    }

    let mut apps: Vec<OpenApp> = Vec::new();

    for app in NSWorkspace::sharedWorkspace().runningApplications().iter() {

        if app.activationPolicy() != NSApplicationActivationPolicy::Regular {
            continue;
        }

        let pid = app.processIdentifier();

        let Some(name) = app_name(&app) else {
            continue;
        };

        let windows = window_titles(pid);

        // An app with no titled window is still worth listing: switching to it
        // is exactly what the app-only row does.
        if windows.is_empty() {
            apps.push(OpenApp {
                name: name.clone(),
                id: window_id(pid, None, ""),
                app: name,
            });
            continue;
        }

        for (index, title) in windows {
            apps.push(OpenApp {
                name: format!("{name} - {title}"),
                id: window_id(pid, Some(index), &title),
                app: name.clone(),
            });
        }
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(OpenAppsResult {
        apps,
        icons: icons(),
    })
}

/// Pack the identity of one row: the process, the window's position in
/// `AXWindows`, and its title. `None` for an app listed without a window.
///
/// This is the whole contract between `list_open_apps` and `focus_app`; the
/// frontend passes the string back untouched.
fn window_id(pid: i32, index: Option<usize>, title: &str) -> String {

    match index {
        Some(index) => format!("{pid}{SEP}{index}{SEP}{title}"),
        None => format!("{pid}{SEP}{SEP}"),
    }
}

/// Unpack what `window_id` built.
///
/// A missing or unparseable index means the row named no window, which is a
/// request to raise the app alone rather than an error.
fn parse_window_id(id: &str) -> Result<(i32, Option<usize>, &str), AppError> {

    let mut fields = id.splitn(3, SEP);

    let pid = fields.next().unwrap_or_default();
    let index = fields.next().and_then(|index| index.parse().ok());
    let title = fields.next().unwrap_or_default();

    let pid: i32 = pid
        .parse()
        .map_err(|_| AppError::Validation(format!("invalid pid in app id: {pid}")))?;

    Ok((pid, index, title))
}

/// The titled windows of `pid`, paired with their position in `AXWindows`.
///
/// Two windows of one app routinely carry the same title (two Finder folders
/// named Pictures, two untitled editors), so the position is what keeps their
/// rows apart; the title alone cannot.
///
/// Untitled windows are dropped: there is nothing to render as their label.
fn window_titles(pid: i32) -> Vec<(usize, String)> {

    let Some(app) = ax::Element::application(pid) else {
        return Vec::new();
    };

    app.elements("AXWindows")
        .into_iter()
        .enumerate()
        .filter_map(|(index, window)| {
            let title = window.title()?;
            (!title.trim().is_empty()).then_some((index, title))
        })
        .collect()
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

/// Raise the window that `index` and `title` identify inside `pid`.
///
/// `AXRaise` is honored by Chromium and Electron windows too - verified on
/// Chrome and VS Code - so the whole switch is a few in-process calls, against
/// about 180 ms for the scripted equivalent.
///
/// `index` is the window's position in `AXWindows` when the list was built, and
/// an app reorders that array as its windows are raised, so the title is
/// checked before the index is trusted and the index only breaks ties between
/// identically titled windows.
///
/// Requires ClipX itself to hold Accessibility permission, where the scripted
/// path borrowed System Events'. Returns false if the permission is missing or
/// the window is gone, leaving the caller its fallback.
fn raise_window(pid: i32, index: Option<usize>, title: &str) -> bool {

    let Some(app) = ax::Element::application(pid) else {
        return false;
    };

    let windows = app.elements("AXWindows");

    let Some(target) = index
        .and_then(|i| windows.get(i))
        .filter(|window| window.title().as_deref() == Some(title))
        .or_else(|| windows.iter().find(|window| window.title().as_deref() == Some(title)))
        .or_else(|| index.and_then(|i| windows.get(i)))
    else {
        return false;
    };

    // A minimized window is listed like any other, and raising it leaves it in
    // the Dock, so restore it first or the switch does nothing visible.
    if target.bool_attribute("AXMinimized") == Some(true) {
        target.set_bool("AXMinimized", false);
    }

    target.perform("AXRaise")
}

/// The slice of the Accessibility API needed to list and raise windows.
mod ax {

    use core_foundation::array::CFArray;
    use core_foundation::base::{CFRelease, CFRetain, CFTypeRef, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::string::{CFString, CFStringRef};

    use std::ffi::c_void;

    type AXUIElementRef = *const c_void;

    const AX_SUCCESS: i32 = 0;

    // An app that stops answering must not hang the popup: every window of every
    // running app is read on the path the user waits on, so one wedged app would
    // otherwise stall the whole list for the API's default minute.
    const MESSAGING_TIMEOUT: f32 = 1.0;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
        fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: *mut CFTypeRef,
        ) -> i32;
        fn AXUIElementSetAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: CFTypeRef,
        ) -> i32;
        fn AXUIElementSetMessagingTimeout(element: AXUIElementRef, timeout: f32) -> i32;
        fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> i32;
    }

    /// Whether ClipX may read other apps' UI. Every call below returns nothing
    /// when this is false, and the grant is per binary: a bundled build is a
    /// different identity than the one `cargo tauri dev` produces.
    pub(super) fn is_trusted() -> bool {
        unsafe { AXIsProcessTrusted() }
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

            if element.is_null() {
                return None;
            }

            // Inherited by every element copied out of this one.
            unsafe { AXUIElementSetMessagingTimeout(element, MESSAGING_TIMEOUT) };

            Some(Self(element))
        }

        /// Copy an attribute whose value is an array of elements.
        pub(super) fn elements(&self, name: &str) -> Vec<Self> {

            let Some(value) = self.copy_attribute(name) else {
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

        pub(super) fn bool_attribute(&self, name: &str) -> Option<bool> {
            let value = self.copy_attribute(name)?;
            let flag = unsafe { CFBoolean::wrap_under_create_rule(value as _) };
            Some(flag.into())
        }

        pub(super) fn set_bool(&self, name: &str, value: bool) -> bool {

            let key = CFString::new(name);
            let flag = CFBoolean::from(value);

            AX_SUCCESS
                == unsafe {
                    AXUIElementSetAttributeValue(
                        self.0,
                        key.as_concrete_TypeRef(),
                        flag.as_CFTypeRef(),
                    )
                }
        }

        pub(super) fn perform(&self, action: &str) -> bool {
            let action = CFString::new(action);
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

/// macOS cycles an app's own windows with Command+` already, so ClipX has no
/// reason to reimplement it. Only reachable if the hotkey, unset here by
/// default, is bound by hand.
pub fn cycle_active_app_windows(_reverse: bool) -> Result<bool, AppError> {
    Err(AppError::State(
        "cycling an app's windows is Windows-only; macOS has Command+`".into(),
    ))
}

// id == "<pid>\u{1f}<window index>\u{1f}<window title>", with both window
// fields empty for an app that had no titled window to list. Raise that exact
// window, then bring its process to the front.
pub fn focus_app(id: &str) -> Result<(), AppError> {

    let (pid, index, title) = parse_window_id(id)?;

    let Some(running) = running_app(pid) else {
        return Err(AppError::State(format!("no running app with pid {pid}")));
    };

    // Selecting the window before activating keeps the app from coming forward
    // on the previously-active window first.
    if (title.is_empty() || raise_window(pid, index, title)) && activate(&running) {
        return Ok(());
    }

    // Reached when ClipX holds no Accessibility permission of its own, or the
    // window is gone. Costs the ~180 ms the fast path saves.
    log::info!("focus_app: falling back to osascript for pid {pid}");

    // The title is interpolated into the script, so a window titled with a
    // quote or a backslash gets the app raised without it rather than a broken
    // script; the pid is a number by now.
    let usable_title = !title.is_empty()
        && !title.chars().any(|c| matches!(c, '"' | '\\' | '\0' | '\r' | '\n' | '\t'));

    // Raise the window via the app's own Window menu item. Its entries are
    // shorter than the AXTitle this id carries - Chrome appends the profile and
    // the group, editors append the folder - so the match is tried in both
    // directions: `contains` for a decorated menu entry, `is in` for the
    // decorated id.
    let inner = if usable_title {
        format!(
            "try\n\
            click (first menu item of menu 1 of menu bar item \"Window\" of menu bar 1 whose name is \"{title}\")\n\
            on error\n\
            try\n\
            click (first menu item of menu 1 of menu bar item \"Window\" of menu bar 1 whose name contains \"{title}\")\n\
            on error\n\
            try\n\
            click (first menu item of menu 1 of menu bar item \"Window\" of menu bar 1 whose name is in \"{title}\")\n\
            end try\n\
            end try\n\
            end try\n\
            set frontmost to true"
        )
    } else {
        "set frontmost to true".to_string()
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
    use crate::error::AppError;
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
    // The id is the whole contract between the listing and `focus_app`, and the
    // only part of this module that runs without a live Accessibility grant.
    #[test]
    fn window_id_round_trips() {

        let id = super::window_id(4321, Some(2), "draft.txt");

        assert_eq!(super::parse_window_id(&id).unwrap(), (4321, Some(2), "draft.txt"));
    }

    #[test]
    fn an_app_without_windows_round_trips_to_no_window() {

        let id = super::window_id(77, None, "");

        assert_eq!(super::parse_window_id(&id).unwrap(), (77, None, ""));
    }

    // Titles carry anything a window can be called, separators included.
    #[test]
    fn a_title_keeps_its_own_separators() {

        let id = super::window_id(9, Some(0), "a - b\u{1f}c");

        assert_eq!(super::parse_window_id(&id).unwrap(), (9, Some(0), "a - b\u{1f}c"));
    }

    #[test]
    fn a_non_numeric_pid_is_rejected() {

        assert!(matches!(
            super::parse_window_id("Finder\u{1f}\u{1f}"),
            Err(AppError::Validation(_))
        ));
    }

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
