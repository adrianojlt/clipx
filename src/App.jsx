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
} from "./services/clipboardService";

import { useAppEvents } from "./hooks/useAppEvents";
import { IS_MAC } from "./utils/shortcuts";

const TAB_MOD = IS_MAC ? "Command" : "Alt";

import AppsTab from "./components/AppsTab";
import PinnedTab from "./components/PinnedTab";
import HistoryTab from "./components/HistoryTab";
import SessionsTab from "./components/SessionsTab";

import "./App.css";

function App() {
  const [activeTab, setActiveTab] = useState("apps");
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
  const [tabShortcutApps, setTabShortcutApps] = useState(`${TAB_MOD}+1`);
  const [tabShortcutPinned, setTabShortcutPinned] = useState(`${TAB_MOD}+2`);
  const [tabShortcutHistory, setTabShortcutHistory] = useState(`${TAB_MOD}+3`);
  const [tabShortcutSessions, setTabShortcutSessions] = useState(`${TAB_MOD}+4`);
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
      const [apps, pinned, history, sessions, find] = await Promise.all([
        getSetting("tab_shortcut_apps"),
        getSetting("tab_shortcut_pinned"),
        getSetting("tab_shortcut_history"),
        getSetting("tab_shortcut_sessions"),
        getSetting("tab_shortcut_find"),
      ]);
      setTabShortcutApps(apps);
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
    tabShortcutApps,
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

  return (
    <main className="container">
      <div className="title-bar">
        <img src="/icon.png" alt="ClipX" className="app-icon" />
        <h1>ClipX</h1>
      </div>
      <div className="tabs">
        <button
          className={activeTab === "apps" ? "active" : ""}
          onClick={() => setActiveTab("apps")}
        >
          Apps
        </button>
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
      {activeTab === "apps" && (
        <AppsTab
          filteredApps={filteredApps}
          appsSearch={appsSearch}
          setAppsSearch={setAppsSearch}
          appsSearchRef={appsSearchRef}
          onSelect={handleSelectApp}
        />
      )}
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
    </main>
  );
}

export default App;
