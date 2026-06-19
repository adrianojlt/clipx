import { useState } from "react";
import { pinItem, deleteHistoryItem, pinItemToSession, logError } from "../services/clipboardService";
import HistoryItem from "./HistoryItem";

export default function HistoryTab({
  history,
  historySearch,
  setHistorySearch,
  historySearchRef,
  currentClipboard,
  pinnedSet,
  pinnedHiddenSet,
  sessions,
  onCopy,
  onDataChanged,
}) {
  const [confirmDeleteHistoryId, setConfirmDeleteHistoryId] = useState(null);

  const filteredHistory = history.filter((item) =>
    item.content.toLowerCase().includes(historySearch.toLowerCase())
  );

  const handlePin = async (content) => {
    try {
      await pinItem(content);
      await onDataChanged();
    } catch (e) {
      await logError("error", `Failed to pin item: ${e}`);
    }
  };

  const handleDeleteHistory = async (id) => {
    try {
      await deleteHistoryItem(id);
      setConfirmDeleteHistoryId(null);
      await onDataChanged();
    } catch (e) {
      await logError("error", `Failed to delete history item: ${e}`);
    }
  };

  const handlePinToSession = async (content, sessionId) => {
    try {
      await pinItemToSession(content, sessionId);
      await onDataChanged();
    } catch (e) {
      await logError("error", `Failed to pin to session: ${e}`);
    }
  };

  return (
    <>
      <div className="search-bar">
        <input
          ref={historySearchRef}
          className="search-input"
          type="text"
          autoComplete="off"
          autoCorrect="off"
          spellCheck={false}
          placeholder="Search history..."
          value={historySearch}
          onChange={(e) => setHistorySearch(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              e.stopPropagation();
              e.target.blur();
            }
          }}
        />
      </div>
      <div className="list">
        {history.length === 0 && <div className="empty">No history</div>}
        {filteredHistory.map((item) => (
          <HistoryItem
            key={item.id}
            item={item}
            isCurrentClipboard={item.content === currentClipboard}
            isPinned={pinnedSet.has(item.content)}
            isHidden={pinnedHiddenSet.has(item.content)}
            confirmDeleteId={confirmDeleteHistoryId}
            sessions={sessions}
            onCopy={onCopy}
            onPin={handlePin}
            onPinToSession={handlePinToSession}
            onRequestDelete={setConfirmDeleteHistoryId}
            onConfirmDelete={handleDeleteHistory}
            onCancelDelete={() => setConfirmDeleteHistoryId(null)}
          />
        ))}
      </div>
    </>
  );
}
