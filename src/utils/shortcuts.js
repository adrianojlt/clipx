export function parseShortcut(shortcut) {
  const parts = shortcut.split("+").map((p) => p.trim());
  const key = parts[parts.length - 1];
  const modifiers = parts.slice(0, -1);
  return {
    key: key === "Space" ? " " : key.length === 1 ? key.toUpperCase() : key,
    meta:
      modifiers.includes("Command") || modifiers.includes("Super") || modifiers.includes("Meta"),
    ctrl: modifiers.includes("Ctrl") || modifiers.includes("Control"),
    alt: modifiers.includes("Option") || modifiers.includes("Alt"),
    shift: modifiers.includes("Shift"),
  };
}

export function matchesShortcut(e, shortcut) {
  const s = parseShortcut(shortcut);
  return (
    e.key === s.key &&
    e.metaKey === s.meta &&
    e.ctrlKey === s.ctrl &&
    e.altKey === s.alt &&
    e.shiftKey === s.shift
  );
}
