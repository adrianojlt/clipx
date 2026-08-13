//! Row building, icon extraction scripting, and icon caching for Windows.
//!
//! Kept outside `windows`, which cannot compile on a non-Windows host because of
//! its `std::os::windows` import, so these pure functions stay unit-testable from
//! any development machine. Compiled under `test` everywhere for that reason.

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
