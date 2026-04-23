import { useState, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import "./Settings.css";

function Settings() {
  const [hotkey, setHotkey] = useState("");
  const [historyLimit, setHistoryLimit] = useState(20);
  const [error, setError] = useState("");

  useEffect(() => {
    const onKey = async (e) => {
      if (e.key === "Escape") await getCurrentWindow().hide();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    const load = async () => {
      try {
        const value = await invoke("get_setting", { key: "hotkey" });
        setHotkey(value);
      } catch {
        setHotkey("Option+Command+1");
      }
      try {
        const value = await invoke("get_setting", { key: "history_limit" });
        setHistoryLimit(Number(value));
      } catch {
        setHistoryLimit(20);
      }
    };
    load();
  }, []);

  const handleSave = async () => {
    setError("");
    try {
      await invoke("update_shortcut", { shortcut: hotkey });
      await invoke("set_setting", { key: "history_limit", value: String(historyLimit) });
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
        <input
          id="hotkey"
          type="text"
          value={hotkey}
          onChange={(e) => setHotkey(e.target.value)}
          placeholder="e.g. Option+Command+1"
        />
        <p className="hint">
          Examples: Option+Command+1, Cmd+Shift+A, Ctrl+Alt+T
        </p>
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
