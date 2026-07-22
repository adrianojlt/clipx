import { getSetting } from "./services/clipboardService";

// Resolve a stored theme value to a concrete appearance. `auto` follows the OS
// via matchMedia; anything unknown/missing/corrupt falls back to dark.
export function resolveTheme(value) {
  if (value === "light") return "light";
  if (value === "auto") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return "dark";
}

// Read the saved theme and set data-theme on the document root. Awaited before
// the first render in each window's bootstrap to minimize flash-of-wrong-theme.
export async function applyTheme() {
  let value = "dark";
  try {
    value = await getSetting("theme");
  } catch {
    value = "dark";
  }
  document.documentElement.setAttribute("data-theme", resolveTheme(value));
}
