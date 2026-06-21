import { useState, useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  getSetting,
  setSetting,
  updateShortcut,
  updateOpenAppsShortcut,
  applyWindowSize,
  logError,
} from "../services/clipboardService";
import { IS_MAC } from "../utils/shortcuts";
import "./Settings.css";

const TAB_MOD = IS_MAC ? "Command" : "Alt";

const SYM = {
  Command: "⌘", Ctrl: "⌃", Control: "⌃", Option: "⌥", Alt: "⌥",
  Shift: "⇧", Space: "␣", Enter: "⏎", Escape: "⎋", Tab: "⇥",
  ArrowUp: "↑", ArrowDown: "↓", ArrowLeft: "←", ArrowRight: "→",
};

const KeyboardIcon = () => (
  <svg viewBox="0 0 16 16" width="14" height="14" fill="none">
    <rect x="1.5" y="3.5" width="13" height="9" rx="1.5" stroke="currentColor" strokeWidth="1.2" />
    <path d="M4 6h.5M7 6h.5M10 6h.5M12 6h.5M4 8.5h.5M7 8.5h.5M10 8.5h.5M12 8.5h.5M5 11h6" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
  </svg>
);

const LayoutIcon = () => (
  <svg viewBox="0 0 16 16" width="14" height="14" fill="none">
    <rect x="2" y="2.5" width="12" height="11" rx="1.5" stroke="currentColor" strokeWidth="1.2" />
    <path d="M2 6h12M6 6v7.5" stroke="currentColor" strokeWidth="1.2" />
  </svg>
);

const SlidersIcon = () => (
  <svg viewBox="0 0 16 16" width="14" height="14" fill="none">
    <path d="M2 4.5h8M12 4.5h2M2 11.5h2M6 11.5h8" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    <circle cx="11" cy="4.5" r="1.5" fill="currentColor" />
    <circle cx="5" cy="11.5" r="1.5" fill="currentColor" />
  </svg>
);

function KeyChips({ value }) {
  if (!value) return <span className="kbd-placeholder">Not set</span>;
  const parts = value.split("+");
  return (
    <div className="kbd-row">
      {parts.map((p, i) => (
        <span key={i} style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
          {i > 0 && <span className="kbd-plus">+</span>}
          <span className="kbd">
            {SYM[p] && <span className="kbd-sym">{SYM[p]}</span>}
            <span className="kbd-name">{p}</span>
          </span>
        </span>
      ))}
    </div>
  );
}

function HotkeyField({ label, hint, value, onChange }) {

  const [recording, setRecording] = useState(false);
  const [draft, setDraft] = useState([]);
  const wrapRef = useRef(null);
  const onChangeRef = useRef(onChange);

  useEffect(() => { onChangeRef.current = onChange; });

  useEffect(() => {

    if (!recording) return;

    const heldKeys = new Set();

    let mainKey = null;
    let mainModifiers = new Set();

    const formatKey = (k) => (k === " " ? "Space" : k.length === 1 ? k.toUpperCase() : k);

    const toParts = () => {

      const parts = [];

      if (mainModifiers.has("Meta")) parts.push("Command");
      if (mainModifiers.has("Control")) parts.push("Ctrl");
      if (mainModifiers.has("Alt")) parts.push("Option");
      if (mainModifiers.has("Shift")) parts.push("Shift");

      if (mainKey) parts.push(formatKey(mainKey));

      return parts;
    };

    const down = (e) => {

      e.preventDefault();
      e.stopPropagation();

      if (e.key === "Escape" && !e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey) {
        setRecording(false);
        setDraft([]);
        return;
      }

      heldKeys.add(e.key);

      if (["Meta", "Control", "Alt", "Shift"].includes(e.key)) return;

      mainKey = e.key;
      mainModifiers = new Set();

      if (e.metaKey) mainModifiers.add("Meta");
      if (e.ctrlKey) mainModifiers.add("Control");
      if (e.altKey) mainModifiers.add("Alt");
      if (e.shiftKey) mainModifiers.add("Shift");

      setDraft(toParts());
    };

    const up = (e) => {

      heldKeys.delete(e.key);

      if (heldKeys.size === 0 && mainKey) {
        onChangeRef.current(toParts().join("+"));
        setRecording(false);
        setDraft([]);
      }
    };

    window.addEventListener("keydown", down, true);
    window.addEventListener("keyup", up, true);

    return () => {
      window.removeEventListener("keydown", down, true);
      window.removeEventListener("keyup", up, true);
    };
  }, [recording]);

  useEffect(() => {
    if (!recording) return;
    const onClick = (e) => {
      if (wrapRef.current && !wrapRef.current.contains(e.target)) {
        setRecording(false);
        setDraft([]);
      }
    };
    window.addEventListener("mousedown", onClick);
    return () => window.removeEventListener("mousedown", onClick);
  }, [recording]);

  const display = recording && draft.length ? draft.join("+") : value;

  return (
    <div className="field" ref={wrapRef}>
      <div className="field-label">{label}</div>
      <div className={`hotkey${recording ? " is-recording" : ""}`}>
        <div className="hotkey-display">
          {recording && !draft.length ? (
            <span className="hotkey-prompt">
              <span className="rec-dot" /> Press keys...
            </span>
          ) : (
            <KeyChips value={display} />
          )}
        </div>
        <button
          type="button"
          className={`btn-record${recording ? " is-active" : ""}`}
          onClick={() => { setRecording((r) => !r); setDraft([]); }}
        >
          {recording ? "Cancel" : "Record"}
        </button>
      </div>
      {hint && <div className="field-hint">{hint}</div>}
    </div>
  );
}

function NumberField({ label, hint, value, onChange, min, max }) {
  const [draft, setDraft] = useState(String(value));

  useEffect(() => {
    setDraft(String(value));
  }, [value]);

  const commit = () => {
    const n = Number(draft);
    if (draft.trim() === "" || Number.isNaN(n)) {
      setDraft(String(value));
      return;
    }
    const clamped = Math.min(max ?? Infinity, Math.max(min ?? -Infinity, n));
    setDraft(String(clamped));
    if (clamped !== value) onChange(clamped);
  };

  return (
    <div className="field">
      <div className="field-label">{label}</div>
      <div className="num">
        <input
          type="text"
          inputMode="numeric"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => { if (e.key === "Enter") e.target.blur(); }}
        />
      </div>
      {hint && <div className="field-hint">{hint}</div>}
    </div>
  );
}

const TABS = [
  { id: "hotkeys", label: "Hotkeys", Icon: KeyboardIcon },
  { id: "ui", label: "UI", Icon: LayoutIcon },
  { id: "others", label: "Others", Icon: SlidersIcon },
];

function TabStrip({ active, onChange }) {
  const stripRef = useRef(null);
  const btnRefs = useRef({});
  const [indicator, setIndicator] = useState({ left: 0, width: 0 });

  useEffect(() => {
    const el = btnRefs.current[active];
    const wrap = stripRef.current;
    if (!el || !wrap) return;
    const er = el.getBoundingClientRect();
    const wr = wrap.getBoundingClientRect();
    setIndicator({ left: er.left - wr.left, width: er.width });
  }, [active]);

  return (
    <div className="tabs" ref={stripRef}>
      <div
        className="tab-indicator"
        style={{ transform: `translateX(${indicator.left}px)`, width: indicator.width }}
      />
      {TABS.map(({ id, label, Icon }) => (
        <button
          key={id}
          ref={(el) => (btnRefs.current[id] = el)}
          className={`tab${active === id ? " is-active" : ""}`}
          onClick={() => onChange(id)}
          type="button"
        >
          <Icon />
          <span>{label}</span>
        </button>
      ))}
    </div>
  );
}

function HotkeysPanel({ s, set }) {
  return (
    <>
      <div className="section-header">
        <h3>Global Shortcut</h3>
        <p>System-wide hotkey to open ClipX from anywhere.</p>
      </div>
      <HotkeyField
        label="Open ClipX"
        hint="Click Record then press your desired key combination."
        value={s.hotkey}
        onChange={(v) => set("hotkey", v)}
      />
      <HotkeyField
        label="Open List of Apps"
        hint="System-wide hotkey to open the list of running apps."
        value={s.openAppsHotkey}
        onChange={(v) => set("openAppsHotkey", v)}
      />
      <div className="section-header">
        <h3>In-app Shortcuts</h3>
        <p>Used when ClipX is focused.</p>
      </div>
      <HotkeyField label="Switch to Pinned" hint="Jump to the Pinned tab." value={s.tabShortcutPinned} onChange={(v) => set("tabShortcutPinned", v)} />
      <HotkeyField label="Switch to History" hint="Jump to the History tab." value={s.tabShortcutHistory} onChange={(v) => set("tabShortcutHistory", v)} />
      <HotkeyField label="Switch to Sessions" hint="Jump to the Sessions tab." value={s.tabShortcutSessions} onChange={(v) => set("tabShortcutSessions", v)} />
      <HotkeyField label="Focus Search Box" hint="Focus search of the active tab." value={s.tabShortcutFind} onChange={(v) => set("tabShortcutFind", v)} />
    </>
  );
}

function UIPanel({ s, set }) {
  return (
    <>
      <div className="section-header">
        <h3>Popup Window</h3>
        <p>Size of the ClipX popup when it appears.</p>
      </div>
      <div className="grid-2">
        <NumberField label="Width" hint="Pixels (300-800)." min={300} max={800} value={s.windowWidth} onChange={(v) => set("windowWidth", v)} />
        <NumberField label="Height" hint="Pixels (400-900)." min={400} max={900} value={s.windowHeight} onChange={(v) => set("windowHeight", v)} />
      </div>
      <div className="preview">
        <div className="preview-label">
          <span>Preview</span>
          <span className="preview-dim">{s.windowWidth} x {s.windowHeight}</span>
        </div>
        <div className="preview-stage">
          <div
            className="preview-window"
            style={{
              width: `${(s.windowWidth / 800) * 100}%`,
              height: `${(s.windowHeight / 900) * 100}%`,
            }}
          >
            <div className="pw-titlebar">
              <span className="pw-dot pw-dot-r" />
              <span className="pw-dot pw-dot-y" />
              <span className="pw-dot pw-dot-g" />
              <span className="pw-title">ClipX</span>
            </div>
            <div className="pw-tabs">
              <span className="pw-tab is-active">Pinned</span>
              <span className="pw-tab">History</span>
              <span className="pw-tab">Sessions</span>
            </div>
            <div className="pw-list">
              <div className="pw-row" /><div className="pw-row" /><div className="pw-row" />
            </div>
          </div>
        </div>
      </div>
    </>
  );
}

function OthersPanel({ s, set }) {
  return (
    <>
      <div className="section-header">
        <h3>Clipboard History</h3>
        <p>Storage behavior for captured entries.</p>
      </div>
      <NumberField
        label="History Limit"
        hint="Number of clipboard entries to keep (max 500)."
        min={1}
        max={500}
        value={s.historyLimit}
        onChange={(v) => set("historyLimit", v)}
      />
      <div className="meter">
        <div className="meter-bar">
          <div className="meter-fill" style={{ width: `${(s.historyLimit / 500) * 100}%` }} />
        </div>
        <div className="meter-labels"><span>1</span><span>50</span></div>
      </div>
    </>
  );
}

function Settings() {
  const [activeTab, setActiveTab] = useState("hotkeys");
  const [dirty, setDirty] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState("");

  const [s, setS] = useState({
    hotkey: "",
    openAppsHotkey: "Control+Option+Esc",
    tabShortcutPinned: `${TAB_MOD}+1`,
    tabShortcutHistory: `${TAB_MOD}+2`,
    tabShortcutSessions: `${TAB_MOD}+3`,
    tabShortcutFind: `${TAB_MOD}+F`,
    historyLimit: 20,
    windowWidth: 600,
    windowHeight: 700,
  });

  const set = (k, v) => {
    setS((p) => ({ ...p, [k]: v }));
    setDirty(true);
    setSaved(false);
  };

  useEffect(() => {
    const onKey = async (e) => {
      if (e.key === "Escape") await getCurrentWindow().hide();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    const load = async () => {
      const safeGet = async (key, fallback, transform = (v) => v) => {
        try {
          return transform(await getSetting(key));
        } catch (e) {
          await logError("warn", `Failed to load setting ${key}: ${e}`);
          return fallback;
        }
      };
      const [hotkey, openApps, pinned, history, sessions, find, limit, width, height] = await Promise.all([
        safeGet("hotkey", "Option+Space"),
        safeGet("open_apps_hotkey", "Control+Option+Esc"),
        safeGet("tab_shortcut_pinned", `${TAB_MOD}+1`),
        safeGet("tab_shortcut_history", `${TAB_MOD}+2`),
        safeGet("tab_shortcut_sessions", `${TAB_MOD}+3`),
        safeGet("tab_shortcut_find", `${TAB_MOD}+F`),
        safeGet("history_limit", 20, Number),
        safeGet("window_width", 600, (v) => Number(v) || 600),
        safeGet("window_height", 700, (v) => Number(v) || 700),
      ]);
      setS({ hotkey, openAppsHotkey: openApps, tabShortcutPinned: pinned, tabShortcutHistory: history, tabShortcutSessions: sessions, tabShortcutFind: find, historyLimit: limit, windowWidth: width, windowHeight: height });
    };
    load();
  }, []);

  const handleSave = async () => {

    setError("");

    const errors = [];

    const attempt = async (fn) => { try { await fn(); } catch (e) { errors.push(String(e)); } };

    await attempt(() => updateShortcut(s.hotkey));
    await attempt(() => updateOpenAppsShortcut(s.openAppsHotkey));

    await attempt(() => setSetting("tab_shortcut_pinned", s.tabShortcutPinned));
    await attempt(() => setSetting("tab_shortcut_history", s.tabShortcutHistory));
    await attempt(() => setSetting("tab_shortcut_sessions", s.tabShortcutSessions));
    await attempt(() => setSetting("tab_shortcut_find", s.tabShortcutFind));
    await attempt(() => setSetting("history_limit", String(s.historyLimit)));
    await attempt(() => setSetting("window_width", String(s.windowWidth)));
    await attempt(() => setSetting("window_height", String(s.windowHeight)));

    await attempt(() => applyWindowSize());

    if (errors.length > 0) {
      const msg = errors.join("; ");
      setError(`Failed to save settings: ${msg}`);
      await logError("error", `Failed to save settings: ${msg}`);
    } else {
      setDirty(false);
      setSaved(true);
      setTimeout(() => setSaved(false), 1800);
      await getCurrentWindow().hide();
    }
  };

  return (
    <div className="settings">
      <TabStrip active={activeTab} onChange={setActiveTab} />
      <div className="settings-content">
        <div className="content-scroll" key={activeTab}>
          {activeTab === "hotkeys" && <HotkeysPanel s={s} set={set} />}
          {activeTab === "ui" && <UIPanel s={s} set={set} />}
          {activeTab === "others" && <OthersPanel s={s} set={set} />}
        </div>
      </div>
      {error && <p className="error">{error}</p>}
      <div className="settings-footer">
        <div className="footer-status">
          {saved && (
            <span className="status status-ok">
              <svg width="12" height="12" viewBox="0 0 12 12"><path d="M2.5 6.5l2.5 2.5 4.5-5" stroke="currentColor" strokeWidth="1.6" fill="none" strokeLinecap="round" strokeLinejoin="round" /></svg>
              {" "}Saved
            </span>
          )}
          {dirty && !saved && <span className="status status-dirty"><span className="dot" /> Unsaved changes</span>}
          {!dirty && !saved && <span className="status status-idle">All changes saved</span>}
        </div>
        <div className="footer-actions">
          <button className="btn btn-ghost" type="button" onClick={async () => await getCurrentWindow().hide()}>Cancel</button>
          <button
            className={`btn btn-primary${dirty ? "" : " is-disabled"}`}
            type="button"
            onClick={handleSave}
            disabled={!dirty}
          >
            Save Changes
          </button>
        </div>
      </div>
    </div>
  );
}

export default Settings;
