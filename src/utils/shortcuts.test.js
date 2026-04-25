import { describe, it, expect } from "vitest";
import { parseShortcut, matchesShortcut } from "./shortcuts";

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

  it("treats Super, Command, and Meta as meta", () => {
    expect(parseShortcut("Super+K").meta).toBe(true);
    expect(parseShortcut("Command+K").meta).toBe(true);
    expect(parseShortcut("Meta+K").meta).toBe(true);
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
});
