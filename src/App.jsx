import { useRef, useState, useMemo, useCallback } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import {
  getHistory,
  getPinned,
  getSessions,
  getClipboard,
  getSetting,
  activateSession,
  logError,
} from "./services/clipboardService";
import { useAppEvents } from "./hooks/useAppEvents";
import PinnedTab from "./components/PinnedTab";
import HistoryTab from "./components/HistoryTab";
import SessionsTab from "./components/SessionsTab";
import "./App.css";

function App() {
  const [activeTab, setActiveTab] = useState("pinned");
  const [history, setHistory] = useState([]);
  const [pinned, setPinned] = useState([]);
  const [sessions, setSessions] = useState([]);
  const [currentClipboard, setCurrentClipboard] = useState("");
  const [historySearch, setHistorySearch] = useState("");
  const [pinnedSearch, setPinnedSearch] = useState("");
  const [sessionsSearch, setSessionsSearch] = useState("");
  const [tabShortcutPinned, setTabShortcutPinned] = useState("Command+1");
  const [tabShortcutHistory, setTabShortcutHistory] = useState("Command+2");
  const [tabShortcutSessions, setTabShortcutSessions] = useState("Command+3");
  const [tabShortcutFind, setTabShortcutFind] = useState("Command+F");
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

  const pinnedSet = useMemo(() => new Set(pinned.map((p) => p.content)), [pinned]);

  const pinnedHiddenSet = useMemo(
    () => new Set(pinned.filter((p) => p.hidden).map((p) => p.content)),
    [pinned]
  );

  const loadData = useCallback(async () => {
    try {
      const h = await getHistory();
      setHistory(h);
      const p = await getPinned();
      setPinned(p);
      const s = await getSessions();
      setSessions(s);
    } catch (e) {
      console.error("Failed to load data", e);
      await logError("error", `Failed to load data: ${e}`);
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
      const v = await getSetting("tab_shortcut_pinned");
      setTabShortcutPinned(v);
    } catch (e) {
      await logError("warn", `Failed to load tab shortcut pinned: ${e}`);
    }
    try {
      const v = await getSetting("tab_shortcut_history");
      setTabShortcutHistory(v);
    } catch (e) {
      await logError("warn", `Failed to load tab shortcut history: ${e}`);
    }
    try {
      const v = await getSetting("tab_shortcut_sessions");
      setTabShortcutSessions(v);
    } catch (e) {
      await logError("warn", `Failed to load tab shortcut sessions: ${e}`);
    }
    try {
      const v = await getSetting("tab_shortcut_find");
      setTabShortcutFind(v);
    } catch (e) {
      await logError("warn", `Failed to load tab shortcut find: ${e}`);
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
