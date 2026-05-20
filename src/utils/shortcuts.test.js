import { describe, it, expect, vi, afterEach } from "vitest";
import { parseShortcut, matchesShortcut } from "./shortcuts";

// Load a fresh copy of the module with a stubbed navigator so the module-level
// IS_MAC check is recomputed for the desired platform.
async function loadForPlatform(platform) {
  vi.resetModules();
  vi.stubGlobal("navigator", { platform, userAgent: platform });
  return import("./shortcuts.js");
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.resetModules();
});

describe("parseShortcut", () => {
  it("parses a simple shortcut", () => {
    const result = parseShortcut("Alt+H");
    expect(result.alt).toBe(true);
    expect(result.key).toBe("H");
    expect(result.meta).toBe(false);
    expect(result.ctrl).toBe(false);
    expect(result.shift).toBe(false);
  });

  it("parses a multi-modifier shortcut", () => {
    const result = parseShortcut("Ctrl+Shift+P");
    expect(result.ctrl).toBe(true);
    expect(result.shift).toBe(true);
    expect(result.key).toBe("P");
  });

  it("normalises the Space key to a literal space", () => {
    const result = parseShortcut("Alt+Space");
    expect(result.alt).toBe(true);
    expect(result.key).toBe(" ");
  });

  it("treats Option as alt", () => {
    const result = parseShortcut("Option+Space");
    expect(result.alt).toBe(true);
    expect(result.key).toBe(" ");
  });

  it("treats Super, Command, and Meta as meta on macOS", async () => {
    const { parseShortcut: parse } = await loadForPlatform("MacIntel");
    expect(parse("Super+K").meta).toBe(true);
    expect(parse("Command+K").meta).toBe(true);
    expect(parse("Meta+K").meta).toBe(true);
    expect(parse("Command+K").ctrl).toBe(false);
  });

  it("folds Command/Meta/Super to Ctrl on Windows", async () => {
    const { parseShortcut: parse } = await loadForPlatform("Win32");
    expect(parse("Command+1").meta).toBe(false);
    expect(parse("Command+1").ctrl).toBe(true);
    expect(parse("Meta+K").ctrl).toBe(true);
    expect(parse("Super+K").ctrl).toBe(true);
  });
});

describe("matchesShortcut", () => {
  it("returns true when event matches shortcut", () => {
    const event = { metaKey: false, ctrlKey: false, altKey: true, shiftKey: false, key: "H" };
    expect(matchesShortcut(event, "Alt+H")).toBe(true);
  });

  it("returns false when modifier does not match", () => {
    const event = { metaKey: false, ctrlKey: true, altKey: false, shiftKey: false, key: "H" };
    expect(matchesShortcut(event, "Alt+H")).toBe(false);
  });

  it("matches a Space shortcut against a real space key", () => {
    const event = { metaKey: false, ctrlKey: false, altKey: true, shiftKey: false, key: " " };
    expect(matchesShortcut(event, "Alt+Space")).toBe(true);
  });

  it("matches a default Command+1 against Ctrl+1 on Windows", async () => {
    const { matchesShortcut: matches } = await loadForPlatform("Win32");
    const ctrlEvent = { metaKey: false, ctrlKey: true, altKey: false, shiftKey: false, key: "1" };
    expect(matches(ctrlEvent, "Command+1")).toBe(true);
    const winEvent = { metaKey: true, ctrlKey: false, altKey: false, shiftKey: false, key: "1" };
    expect(matches(winEvent, "Command+1")).toBe(false);
  });

  it("matches a default Command+1 against Cmd+1 on macOS", async () => {
    const { matchesShortcut: matches } = await loadForPlatform("MacIntel");
    const cmdEvent = { metaKey: true, ctrlKey: false, altKey: false, shiftKey: false, key: "1" };
    expect(matches(cmdEvent, "Command+1")).toBe(true);
  });
});
