import { useRef, useState, useMemo, useCallback, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import {
  getHistory,
  getPinned,
  getGlobalPinned,
  getSessions,
  getClipboard,
  getSetting,
  activateSession,
  listOpenApps,
  focusApp,
  logError,
  setAlwaysOnTop,
  setSoftPin,
} from "./services/clipboardService";

import { useAppEvents } from "./hooks/useAppEvents";
import { IS_MAC } from "./utils/shortcuts";

const TAB_MOD = IS_MAC ? "Command" : "Alt";

const PIN_NEXT = { none: "always-on-top", "always-on-top": "soft", soft: "none" };
const PIN_LABELS = { none: "Unpinned", "always-on-top": "Always on top", soft: "Soft pin" };
const PIN_TOOLTIPS = {
  none: "Pin window",
  "always-on-top": "Always on top - click to soft pin",
  soft: "Soft pin - click to unpin",
};

import AppsTab from "./components/AppsTab";
import PinnedTab from "./components/PinnedTab";
import HistoryTab from "./components/HistoryTab";
import SessionsTab from "./components/SessionsTab";

import "./App.css";

function App() {
  const [mode, setMode] = useState("clipboard");
  const [activeTab, setActiveTab] = useState("pinned");
  // pinMode: 'none' | 'always-on-top' | 'soft'. Ephemeral, resets on restart.
  const [pinMode, setPinMode] = useState("none");
  const [showPinLabel, setShowPinLabel] = useState(false);
  const pinLabelTimer = useRef(null);
  const pinChangeAtRef = useRef(0);
  const [history, setHistory] = useState([]);
  const [pinned, setPinned] = useState([]);
  const [globalPinned, setGlobalPinned] = useState([]);
  const [sessions, setSessions] = useState([]);
  const [apps, setApps] = useState([]);
  const [currentClipboard, setCurrentClipboard] = useState("");
  const [historySearch, setHistorySearch] = useState("");
  const [pinnedSearch, setPinnedSearch] = useState("");
  const [sessionsSearch, setSessionsSearch] = useState("");
  const [appsSearch, setAppsSearch] = useState("");
  const [tabShortcutPinned, setTabShortcutPinned] = useState(`${TAB_MOD}+1`);
  const [tabShortcutHistory, setTabShortcutHistory] = useState(`${TAB_MOD}+2`);
  const [tabShortcutSessions, setTabShortcutSessions] = useState(`${TAB_MOD}+3`);
  const [tabShortcutFind, setTabShortcutFind] = useState(`${TAB_MOD}+F`);
  const pinnedSearchRef = useRef(null);
  const historySearchRef = useRef(null);
  const sessionsSearchRef = useRef(null);
  const appsSearchRef = useRef(null);

  const filteredHistory = useMemo(
    () => history.filter((item) => item.content.toLowerCase().includes(historySearch.toLowerCase())),
    [history, historySearch]
  );

  const filteredPinned = useMemo(
    () =>
      pinned.filter(
        (item) =>
          item.content.toLowerCase().includes(pinnedSearch.toLowerCase()) ||
          item.description.toLowerCase().includes(pinnedSearch.toLowerCase())
      ),
    [pinned, pinnedSearch]
  );

  const filteredSessions = useMemo(
    () => sessions.filter((s) => s.name.toLowerCase().includes(sessionsSearch.toLowerCase())),
    [sessions, sessionsSearch]
  );

  const filteredApps = useMemo(() => {
    const tokens = appsSearch.toLowerCase().split(/\s+/).filter(Boolean);
    return apps.filter((a) => {
      const name = a.name.toLowerCase();
      return tokens.every((t) => name.includes(t));
    });
  }, [apps, appsSearch]);

  const pinnedSet = useMemo(() => new Set(globalPinned.map((p) => p.content)), [globalPinned]);

  const pinnedHiddenSet = useMemo(
    () => new Set(globalPinned.filter((p) => p.hidden).map((p) => p.content)),
    [globalPinned]
  );

  const loadingRef = useRef(false);
  const loadData = useCallback(async () => {
    if (loadingRef.current) return;
    loadingRef.current = true;
    try {
      const [h, p, gp, s] = await Promise.all([getHistory(), getPinned(), getGlobalPinned(), getSessions()]);
      setHistory(h);
      setPinned(p);
      setGlobalPinned(gp);
      setSessions(s);
    } catch (e) {
      console.error("Failed to load data", e);
      await logError("error", `Failed to load data: ${e}`);
    } finally {
      loadingRef.current = false;
    }
  }, []);

  const loadHistory = useCallback(async () => {
    try {
      const h = await getHistory();
      setHistory(h);
    } catch (e) {
      await logError("error", `Failed to load history: ${e}`);
    }
  }, []);

  const loadClipboard = useCallback(async () => {
    try {
      const text = await getClipboard();
      setCurrentClipboard(text);
    } catch (e) {
      await logError("warn", `Failed to load clipboard: ${e}`);
    }
  }, []);

  const loadTabShortcuts = useCallback(async () => {
    try {
      const [pinned, history, sessions, find] = await Promise.all([
        getSetting("tab_shortcut_pinned"),
        getSetting("tab_shortcut_history"),
        getSetting("tab_shortcut_sessions"),
        getSetting("tab_shortcut_find"),
      ]);
      setTabShortcutPinned(pinned);
      setTabShortcutHistory(history);
      setTabShortcutSessions(sessions);
      setTabShortcutFind(find);
    } catch (e) {
      await logError("warn", `Failed to load tab shortcuts: ${e}`);
    }
  }, []);

  const loadApps = useCallback(async () => {
    try {
      const a = await listOpenApps();
      setApps(a);
    } catch (e) {
      await logError("error", `Failed to load apps: ${e}`);
    }
  }, []);

  const clearSearch = useCallback(() => {
    setHistorySearch("");
    setPinnedSearch("");
    setSessionsSearch("");
    setAppsSearch("");
  }, []);

  const handleCopy = useCallback(async (text) => {
    await writeText(text);
    await getCurrentWindow().hide();
  }, []);

  const handleSelectApp = useCallback(async (id) => {
    try {
      await focusApp(id);
      await getCurrentWindow().hide();
    } catch (e) {
      await logError("error", `Failed to focus app: ${e}`);
    }
  }, []);

  // Cycle none -> always-on-top -> soft -> none and flash the status label.
  const cyclePinMode = useCallback(() => {
    pinChangeAtRef.current = Date.now();
    setPinMode((prev) => PIN_NEXT[prev]);
    setShowPinLabel(true);
    if (pinLabelTimer.current) clearTimeout(pinLabelTimer.current);
    pinLabelTimer.current = setTimeout(() => setShowPinLabel(false), 1500);
  }, []);

  useEffect(() => () => {
    if (pinLabelTimer.current) clearTimeout(pinLabelTimer.current);
  }, []);

  useEffect(() => {
    loadApps();
  }, [loadApps]);

  const handleActivateSession = useCallback(async (id) => {
    try {
      await activateSession(id);
      await loadData();
    } catch (e) {
      await logError("error", `Failed to activate session: ${e}`);
    }
  }, [loadData]);

  useAppEvents({
    activeTab,
    setActiveTab,
    mode,
    onSetMode: setMode,
    tabShortcutPinned,
    tabShortcutHistory,
    tabShortcutSessions,
    tabShortcutFind,
    filteredApps,
    filteredPinned,
    filteredHistory,
    filteredSessions,
    pinnedSearchRef,
    historySearchRef,
    sessionsSearchRef,
    appsSearchRef,
    onLoadData: loadData,
    onLoadApps: loadApps,
    onLoadHistory: loadHistory,
    onLoadClipboard: loadClipboard,
    onLoadTabShortcuts: loadTabShortcuts,
    onClearSearch: clearSearch,
    onCopy: handleCopy,
    onActivateSession: handleActivateSession,
    onFocusApp: handleSelectApp,
  });

  // Apply the backend window toggles whenever the pin mode changes. Each branch
  // sets both commands explicitly so the prior mode is always cleared first.
  useEffect(() => {
    const applyPinMode = async () => {
      try {
        if (pinMode === "always-on-top") {
          // Regular activation policy (set_soft_pin) puts ClipX in the app
          // switcher with its icon; set_always_on_top makes it float above all.
          await setSoftPin(true);
          await setAlwaysOnTop(true);
        } else if (pinMode === "soft") {
          await setAlwaysOnTop(false);
          await setSoftPin(true);
        } else {
          await setAlwaysOnTop(false);
          await setSoftPin(false);
        }
      } catch (e) {
        await logError("error", `Failed to apply pin mode: ${e}`);
      }
    };
    applyPinMode();
  }, [pinMode]);

  // Hide the window on focus loss only when unpinned. Pinned modes stay visible.
  // The 50ms debounce absorbs the brief focus-flicker that Windows/WebView2 emits
  // when clicking UI elements in a frameless window before the click event fires.
  // The pin-change guard ignores the self-induced blur that macOS emits when
  // unpinning flips the activation policy back to Accessory, so toggling to
  // unpinned keeps the window open (matching Windows) instead of auto-hiding.
  useEffect(() => {
    const win = getCurrentWindow();
    let timer = null;
    const unlistenPromise = win.onFocusChanged(({ payload: focused }) => {
      if (!focused && pinMode === "none" && Date.now() - pinChangeAtRef.current > 700) {
        timer = setTimeout(() => win.hide(), 50);
      } else if (timer !== null) {
        clearTimeout(timer);
        timer = null;
      }
    });
    return () => {
      if (timer !== null) clearTimeout(timer);
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [pinMode]);

  return (
    <main className="container">
      <div className="title-bar" data-tauri-drag-region>
        <button
          type="button"
          className={`pin-btn pin-btn--${pinMode}`}
          title={PIN_TOOLTIPS[pinMode]}
          onMouseDown={(e) => e.stopPropagation()}
          onClick={cyclePinMode}
        >
          <svg
            className="pin-icon"
            viewBox="0 0 24 24"
            width="16"
            height="16"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M12 17v5" />
            <path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z" />
          </svg>
          {pinMode === "soft" && <span className="pin-indicator" />}
        </button>
        <span className={`pin-label ${showPinLabel ? "pin-label--visible" : ""}`}>
          {PIN_LABELS[pinMode]}
        </span>
        <img src="/icon.png" alt="ClipX" className="app-icon" />
        <h1>ClipX</h1>
      </div>
      {mode === "apps" && (
        <AppsTab
          filteredApps={filteredApps}
          appsSearch={appsSearch}
          setAppsSearch={setAppsSearch}
          appsSearchRef={appsSearchRef}
          onSelect={handleSelectApp}
        />
      )}
      {mode === "clipboard" && (
        <>
      <div className="tabs">
        <button
          className={activeTab === "pinned" ? "active" : ""}
          onClick={() => setActiveTab("pinned")}
        >
          Pinned
        </button>
        <button
          className={activeTab === "history" ? "active" : ""}
          onClick={() => setActiveTab("history")}
        >
          History
        </button>
        <button
          className={activeTab === "sessions" ? "active" : ""}
          onClick={() => setActiveTab("sessions")}
        >
          Sessions
        </button>
      </div>
      {activeTab === "pinned" && (
        <PinnedTab
          pinned={pinned}
          pinnedSearch={pinnedSearch}
          setPinnedSearch={setPinnedSearch}
          pinnedSearchRef={pinnedSearchRef}
          currentClipboard={currentClipboard}
          sessions={sessions}
          onCopy={handleCopy}
          onDataChanged={loadData}
        />
      )}
      {activeTab === "history" && (
        <HistoryTab
          history={history}
          historySearch={historySearch}
          setHistorySearch={setHistorySearch}
          historySearchRef={historySearchRef}
          currentClipboard={currentClipboard}
          pinnedSet={pinnedSet}
          pinnedHiddenSet={pinnedHiddenSet}
          sessions={sessions}
          onCopy={handleCopy}
          onDataChanged={loadData}
        />
      )}
      {activeTab === "sessions" && (
        <SessionsTab
          sessions={sessions}
          sessionsSearch={sessionsSearch}
          setSessionsSearch={setSessionsSearch}
          sessionsSearchRef={sessionsSearchRef}
          onDataChanged={loadData}
        />
      )}
        </>
      )}
    </main>
  );
}

export default App;
