import { useState, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import "./Settings.css";

function Settings() {
  const [hotkey, setHotkey] = useState("");
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
    };
    load();
  }, []);

  const handleSave = async () => {
    setError("");
    try {
      await invoke("update_shortcut", { shortcut: hotkey });
      await getCurrentWindow().hide();
    } catch (e) {
      setError(`Failed to update shortcut: ${e}`);
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
