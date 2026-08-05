// Resolve the app row under a client-space point, or null when the point hits
// the search bar, an empty list, or anything else that is not a row.
export function appIdAtPoint(x, y) {
  return document.elementFromPoint(x, y)?.closest("[data-app-id]")?.dataset.appId ?? null;
}
