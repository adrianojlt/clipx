import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
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
  const pinnedRef = useRef([]);
  const listRef = useRef(null);

  const loadData = async () => {
    try {
      const h = await invoke("get_history");
      setHistory(h);
      const p = await invoke("get_pinned");
      setPinned(p);
      pinnedRef.current = p;
    } catch (e) {
      console.error("Failed to load data", e);
    }
  };

  useEffect(() => {
    pinnedRef.current = pinned;
  }, [pinned]);

  useEffect(() => {
    loadData();

    let unlisten;
    const setupListener = async () => {
      unlisten = await listen("clipboard-changed", () => {
        loadData();
      });
    };
    setupListener();

    return () => {
      if (unlisten) unlisten();
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
    const titleBar = document.querySelector(".title-bar");
    if (!titleBar) return;
    const onMouseDown = async () => {
      await getCurrentWindow().startDragging();
    };
    titleBar.addEventListener("mousedown", onMouseDown);
    return () => titleBar.removeEventListener("mousedown", onMouseDown);
  }, []);

  const handleCopy = async (text) => {
    await writeText(text);
    await getCurrentWindow().hide();
  };

  const handlePin = async (content) => {
    await invoke("pin_item", { content });
    await loadData();
  };

  const handleUnpin = async (id) => {
    await invoke("unpin_item", { id });
    await loadData();
  };

  const handleSaveDescription = async (id) => {
    await invoke("update_pinned_description", { id, description: editingValue });
    setEditingId(null);
    await loadData();
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

        const ids = newPinned.map((item) => item.id);
        await invoke("reorder_pinned", { items: ids });
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
      {activeTab === "history" && (
        <div className="search-bar">
          <input
            className="search-input"
            type="text"
            placeholder="Search history..."
            value={historySearch}
            onChange={e => setHistorySearch(e.target.value)}
            onKeyDown={e => {
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
            {pinned.length === 0 && (
              <div className="empty">No pinned items</div>
            )}
            {pinned.map((item, index) => (
              <div
                key={item.id}
                data-id={item.id}
                className={`item-wrapper${draggingId === item.id ? " dragging" : ""}`}
              >
                {dragIndicator?.targetId === item.id && dragIndicator.position === "before" && (
                  <div className="drop-indicator" />
                )}
                <div
                  className="item"
                  onClick={() => handleCopy(item.content)}
                >
                  <span
                    className="drag-handle"
                    onMouseDown={(e) => handleMouseDown(e, item.id)}
                    title="Drag to reorder"
                  >
                    &#x2630;
                  </span>
                  <div className="pinned-text">
                    {editingId === item.id ? (
                      <input
                        className="description-edit"
                        value={editingValue}
                        autoFocus
                        onChange={e => setEditingValue(e.target.value)}
                        onKeyDown={e => {
                          if (e.key === "Enter") handleSaveDescription(item.id);
                          if (e.key === "Escape") { e.stopPropagation(); setEditingId(null); }
                        }}
                        onClick={e => e.stopPropagation()}
                      />
                    ) : (
                      <span className="description">{item.description}</span>
                    )}
                    <span className="content-text">{item.content}</span>
                  </div>
                  <button
                    className="action"
                    onClick={e => { e.stopPropagation(); setEditingId(item.id); setEditingValue(item.description); }}
                    title="Edit description"
                  >
                    &#x270E;
                  </button>
                  <button
                    className="action"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleUnpin(item.id);
                    }}
                    title="Unpin"
                  >
                    &#x2715;
                  </button>
                </div>
                {dragIndicator?.targetId === item.id && dragIndicator.position === "after" && (
                  <div className="drop-indicator" />
                )}
              </div>
            ))}
          </>
        )}
        {activeTab === "history" && (
          <>
            {history.length === 0 && (
              <div className="empty">No history</div>
            )}
            {history.filter(item =>
              item.content.toLowerCase().includes(historySearch.toLowerCase())
            ).map((item) => (
              <div
                key={item.id}
                className="item"
                onClick={() => handleCopy(item.content)}
              >
                <span className="text">{item.content}</span>
                <button
                  className="action"
                  onClick={(e) => {
                    e.stopPropagation();
                    handlePin(item.content);
                  }}
                  title="Pin"
                >
                  &#x2605;
                </button>
              </div>
            ))}
          </>
        )}
      </div>
    </main>
  );
}

export default App;
