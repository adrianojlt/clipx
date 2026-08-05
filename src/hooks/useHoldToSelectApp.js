import { useCallback, useEffect, useMemo, useRef } from "react";
import { getCursorClientPosition, logError } from "../services/clipboardService";
import { appRowAtPoint } from "../utils/appRowAtPoint";

// Modifier keys whose key-up ends the open-apps chord. The chord always carries
// at least one, so the first of them to come up is the user letting go.
const CHORD_MODIFIERS = new Set(["Control", "Alt", "Shift", "Meta"]);

// How far the mouse must travel while the popup is up before a release counts
// as a deliberate pick. Small enough that any real gesture clears it, large
// enough to absorb the jitter of a hand resting on the mouse.
const MOVE_TOLERANCE_PX = 4;

/**
 * Hold the open-apps shortcut, release it with the cursor over a row, and that
 * app is focused. Releasing anywhere else leaves the popup open.
 *
 * The gesture hangs off the webview key-up, not off the global shortcut's
 * release event: on macOS the plugin reports the chord as released about 100 ms
 * after the press, when the popup takes focus, however long the keys are
 * actually held. The webview, once focused, does see the real key-up.
 *
 * Callers drive it from the popup's lifecycle: `arm` when the apps popup is
 * shown, `disarm` when it is shown in any other mode, `markListReady` once the
 * rows are in the DOM.
 */
export function useHoldToSelectApp(onFocusApp) {

  const armedRef = useRef(false);
  // The list arrives asynchronously, so a fast release can beat the rows into
  // the DOM. Its cursor position waits here until they exist.
  const listReadyRef = useRef(false);
  const pendingPointRef = useRef(null);
  // Where the mouse sat when the popup appeared, and whether it has left there.
  const pointerOriginRef = useRef(null);
  const pointerMovedRef = useRef(false);

  // The row under the cursor is resolved from the DOM rather than from React
  // hover state: if the mouse never moved after the popup appeared, no pointer
  // event ever fired and React knows of no hovered row.
  const selectAppAtPoint = useCallback(({ x, y }) => {
    const row = appRowAtPoint(x, y);
    if (row) onFocusApp(row);
  }, [onFocusApp]);

  const arm = useCallback(() => {
    armedRef.current = true;
    listReadyRef.current = false;
    pendingPointRef.current = null;
    pointerOriginRef.current = null;
    pointerMovedRef.current = false;
  }, []);

  const disarm = useCallback(() => {
    armedRef.current = false;
    listReadyRef.current = false;
    pendingPointRef.current = null;
  }, []);

  const markListReady = useCallback(() => {

    listReadyRef.current = true;

    const pending = pendingPointRef.current;
    pendingPointRef.current = null;

    // one turn later, so the rows are painted before they are hit-tested
    if (pending) setTimeout(() => selectAppAtPoint(pending), 0);
  }, [selectAppAtPoint]);

  useEffect(() => {

    // The first move after the popup appears is the cursor's resting position,
    // reported by the webview as it takes the pointer, not the user moving.
    const onMouseMove = (e) => {

      if (!armedRef.current || pointerMovedRef.current) return;

      const origin = pointerOriginRef.current;

      if (!origin) {
        pointerOriginRef.current = { x: e.clientX, y: e.clientY };
        return;
      }

      if (
        Math.abs(e.clientX - origin.x) > MOVE_TOLERANCE_PX ||
        Math.abs(e.clientY - origin.y) > MOVE_TOLERANCE_PX
      ) {
        pointerMovedRef.current = true;
      }
    };

    const onKeyUp = async (e) => {

      if (!armedRef.current || !CHORD_MODIFIERS.has(e.key)) return;

      armedRef.current = false;

      // A release without any mouse motion is a tap. It must not select: near a
      // screen edge the popup is clamped back inside the monitor, so the cursor
      // lands on an arbitrary row instead of on the search bar.
      if (!pointerMovedRef.current) return;

      try {
        const point = await getCursorClientPosition();

        if (listReadyRef.current) {
          selectAppAtPoint(point);
        } else {
          pendingPointRef.current = point;
        }
      } catch (e) {
        await logError("warn", `Failed to read cursor position: ${e}`);
      }
    };

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("keyup", onKeyUp);

    return () => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("keyup", onKeyUp);
    };
  }, [selectAppAtPoint]);

  return useMemo(() => ({ arm, disarm, markListReady }), [arm, disarm, markListReady]);
}
