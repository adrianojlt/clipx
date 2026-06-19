import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { setSetting, logError } from "../services/clipboardService";
import { matchesShortcut } from "../utils/shortcuts";
import { EVENTS } from "../constants/events";

export function useAppEvents({
  activeTab,
  setActiveTab,
  mode,
  onSetMode,
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
  onLoadData,
  onLoadApps,
  onLoadHistory,
  onLoadClipboard,
  onLoadTabShortcuts,
  onClearSearch,
  onCopy,
  onActivateSession,
  onFocusApp,
}) {
  useEffect(() => {
    onLoadData();
    onLoadTabShortcuts();

    const retryTimer = setTimeout(() => onLoadData(), 500);

    let cancelled = false;
    const unlisteners = [];

    const setupListeners = async () => {

      await onLoadClipboard();

      if (cancelled) return;

      const u1 = await listen(EVENTS.CLIPBOARD_CHANGED, async () => {
        onLoadHistory();
        await onLoadClipboard();
      });

      if (cancelled) { u1(); return; }

      unlisteners.push(u1);

      const u2 = await listen("tauri://window-focus", async () => {
        await onLoadClipboard();
      });

      if (cancelled) { u2(); return; }

      unlisteners.push(u2);

      const u3 = await listen("main-window-shown", (event) => {
        const nextMode = event.payload === "apps" ? "apps" : "clipboard";
        onSetMode(nextMode);
        onClearSearch();
        onLoadData();
        onLoadApps();
        if (nextMode === "apps") {
          setTimeout(() => appsSearchRef.current?.focus(), 0);
        }
      });

      if (cancelled) { u3(); return; }

      unlisteners.push(u3);

      const u4 = await listen("settings-changed", () => {
        onLoadTabShortcuts();
      });

      if (cancelled) { u4(); return; }

      unlisteners.push(u4);
    };

    setupListeners();

    return () => {
      cancelled = true;
      clearTimeout(retryTimer);
      unlisteners.forEach((fn) => fn());
    };
  }, [onLoadData, onLoadApps, onLoadHistory, onLoadClipboard, onLoadTabShortcuts, onClearSearch, onSetMode, appsSearchRef]);

  useEffect(() => {
    const onKey = async (e) => {
      if (e.key === "Escape") await getCurrentWindow().hide();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {

    const onKey = (e) => {

      if (mode !== "clipboard") return;

      const tag = e.target.tagName;

      if (tag === "INPUT" || tag === "TEXTAREA") return;

      if (matchesShortcut(e, tabShortcutPinned)) {
        e.preventDefault();
        setActiveTab("pinned");
      } else if (matchesShortcut(e, tabShortcutHistory)) {
        e.preventDefault();
        setActiveTab("history");
      } else if (matchesShortcut(e, tabShortcutSessions)) {
        e.preventDefault();
        setActiveTab("sessions");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [mode, tabShortcutPinned, tabShortcutHistory, tabShortcutSessions, setActiveTab]);

  useEffect(() => {
    const onKey = (e) => {

      if (mode !== "clipboard") return;

      if (!matchesShortcut(e, tabShortcutFind)) return;

      e.preventDefault();

      const ref =
        activeTab === "pinned"
          ? pinnedSearchRef
          : activeTab === "sessions"
          ? sessionsSearchRef
          : historySearchRef;

      ref.current?.focus();
    };

    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [mode, tabShortcutFind, activeTab, pinnedSearchRef, historySearchRef, sessionsSearchRef]);

  useEffect(() => {

    let timeout;

    const onResize = () => {

      clearTimeout(timeout);

      timeout = setTimeout(async () => {
        try {
          await setSetting("window_width", String(window.innerWidth));
          await setSetting("window_height", String(window.innerHeight));
        } catch (e) {
          await logError("warn", `Failed to save window size: ${e}`);
        }
      }, 300);
    };

    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("resize", onResize);
      clearTimeout(timeout);
    };
  }, []);

  useEffect(() => {

    const titleBar = document.querySelector(".title-bar");

    if (!titleBar) return;

    const onMouseDown = async () => {
      try {
        await getCurrentWindow().startDragging();
      } catch (e) {
        await logError("warn", `Failed to start dragging: ${e}`);
      }
    };

    titleBar.addEventListener("mousedown", onMouseDown);
    return () => titleBar.removeEventListener("mousedown", onMouseDown);
  }, []);

  useEffect(() => {
    const onKey = (e) => {

      const tag = e.target.tagName;

      if (mode !== "clipboard") return;

      if (tag === "INPUT" || tag === "TEXTAREA") return;

      const num = parseInt(e.key);

      if (num >= 1 && num <= 5 && !e.metaKey && !e.ctrlKey && !e.altKey) {

        const index = num - 1;

        if (activeTab === "sessions" && index < filteredSessions.length) {
          e.preventDefault();
          onActivateSession(filteredSessions[index].id);
          return;
        }

        const list = activeTab === "pinned" ? filteredPinned : filteredHistory;

        if (index < list.length) {
          e.preventDefault();
          onCopy(list[index].content);
        }
      }
    };

    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [mode, activeTab, filteredPinned, filteredHistory, filteredSessions, onCopy, onActivateSession]);

  useEffect(() => {

    const onKey = (e) => {

      if (mode !== "apps") return;

      const tag = e.target.tagName;

      // number keys only act once the search box has been blurred (via Esc)
      if (tag === "INPUT" || tag === "TEXTAREA") return;

      const num = parseInt(e.key);

      if (num >= 1 && num <= 9 && !e.metaKey && !e.ctrlKey && !e.altKey) {

        const index = num - 1;

        if (index < filteredApps.length) {
          e.preventDefault();
          onFocusApp(filteredApps[index].id);
        }
      }
    };

    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [mode, filteredApps, onFocusApp]);
}
