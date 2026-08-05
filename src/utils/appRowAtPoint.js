// Resolve the app row under a client-space point as { id, app }, or null when
// the point hits the search bar, an empty list, or anything else that is not a
// row. `app` is the process name, which focusing needs as the usage key.
export function appRowAtPoint(x, y) {
  const row = document.elementFromPoint(x, y)?.closest("[data-app-id]");
  return row ? { id: row.dataset.appId, app: row.dataset.app } : null;
}
