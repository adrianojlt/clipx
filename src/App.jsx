import { useRef, useState, useMemo, useCallback } from "react";
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
  logError,
} from "./services/clipboardService";

import { useAppEvents } from "./hooks/useAppEvents";
import { IS_MAC } from "./utils/shortcuts";

const TAB_MOD = IS_MAC ? "Command" : "Alt";

import PinnedTab from "./components/PinnedTab";
import HistoryTab from "./components/HistoryTab";
import SessionsTab from "./components/SessionsTab";

import "./App.css";

function App() {
  const [activeTab, setActiveTab] = useState("pinned");
  const [history, setHistory] = useState([]);
  const [pinned, setPinned] = useState([]);
  const [globalPinned, setGlobalPinned] = useState([]);
  const [sessions, setSessions] = useState([]);
  const [currentClipboard, setCurrentClipboard] = useState("");
  const [historySearch, setHistorySearch] = useState("");
  const [pinnedSearch, setPinnedSearch] = useState("");
  const [sessionsSearch, setSessionsSearch] = useState("");
  const [tabShortcutPinned, setTabShortcutPinned] = useState(`${TAB_MOD}+1`);
  const [tabShortcutHistory, setTabShortcutHistory] = useState(`${TAB_MOD}+2`);
  const [tabShortcutSessions, setTabShortcutSessions] = useState(`${TAB_MOD}+3`);
  const [tabShortcutFind, setTabShortcutFind] = useState(`${TAB_MOD}+F`);
  const pinnedSearchRef = useRef(null);
  const historySearchRef = useRef(null);
  const sessionsSearchRef = useRef(null);

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

  const clearSearch = useCallback(() => {
    setHistorySearch("");
    setPinnedSearch("");
    setSessionsSearch("");
  }, []);

  const handleCopy = useCallback(async (text) => {
    await writeText(text);
    await getCurrentWindow().hide();
  }, []);

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
    tabShortcutPinned,
    tabShortcutHistory,
    tabShortcutSessions,
    tabShortcutFind,
    filteredPinned,
    filteredHistory,
    filteredSessions,
    pinnedSearchRef,
    historySearchRef,
    sessionsSearchRef,
    onLoadData: loadData,
    onLoadHistory: loadHistory,
    onLoadClipboard: loadClipboard,
    onLoadTabShortcuts: loadTabShortcuts,
    onClearSearch: clearSearch,
    onCopy: handleCopy,
    onActivateSession: handleActivateSession,
  });

  return (
    <main className="container">
      <div className="title-bar">
        <img src="/icon.png" alt="ClipX" className="app-icon" />
        <h1>ClipX</h1>
      </div>
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
    </main>
  );
}

export default App;
