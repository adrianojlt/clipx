//! Rotation order for the window cycler. Kept outside `windows` for the same
//! reason as `windows_icons`: pure logic, unit-testable from any development
//! machine, with the Win32 calls left on the other side of the boundary.

/// The next window to raise when cycling within one app.
///
/// `windows` is every switchable window on the desktop in Z-order, front to
/// back, each paired with the identity of the app that owns it; `current` is
/// the focused window. Returns the entry that follows `current` among its own
/// app's windows, wrapping at the end.
///
/// `None` when there is nothing to cycle to: the focused window is not one
/// the enumeration offers (ClipX's own popup, desktop furniture), or its app
/// has only the one window.
pub fn next_window(windows: &[(isize, String)], current: isize) -> Option<isize> {

    let identity = &windows.iter().find(|(handle, _)| *handle == current)?.1;

    let same_app: Vec<isize> = windows
        .iter()
        .filter(|(_, owner)| owner == identity)
        .map(|(handle, _)| *handle)
        .collect();

    if same_app.len() < 2 {
        return None;
    }

    let at = same_app.iter().position(|handle| *handle == current)?;

    Some(same_app[(at + 1) % same_app.len()])
}

/// The window to raise when cycling the other way.
///
/// The inverse of `next_window` under the Z-order each of them produces: the
/// backmost of the app's own windows. The caller raises it and, unlike the
/// forward step, demotes nothing - raising the backmost window is by itself
/// what moves the rotation one place back, and pushing the outgoing window
/// down as well would skip an entry.
///
/// `None` in the same cases as `next_window`.
pub fn previous_window(windows: &[(isize, String)], current: isize) -> Option<isize> {

    let identity = &windows.iter().find(|(handle, _)| *handle == current)?.1;

    let same_app: Vec<isize> = windows
        .iter()
        .filter(|(_, owner)| owner == identity)
        .map(|(handle, _)| *handle)
        .collect();

    if same_app.len() < 2 {
        return None;
    }

    let at = same_app.iter().position(|handle| *handle == current)?;

    Some(same_app[(at + same_app.len() - 1) % same_app.len()])
}

#[cfg(test)]
mod tests {

    use super::{next_window, previous_window};

    fn desktop(entries: &[(isize, &str)]) -> Vec<(isize, String)> {
        entries
            .iter()
            .map(|(handle, owner)| (*handle, owner.to_string()))
            .collect()
    }

    #[test]
    fn advances_to_the_next_window_of_the_same_app() {

        let windows = desktop(&[(1, "chrome.exe"), (2, "chrome.exe"), (3, "chrome.exe")]);

        assert_eq!(next_window(&windows, 1), Some(2));
    }

    // The whole point of the feature: windows of other apps sit between the
    // focused app's own in Z-order and must be stepped over, not landed on.
    #[test]
    fn steps_over_windows_of_other_apps() {

        let windows = desktop(&[
            (1, "code.exe"),
            (2, "chrome.exe"),
            (3, "explorer.exe"),
            (4, "code.exe"),
        ]);

        assert_eq!(next_window(&windows, 1), Some(4));
    }

    // Reachable once the cycler has demoted a window to the back of the
    // Z-order and the user comes back round to it.
    #[test]
    fn wraps_around_at_the_end() {

        let windows = desktop(&[(1, "code.exe"), (2, "chrome.exe"), (3, "code.exe")]);

        assert_eq!(next_window(&windows, 3), Some(1));
    }

    // Three presses must visit all three windows. Raising alone would leave
    // the previous window second in Z-order and ping-pong between two, which
    // is why the caller also sends the outgoing window to the back: this
    // models the Z-order it produces.
    #[test]
    fn three_presses_visit_every_window() {

        let mut order = vec![1isize, 2, 3];
        let mut visited = Vec::new();

        for _ in 0..3 {

            let windows = desktop(
                &order
                    .iter()
                    .map(|handle| (*handle, "code.exe"))
                    .collect::<Vec<_>>(),
            );

            let current = order[0];
            let next = next_window(&windows, current).expect("three windows must cycle");

            visited.push(next);

            // Raised to the front, and the outgoing window demoted to the back.
            order.retain(|h| *h != next && *h != current);
            order.insert(0, next);
            order.push(current);
        }

        assert_eq!(visited, vec![2, 3, 1], "the third window was never reached");
    }

    #[test]
    fn single_window_app_has_nothing_to_cycle() {

        let windows = desktop(&[(1, "notepad.exe"), (2, "chrome.exe")]);

        assert_eq!(next_window(&windows, 1), None);
    }

    #[test]
    fn unlisted_focused_window_is_not_cycled() {

        let windows = desktop(&[(1, "chrome.exe"), (2, "chrome.exe")]);

        assert_eq!(next_window(&windows, 99), None);
    }

    // Two elevated apps both fall back to a pid-derived identity; those must
    // not read as one app just because neither path could be opened.
    #[test]
    fn distinct_fallback_identities_do_not_merge() {

        let windows = desktop(&[(1, "pid:1000"), (2, "pid:2000")]);

        assert_eq!(next_window(&windows, 1), None);
    }

    // The mirror of `three_presses_visit_every_window`. Stepping back has to
    // reach every window too, and here raising is the whole move: nothing is
    // demoted, which is what this models.
    #[test]
    fn three_reverse_presses_visit_every_window() {

        let mut order = vec![1isize, 2, 3];
        let mut visited = Vec::new();

        for _ in 0..3 {

            let windows = desktop(
                &order
                    .iter()
                    .map(|handle| (*handle, "code.exe"))
                    .collect::<Vec<_>>(),
            );

            let previous =
                previous_window(&windows, order[0]).expect("three windows must cycle back");

            visited.push(previous);

            order.retain(|h| *h != previous);
            order.insert(0, previous);
        }

        assert_eq!(visited, vec![3, 2, 1], "stepping back skipped a window");
    }

    // Why the two are paired: a reverse press must undo a forward one, both in
    // which window ends up focused and in the Z-order left behind.
    #[test]
    fn reverse_undoes_a_forward_step() {

        let before = desktop(&[(1, "code.exe"), (2, "chrome.exe"), (3, "code.exe")]);

        let next = next_window(&before, 1).expect("two windows to cycle");
        assert_eq!(next, 3);

        // What the forward step leaves: 3 raised, 1 demoted to the back.
        let after = desktop(&[(3, "code.exe"), (2, "chrome.exe"), (1, "code.exe")]);

        assert_eq!(previous_window(&after, next), Some(1));
    }

    #[test]
    fn reverse_steps_over_windows_of_other_apps() {

        let windows = desktop(&[
            (1, "code.exe"),
            (2, "chrome.exe"),
            (3, "explorer.exe"),
            (4, "code.exe"),
        ]);

        assert_eq!(previous_window(&windows, 1), Some(4));
    }

    #[test]
    fn reverse_single_window_app_has_nothing_to_cycle() {

        let windows = desktop(&[(1, "notepad.exe"), (2, "chrome.exe")]);

        assert_eq!(previous_window(&windows, 1), None);
    }

    #[test]
    fn reverse_unlisted_focused_window_is_not_cycled() {

        let windows = desktop(&[(1, "chrome.exe"), (2, "chrome.exe")]);

        assert_eq!(previous_window(&windows, 99), None);
    }
}
