import { useEffect, useRef, useState, useMemo } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import {
  getHistory,
  getPinned,
  getClipboard,
  getSetting,
  setSetting,
  pinItem,
  deleteHistoryItem,
  unpinItem,
  updatePinnedDescription,
  togglePinnedHidden,
  reorderPinned,
  logError,
} from "./services/clipboardService";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { matchesShortcut } from "./utils/shortcuts";
import HistoryItem from "./components/HistoryItem";
import PinnedItem from "./components/PinnedItem";
import { EVENTS } from "./constants/events";
import "./App.css";

function App() {
  const [activeTab, setActiveTab] = useState("pinned");
  const [history, setHistory] = useState([]);
  const [pinned, setPinned] = useState([]);
  const [dragIndicator, setDragIndicator] = useState(null);
  const [draggingId, setDraggingId] = useState(null);
  const [editingId, setEditingId] = useState(null);
  const [editingValue, setEditingValue] = useState("");
  const [historySearch, setHistorySearch] = useState("");
  const [pinnedSearch, setPinnedSearch] = useState("");
  const [confirmDeleteHistoryId, setConfirmDeleteHistoryId] = useState(null);
  const [confirmUnpinId, setConfirmUnpinId] = useState(null);
  const [currentClipboard, setCurrentClipboard] = useState("");
  const [tabShortcutPinned, setTabShortcutPinned] = useState("Command+1");
  const [tabShortcutHistory, setTabShortcutHistory] = useState("Command+2");
  const pinnedRef = useRef([]);
  const listRef = useRef(null);

  const filteredHistory = useMemo(
    () =>
      history.filter((item) => item.content.toLowerCase().includes(historySearch.toLowerCase())),
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

  const loadData = async () => {
    try {
      const h = await getHistory();
      setHistory(h);
      const p = await getPinned();
      setPinned(p);
      pinnedRef.current = p;
    } catch (e) {
      console.error("Failed to load data", e);
      await logError("error", `Failed to load data: ${e}`);
    }
  };

  const loadClipboard = async () => {
    try {
      const text = await getClipboard();
      setCurrentClipboard(text);
    } catch (e) {
      await logError("warn", `Failed to load clipboard: ${e}`);
    }
  };

  const loadTabShortcuts = async () => {
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
  };

  useEffect(() => {
    pinnedRef.current = pinned;
  }, [pinned]);

  useEffect(() => {
    loadData();
    loadTabShortcuts();

    let unlistenClipboard;
    let unlistenFocus;

    const setupListeners = async () => {
      await loadClipboard();

      unlistenClipboard = await listen(EVENTS.CLIPBOARD_CHANGED, async () => {
        loadData();
        await loadClipboard();
      });

      unlistenFocus = await listen("tauri://window-focus", async () => {
        await loadClipboard();
      });
    };

    setupListeners();

    return () => {
      if (unlistenClipboard) unlistenClipboard();
      if (unlistenFocus) unlistenFocus();
    };
  }, []);

  useEffect(() => {
    const onKey = async (e) => {
      if (e.key === "Escape") await getCurrentWindow().hide();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    const onKey = (e) => {
      const tag = e.target.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;
      if (matchesShortcut(e, tabShortcutPinned)) {
        e.preventDefault();
        setActiveTab("pinned");
      } else if (matchesShortcut(e, tabShortcutHistory)) {
        e.preventDefault();
        setActiveTab("history");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [tabShortcutPinned, tabShortcutHistory]);

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

  const handleCopy = async (text) => {
    await writeText(text);
    await getCurrentWindow().hide();
  };

  const handlePin = async (content) => {
    try {
      await pinItem(content);
      await loadData();
    } catch (e) {
      await logError("error", `Failed to pin item: ${e}`);
    }
  };

  const handleDeleteHistory = async (id) => {
    try {
      await deleteHistoryItem(id);
      setConfirmDeleteHistoryId(null);
      await loadData();
    } catch (e) {
      await logError("error", `Failed to delete history item: ${e}`);
    }
  };

  const handleUnpin = async (id) => {
    try {
      await unpinItem(id);
      await loadData();
    } catch (e) {
      await logError("error", `Failed to unpin item: ${e}`);
    }
  };

  const handleSaveDescription = async (id) => {
    try {
      await updatePinnedDescription(id, editingValue);
      setEditingId(null);
      await loadData();
    } catch (e) {
      await logError("error", `Failed to save description: ${e}`);
    }
  };

  const handleToggleHidden = async (id) => {
    try {
      await togglePinnedHidden(id);
      await loadData();
    } catch (e) {
      await logError("error", `Failed to toggle hidden: ${e}`);
    }
  };

  // --- Manual drag-and-drop with mouse events ---

  const handleMouseDown = (e, id) => {
    e.preventDefault();
    e.stopPropagation();
    setDraggingId(id);
    let currentIndicator = null;

    const onMouseMove = (ev) => {
      if (!listRef.current) return;
      const listRect = listRef.current.getBoundingClientRect();
      const relY = ev.clientY - listRect.top + listRef.current.scrollTop;

      const children = Array.from(listRef.current.children);
      let closest = null;
      let closestPos = "after";
      let minDist = Infinity;

      for (const child of children) {
        const rect = child.getBoundingClientRect();
        const childTop = rect.top - listRect.top + listRef.current.scrollTop;
        const childCenter = childTop + rect.height / 2;
        const dist = Math.abs(relY - childCenter);
        if (dist < minDist) {
          minDist = dist;
          closest = child;
          closestPos = relY < childCenter ? "before" : "after";
        }
      }

      if (closest) {
        const targetId = Number(closest.dataset.id);
        if (targetId !== id) {
          currentIndicator = { targetId, position: closestPos };
          setDragIndicator(currentIndicator);
        } else {
          currentIndicator = null;
          setDragIndicator(null);
        }
      }
    };

    const onMouseUp = async () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);

      const currentPinned = pinnedRef.current;
      const draggedIndex = currentPinned.findIndex((item) => item.id === id);
      if (draggedIndex === -1) {
        setDraggingId(null);
        setDragIndicator(null);
        return;
      }

      let targetIndex = draggedIndex;
      if (currentIndicator) {
        targetIndex = currentPinned.findIndex((item) => item.id === currentIndicator.targetId);
        if (targetIndex !== -1) {
          if (currentIndicator.position === "after") {
            targetIndex += 1;
          }
        }
      }

      if (targetIndex !== draggedIndex && targetIndex !== -1) {
        const newPinned = [...currentPinned];
        const [draggedItem] = newPinned.splice(draggedIndex, 1);
        const insertIndex = draggedIndex < targetIndex ? targetIndex - 1 : targetIndex;
        newPinned.splice(insertIndex, 0, draggedItem);

        setPinned(newPinned);
        pinnedRef.current = newPinned;

        try {
          const ids = newPinned.map((item) => item.id);
          await reorderPinned(ids);
        } catch (e) {
          await logError("error", `Failed to reorder pinned: ${e}`);
        }
      }

      setDraggingId(null);
      setDragIndicator(null);
    };

    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  };

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
      </div>
      {activeTab === "pinned" && (
        <div className="search-bar">
          <input
            className="search-input"
            type="text"
            placeholder="Search pinned..."
            value={pinnedSearch}
            onChange={(e) => setPinnedSearch(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                e.stopPropagation();
                setPinnedSearch("");
              }
            }}
          />
        </div>
      )}
      {activeTab === "history" && (
        <div className="search-bar">
          <input
            className="search-input"
            type="text"
            placeholder="Search history..."
            value={historySearch}
            onChange={(e) => setHistorySearch(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                e.stopPropagation();
                setHistorySearch("");
              }
            }}
          />
        </div>
      )}
      <div className="list" ref={listRef}>
        {activeTab === "pinned" && (
          <>
            {filteredPinned.length === 0 && <div className="empty">No pinned items</div>}
            {filteredPinned.map((item) => (
              <PinnedItem
                key={item.id}
                item={item}
                isCurrentClipboard={item.content === currentClipboard}
                isDragging={draggingId === item.id}
                dragIndicator={dragIndicator}
                editingId={editingId}
                editingValue={editingValue}
                confirmUnpinId={confirmUnpinId}
                onCopy={handleCopy}
                onMouseDown={handleMouseDown}
                onToggleHidden={handleToggleHidden}
                onStartEdit={(id, desc) => {
                  setEditingId(id);
                  setEditingValue(desc);
                }}
                onEditChange={setEditingValue}
                onSaveEdit={handleSaveDescription}
                onCancelEdit={() => {
                  setEditingId(null);
                  setEditingValue("");
                }}
                onRequestUnpin={setConfirmUnpinId}
                onConfirmUnpin={(id) => {
                  handleUnpin(id);
                  setConfirmUnpinId(null);
                }}
                onCancelUnpin={() => setConfirmUnpinId(null)}
              />
            ))}
          </>
        )}
        {activeTab === "history" && (
          <>
            {history.length === 0 && <div className="empty">No history</div>}
            {(() => {
              const pinnedSet = new Set(pinned.map((p) => p.content));
              return filteredHistory.map((item) => (
                <HistoryItem
                  key={item.id}
                  item={item}
                  isCurrentClipboard={item.content === currentClipboard}
                  isPinned={pinnedSet.has(item.content)}
                  confirmDeleteId={confirmDeleteHistoryId}
                  onCopy={handleCopy}
                  onPin={handlePin}
                  onRequestDelete={setConfirmDeleteHistoryId}
                  onConfirmDelete={handleDeleteHistory}
                  onCancelDelete={() => setConfirmDeleteHistoryId(null)}
                />
              ));
            })()}
          </>
        )}
      </div>
    </main>
  );
}

export default App;
