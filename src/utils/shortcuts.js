export const IS_MAC = typeof navigator !== "undefined" && /mac/i.test(navigator.platform || navigator.userAgent || "");

export function parseShortcut(shortcut) {

  const parts = shortcut.split("+").map((p) => p.trim());
  const key = parts[parts.length - 1];
  const modifiers = parts.slice(0, -1);

  let meta = modifiers.includes("Command") || modifiers.includes("Super") || modifiers.includes("Meta");
  let ctrl = modifiers.includes("Ctrl") || modifiers.includes("Control");

  // On Windows/Linux the primary accelerator is Ctrl, not the Super/Win key.
  // Treat a stored Command/Meta/Super as Ctrl so cross-platform defaults work.
  if (!IS_MAC && meta) {
    ctrl = true;
    meta = false;
  }

  return {
    key: key === "Space" ? " " : key.length === 1 ? key.toUpperCase() : key,
    meta,
    ctrl,
    alt: modifiers.includes("Option") || modifiers.includes("Alt"),
    shift: modifiers.includes("Shift"),
  };
}

export function matchesShortcut(e, shortcut) {
  const s = parseShortcut(shortcut);
  return (
    e.key.toLowerCase() === s.key.toLowerCase() &&
    e.metaKey === s.meta &&
    e.ctrlKey === s.ctrl &&
    e.altKey === s.alt &&
    e.shiftKey === s.shift
  );
}
