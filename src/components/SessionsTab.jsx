import { useState, useRef, useEffect } from "react";
import { reorderSessions, createSession, deleteSession, activateSession, logError } from "../services/clipboardService";
import { useDragReorder } from "../hooks/useDragReorder";
import SessionItem from "./SessionItem";

export default function SessionsTab({
  sessions,
  sessionsSearch,
  setSessionsSearch,
  sessionsSearchRef,
  onDataChanged,
}) {
  const [items, setItems] = useState(sessions);
  const [confirmDeleteSessionId, setConfirmDeleteSessionId] = useState(null);
  const [newSessionName, setNewSessionName] = useState("");
  const listRef = useRef(null);
  const itemsRef = useRef(sessions);

  useEffect(() => {
    setItems(sessions);
    itemsRef.current = sessions;
  }, [sessions]);

  const { draggingId, dragIndicator, handleMouseDown } = useDragReorder({
    listRef,
    itemsRef,
    setItems,
    reorderFn: reorderSessions,
  });

  const filteredItems = items.filter((s) =>
    s.name.toLowerCase().includes(sessionsSearch.toLowerCase())
  );

  const handleActivateSession = async (id) => {
    try {
      await activateSession(id);
      await onDataChanged();
    } catch (e) {
      await logError("error", `Failed to activate session: ${e}`);
    }
  };

  const handleCreateSession = async () => {
    const name = newSessionName.trim();
    if (!name) return;
    try {
      await createSession(name);
      setNewSessionName("");
      await onDataChanged();
    } catch (e) {
      await logError("error", `Failed to create session: ${e}`);
    }
  };

  const handleDeleteSession = async (id) => {
    try {
      await deleteSession(id);
      setConfirmDeleteSessionId(null);
      await onDataChanged();
    } catch (e) {
      await logError("error", `Failed to delete session: ${e}`);
    }
  };

  return (
    <>
      <div className="search-bar sessions-bar">
        <input
          ref={sessionsSearchRef}
          className="search-input"
          type="text"
          autoComplete="off"
          autoCorrect="off"
          spellCheck={false}
          placeholder="Search sessions..."
          value={sessionsSearch}
          onChange={(e) => setSessionsSearch(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              e.stopPropagation();
              e.target.blur();
            }
          }}
        />
        <input
          className="session-create-input"
          type="text"
          placeholder="New session..."
          value={newSessionName}
          onChange={(e) => setNewSessionName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") handleCreateSession();
            if (e.key === "Escape") {
              e.stopPropagation();
              e.target.blur();
            }
          }}
        />
      </div>
      <div className="list" ref={listRef}>
        {filteredItems.length === 0 && <div className="empty">No sessions</div>}
        {filteredItems.map((item) => (
          <SessionItem
            key={item.id}
            item={item}
            isActive={item.is_active}
            isDragging={draggingId === item.id}
            dragIndicator={dragIndicator}
            confirmDeleteId={confirmDeleteSessionId}
            onActivate={handleActivateSession}
            onMouseDown={handleMouseDown}
            onRequestDelete={setConfirmDeleteSessionId}
            onConfirmDelete={handleDeleteSession}
            onCancelDelete={() => setConfirmDeleteSessionId(null)}
          />
        ))}
      </div>
    </>
  );
}
