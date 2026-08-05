import { describe, it, expect } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import AppsTab from "./AppsTab";

const row = (app, title) => ({ app, name: `${app} - ${title}`, id: `${app}${title}` });

function render(filteredApps, appIcons = {}) {
  const html = renderToStaticMarkup(
    <AppsTab
      filteredApps={filteredApps}
      appIcons={appIcons}
      appsSearch=""
      setAppsSearch={() => {}}
      appsSearchRef={{ current: null }}
      onSelect={() => {}}
    />
  );

  const doc = new DOMParser().parseFromString(`<div>${html}</div>`, "text/html");
  return [...doc.querySelectorAll(".app-item")];
}

const ICON = "data:image/png;base64,AAA";

describe("AppsTab icon slot", () => {
  it("gives every window of an app its icon", () => {
    const items = render(
      [row("Chrome", "Inbox"), row("Chrome", "Docs"), row("Finder", "Downloads")],
      { Chrome: ICON, Finder: ICON }
    );

    const slots = items.map((el) => el.querySelector(".app-item-icon"));

    expect(slots.map((s) => s.tagName)).toEqual(["IMG", "IMG", "IMG"]);
    expect(slots.map((s) => s.getAttribute("src"))).toEqual([ICON, ICON, ICON]);
  });

  it("keeps the slot as the second child, between the index and the name", () => {
    const [item] = render([row("Chrome", "Inbox")], { Chrome: ICON });

    expect([...item.children].map((c) => c.className)).toEqual([
      "app-index",
      "app-item-icon",
      "session-name",
    ]);
  });

  it("renders an empty slot when the app has no icon in the map", () => {
    // Windows Access-Denied path, or a macOS app with no executableURL.
    const items = render([row("Ghost", "Window"), row("Chrome", "Inbox")], { Chrome: ICON });

    const slots = items.map((el) => el.querySelector(".app-item-icon"));

    expect(slots.map((s) => s.tagName)).toEqual(["SPAN", "IMG"]);
    expect(slots[0].innerHTML).toBe("");
  });

  it("gives every row a slot so the name column stays aligned", () => {
    const items = render([row("Ghost", "Window"), row("Chrome", "Inbox")], { Chrome: ICON });

    expect(items.every((el) => el.querySelector(".app-item-icon"))).toBe(true);
  });

  it("renders no rows and no icons for an empty list", () => {
    expect(render([])).toHaveLength(0);
  });

  it("leaves the icon out of the accessibility tree", () => {
    const [item] = render([row("Chrome", "Inbox")], { Chrome: ICON });

    expect(item.querySelector("img").getAttribute("alt")).toBe("");
  });
});
