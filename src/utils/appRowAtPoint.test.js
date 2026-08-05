import { describe, it, expect, afterEach, vi } from "vitest";
import { appRowAtPoint } from "./appRowAtPoint";

// jsdom has no layout and no elementFromPoint, so it is stubbed with the node a
// real browser would have returned for that coordinate.
const hits = (html, selector) => {
  document.body.innerHTML = html;
  const el = selector ? document.querySelector(selector) : null;
  document.elementFromPoint = vi.fn(() => el);
};

afterEach(() => {
  delete document.elementFromPoint;
  document.body.innerHTML = "";
});

describe("appRowAtPoint", () => {
  it("returns the id and app of the row under the point", () => {
    hits(`<div class="app-item" data-app-id="Chrome-1" data-app="Chrome"></div>`, ".app-item");

    expect(appRowAtPoint(10, 200)).toEqual({ id: "Chrome-1", app: "Chrome" });
  });

  it("returns the row when the point lands on a child of the row", () => {
    hits(
      `<div class="app-item" data-app-id="Chrome-1" data-app="Chrome"><span class="session-name">Inbox</span></div>`,
      ".session-name"
    );

    expect(appRowAtPoint(10, 200)).toEqual({ id: "Chrome-1", app: "Chrome" });
  });

  it("returns null when the point lands on the search bar", () => {
    hits(`<div class="search-bar"><input class="search-input" /></div>`, ".search-input");

    expect(appRowAtPoint(10, 5)).toBeNull();
  });

  it("returns null when the list is empty", () => {
    hits(`<div class="list"><div class="empty">No apps</div></div>`, ".empty");

    expect(appRowAtPoint(10, 200)).toBeNull();
  });

  it("returns null when the point is outside the window", () => {
    hits(`<div class="app-item" data-app-id="Chrome-1" data-app="Chrome"></div>`, null);

    expect(appRowAtPoint(-40, 900)).toBeNull();
  });
});
