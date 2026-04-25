import { useState, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getSetting, setSetting, updateShortcut, applyWindowSize } from "../services/clipboardService";
import "./Settings.css";

function Settings() {
  const [hotkey, setHotkey] = useState("");
  const [recording, setRecording] = useState(null);
  const [tabShortcutPinned, setTabShortcutPinned] = useState("Command+1");
  const [tabShortcutHistory, setTabShortcutHistory] = useState("Command+2");
  const [historyLimit, setHistoryLimit] = useState(20);
  const [windowWidth, setWindowWidth] = useState(400);
  const [windowHeight, setWindowHeight] = useState(600);
  const [error, setError] = useState("");

  useEffect(() => {
    const onKey = async (e) => {
      if (e.key === "Escape") await getCurrentWindow().hide();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    if (!recording) return;
    const onKeyDown = (e) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        setRecording(null);
        return;
      }
      if (["Meta", "Control", "Alt", "Shift"].includes(e.key)) return;
      const parts = [];
      if (e.metaKey) parts.push("Command");
      if (e.ctrlKey) parts.push("Ctrl");
      if (e.altKey) parts.push("Option");
      if (e.shiftKey) parts.push("Shift");
      const key = e.key === " " ? "Space" : e.key.length === 1 ? e.key.toUpperCase() : e.key;
      parts.push(key);
      const shortcut = parts.join("+");
      if (recording === "hotkey") setHotkey(shortcut);
      else if (recording === "tab_pinned") setTabShortcutPinned(shortcut);
      else if (recording === "tab_history") setTabShortcutHistory(shortcut);
      setRecording(null);
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [recording]);

  useEffect(() => {
    const load = async () => {
      try {
        const value = await getSetting("hotkey");
        setHotkey(value);
      } catch {
        setHotkey("Option+Space");
      }
      try {
        const value = await getSetting("tab_shortcut_pinned");
        setTabShortcutPinned(value);
      } catch {}
      try {
        const value = await getSetting("tab_shortcut_history");
        setTabShortcutHistory(value);
      } catch {}
      try {
        const value = await getSetting("history_limit");
        setHistoryLimit(Number(value));
      } catch {
        setHistoryLimit(20);
      }
      try {
        const w = await getSetting("window_width");
        setWindowWidth(Number(w) || 400);
      } catch {
        setWindowWidth(400);
      }
      try {
        const h = await getSetting("window_height");
        setWindowHeight(Number(h) || 600);
      } catch {
        setWindowHeight(600);
      }
    };
    load();
  }, []);

  const handleSave = async () => {
    setError("");
    try {
      await updateShortcut(hotkey);
      await setSetting("tab_shortcut_pinned", tabShortcutPinned);
      await setSetting("tab_shortcut_history", tabShortcutHistory);
      await setSetting("history_limit", String(historyLimit));
      await setSetting("window_width", String(windowWidth));
      await setSetting("window_height", String(windowHeight));
      await applyWindowSize();
      await getCurrentWindow().hide();
    } catch (e) {
      setError(`Failed to update settings: ${e}`);
    }
  };

  return (
    <div className="settings">
      <h2>Settings</h2>
      <div className="field">
        <label htmlFor="hotkey">Global Hotkey</label>
        <div className="hotkey-row">
          <input
            id="hotkey"
            type="text"
            readOnly
            value={recording === "hotkey" ? "" : hotkey}
            placeholder={recording === "hotkey" ? "Press shortcut..." : "Click Record"}
            className={recording === "hotkey" ? "recording" : ""}
          />
          <button
            type="button"
            className={`record-btn${recording === "hotkey" ? " active" : ""}`}
            onClick={() => setRecording(r => r === "hotkey" ? null : "hotkey")}
          >
            {recording === "hotkey" ? "Cancel" : "Record"}
          </button>
        </div>
        <p className="hint">Click Record then press your desired key combination</p>
      </div>
      <div className="field">
        <label htmlFor="tab-pinned">Switch to Pinned Tab</label>
        <div className="hotkey-row">
          <input
            id="tab-pinned"
            type="text"
            readOnly
            value={recording === "tab_pinned" ? "" : tabShortcutPinned}
            placeholder={recording === "tab_pinned" ? "Press shortcut..." : "Click Record"}
            className={recording === "tab_pinned" ? "recording" : ""}
          />
          <button
            type="button"
            className={`record-btn${recording === "tab_pinned" ? " active" : ""}`}
            onClick={() => setRecording(r => r === "tab_pinned" ? null : "tab_pinned")}
          >
            {recording === "tab_pinned" ? "Cancel" : "Record"}
          </button>
        </div>
        <p className="hint">In-app shortcut to switch to the Pinned tab</p>
      </div>
      <div className="field">
        <label htmlFor="tab-history">Switch to History Tab</label>
        <div className="hotkey-row">
          <input
            id="tab-history"
            type="text"
            readOnly
            value={recording === "tab_history" ? "" : tabShortcutHistory}
            placeholder={recording === "tab_history" ? "Press shortcut..." : "Click Record"}
            className={recording === "tab_history" ? "recording" : ""}
          />
          <button
            type="button"
            className={`record-btn${recording === "tab_history" ? " active" : ""}`}
            onClick={() => setRecording(r => r === "tab_history" ? null : "tab_history")}
          >
            {recording === "tab_history" ? "Cancel" : "Record"}
          </button>
        </div>
        <p className="hint">In-app shortcut to switch to the History tab</p>
      </div>
      <div className="field">
        <label htmlFor="history-limit">History Limit</label>
        <input
          id="history-limit"
          type="number"
          min={1}
          max={50}
          value={historyLimit}
          onChange={e => setHistoryLimit(Math.min(50, Math.max(1, Number(e.target.value))))}
        />
        <p className="hint">Number of clipboard entries to keep (max 50)</p>
      </div>
      <div className="field">
        <label htmlFor="window-width">Window Width</label>
        <input
          id="window-width"
          type="number"
          min={300}
          max={800}
          value={windowWidth}
          onChange={e => setWindowWidth(Math.min(800, Math.max(300, Number(e.target.value))))}
        />
        <p className="hint">Popup window width in pixels (300-800)</p>
      </div>
      <div className="field">
        <label htmlFor="window-height">Window Height</label>
        <input
          id="window-height"
          type="number"
          min={400}
          max={900}
          value={windowHeight}
          onChange={e => setWindowHeight(Math.min(900, Math.max(400, Number(e.target.value))))}
        />
        <p className="hint">Popup window height in pixels (400-900)</p>
      </div>
      {error && <p className="error">{error}</p>}
      <div className="actions">
        <button onClick={handleSave}>Save</button>
        <button className="secondary" onClick={async () => await getCurrentWindow().hide()}>
          Cancel
        </button>
      </div>
    </div>
  );
}

export default Settings;
