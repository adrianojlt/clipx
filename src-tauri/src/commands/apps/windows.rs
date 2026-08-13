//! Listing, focusing and window cycling for Windows.

use super::window_cycle::next_window;
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
    IsWindow, IsWindowVisible, SetForegroundWindow, SetWindowPos, ShowWindow, GA_ROOTOWNER,
    GWL_EXSTYLE, HWND_BOTTOM, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_RESTORE,
    WS_EX_TOOLWINDOW,
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

/// Drop `hwnd` to the bottom of the Z-order, without activating, moving or
/// resizing it. Permitted across processes: the foreground lock guards
/// activation, not stacking.
fn send_to_back(hwnd: HWND) {

    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_BOTTOM),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

/// What makes two windows "the same app" to the cycler: the path of the
/// executable behind them.
///
/// The path rather than the pid, because Chromium and Electron apps spread
/// their windows across several processes while some apps start a fresh
/// process per window, and pid-matching splits one app into several in both
/// cases. Where the path cannot be read - an unelevated ClipX cannot open an
/// elevated process - the pid stands in: still groups the windows of a
/// single-process app, and never merges two different ones.
fn app_identity(hwnd: HWND, cache: &mut HashMap<u32, String>) -> String {

    let owner = window_pid(hwnd).unwrap_or(0);

    // A UWP window is enumerated as its ApplicationFrameHost frame; the app
    // inside is what decides which windows belong together.
    let pid = hosted_pid(hwnd, owner).unwrap_or(owner);

    cache
        .entry(pid)
        .or_insert_with(|| match process_path(pid) {
            path if path.is_empty() => format!("pid:{pid}"),
            // Windows paths are case-insensitive; two spellings of one
            // executable must not read as two apps.
            path => path.to_lowercase(),
        })
        .clone()
}

/// Every switchable window on the desktop, front to back, paired with its
/// app identity.
///
/// Deliberately not built on `list_open_apps`: `EnumWindows` hands windows
/// back in Z-order and the rotation depends on that, while the listing path
/// sorts by app and then by frecency, so the order is gone by the time the
/// rows come back.
fn switchable_windows_in_z_order() -> Vec<(isize, String)> {

    let mut handles: Vec<HWND> = Vec::new();

    unsafe {
        let _ = EnumWindows(
            Some(collect_window),
            LPARAM(&mut handles as *mut Vec<HWND> as isize),
        );
    }

    let mut identities: HashMap<u32, String> = HashMap::new();

    handles
        .into_iter()
        .map(|hwnd| (hwnd.0 as isize, app_identity(hwnd, &mut identities)))
        .collect()
}

/// Rotate the focused app's windows, the way Alt+Esc rotates the desktop's.
///
/// Raises the app's next window and sends the outgoing one to the back of
/// the Z-order. The demotion is what makes a third window reachable: raising
/// alone leaves the outgoing window second in Z-order, so the next press
/// would raise it straight back and the cycle would never leave that pair.
///
/// `Ok(false)` means there was nothing to do, which is the ordinary result
/// for a single-window app.
pub fn cycle_active_app_windows() -> Result<bool, AppError> {

    let foreground = unsafe { GetForegroundWindow() };

    if foreground.is_invalid() {
        return Ok(false);
    }

    // A focused dialog or palette is reached through its owner, which is the
    // window the enumeration lists.
    let current = unsafe { GetAncestor(foreground, GA_ROOTOWNER) };

    let windows = switchable_windows_in_z_order();

    let Some(next) = next_window(&windows, current.0 as isize) else {
        return Ok(false);
    };

    let next = HWND(next as *mut c_void);

    // Nothing has moved yet, so a refusal here leaves the desktop exactly as
    // it was rather than half-rotated.
    if !raise_window(next) {
        return Err(AppError::State(format!(
            "could not raise window {}",
            next.0 as isize
        )));
    }

    send_to_back(current);

    Ok(true)
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

    // Acceptance evidence for the cycler against the real desktop: how it
    // groups the windows on screen, and where each press would land.
    //
    // Observational on purpose - it enumerates and decides but raises
    // nothing - so running it on a live session moves no windows.
    #[test]
    fn groups_the_live_desktop_into_cycles() {

        let windows = super::switchable_windows_in_z_order();

        if windows.is_empty() {
            eprintln!("skipped: no switchable windows on this desktop");
            return;
        }

        let mut by_app: HashMap<&str, Vec<isize>> = HashMap::new();

        for (handle, identity) in &windows {
            by_app.entry(identity.as_str()).or_default().push(*handle);
        }

        eprintln!(
            "{} switchable windows across {} apps",
            windows.len(),
            by_app.len()
        );

        for (identity, handles) in &by_app {

            let name = std::path::Path::new(identity)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| identity.to_string());

            if handles.len() < 2 {
                eprintln!("  {name}: 1 window, nothing to cycle");
                assert_eq!(
                    super::next_window(&windows, handles[0]),
                    None,
                    "{name}: a lone window must not cycle"
                );
                continue;
            }

            // Stepping from any window of the app must stay inside the app,
            // never stand still, and reach every one of its windows.
            let mut visited = Vec::new();
            let mut at = handles[0];

            for _ in 0..handles.len() {
                let next = super::next_window(&windows, at)
                    .unwrap_or_else(|| panic!("{name}: {at} should cycle"));

                assert_ne!(next, at, "{name}: cycled to itself");
                assert!(handles.contains(&next), "{name}: cycled out of the app");

                visited.push(next);
                at = next;
            }

            visited.sort_unstable();
            let mut expected = handles.clone();
            expected.sort_unstable();

            assert_eq!(
                visited, expected,
                "{name}: pressing once per window must visit them all"
            );

            eprintln!("  {name}: {} windows, full cycle reachable", handles.len());
        }
    }
}
