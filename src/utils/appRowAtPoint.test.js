import { describe, it, expect, afterEach, vi } from "vitest";
import { appIdAtPoint } from "./appRowAtPoint";

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

describe("appIdAtPoint", () => {
  it("returns the id of the row under the point", () => {
    hits(`<div class="app-item" data-app-id="Chrome-1"></div>`, ".app-item");

    expect(appIdAtPoint(10, 200)).toBe("Chrome-1");
  });

  it("returns the row id when the point lands on a child of the row", () => {
    hits(
      `<div class="app-item" data-app-id="Chrome-1"><span class="session-name">Inbox</span></div>`,
      ".session-name"
    );

    expect(appIdAtPoint(10, 200)).toBe("Chrome-1");
  });

  it("returns null when the point lands on the search bar", () => {
    hits(`<div class="search-bar"><input class="search-input" /></div>`, ".search-input");

    expect(appIdAtPoint(10, 5)).toBeNull();
  });

  it("returns null when the list is empty", () => {
    hits(`<div class="list"><div class="empty">No apps</div></div>`, ".empty");

    expect(appIdAtPoint(10, 200)).toBeNull();
  });

  it("returns null when the point is outside the window", () => {
    hits(`<div class="app-item" data-app-id="Chrome-1"></div>`, null);

    expect(appIdAtPoint(-40, 900)).toBeNull();
  });
});
