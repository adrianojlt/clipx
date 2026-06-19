import { useState, useRef, useEffect } from "react";
import {
  reorderPinned,
  unpinItem,
  updatePinnedDescription,
  togglePinnedHidden,
  pinItemToSession,
  logError,
} from "../services/clipboardService";
import { useDragReorder } from "../hooks/useDragReorder";
import PinnedItem from "./PinnedItem";

export default function PinnedTab({
  pinned,
  pinnedSearch,
  setPinnedSearch,
  pinnedSearchRef,
  currentClipboard,
  sessions,
  onCopy,
  onDataChanged,
}) {
  const [items, setItems] = useState(pinned);
  const listRef = useRef(null);
  const itemsRef = useRef(pinned);

  useEffect(() => {
    setItems(pinned);
    itemsRef.current = pinned;
  }, [pinned]);

  const { draggingId, dragIndicator, handleMouseDown } = useDragReorder({
    listRef,
    itemsRef,
    setItems,
    reorderFn: reorderPinned,
  });

  const filteredItems = items.filter(
    (item) =>
      item.content.toLowerCase().includes(pinnedSearch.toLowerCase()) ||
      item.description.toLowerCase().includes(pinnedSearch.toLowerCase())
  );

  const handleUnpin = async (id) => {
    try {
      await unpinItem(id);
      await onDataChanged();
    } catch (e) {
      await logError("error", `Failed to unpin item: ${e}`);
    }
  };

  const handleSaveDescription = async (id, value) => {
    try {
      await updatePinnedDescription(id, value);
      await onDataChanged();
    } catch (e) {
      await logError("error", `Failed to save description: ${e}`);
    }
  };

  const handlePinToSession = async (content, sessionId, description) => {
    try {
      await pinItemToSession(content, sessionId, description);
      await onDataChanged();
    } catch (e) {
      await logError("error", `Failed to pin to session: ${e}`);
    }
  };

  const handleToggleHidden = async (id) => {
    try {
      await togglePinnedHidden(id);
      await onDataChanged();
    } catch (e) {
      await logError("error", `Failed to toggle hidden: ${e}`);
    }
  };

  return (
    <>
      <div className="search-bar">
        <input
          ref={pinnedSearchRef}
          className="search-input"
          type="text"
          autoComplete="off"
          autoCorrect="off"
          spellCheck={false}
          placeholder="Search pinned..."
          value={pinnedSearch}
          onChange={(e) => setPinnedSearch(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              e.stopPropagation();
              e.target.blur();
            }
          }}
        />
      </div>
      <div className="list" ref={listRef}>
        {filteredItems.length === 0 && <div className="empty">No pinned items</div>}
        {filteredItems.map((item) => (
          <PinnedItem
            key={item.id}
            item={item}
            isCurrentClipboard={item.content === currentClipboard}
            isDragging={draggingId === item.id}
            dragIndicator={dragIndicator}
            sessions={sessions}
            onCopy={onCopy}
            onMouseDown={handleMouseDown}
            onToggleHidden={handleToggleHidden}
            onSaveDescription={handleSaveDescription}
            onUnpin={handleUnpin}
            onPinToSession={handlePinToSession}
          />
        ))}
      </div>
    </>
  );
}
