import { useState, useRef } from "react";
import ReactDOM from "react-dom";

export default function HistoryItem({
  item,
  isCurrentClipboard,
  isPinned,
  isHidden,
  confirmDeleteId,
  onCopy,
  onPin,
  onRequestDelete,
  onConfirmDelete,
  onCancelDelete,
}) {
  const isConfirming = confirmDeleteId === item.id;
  const [tooltip, setTooltip] = useState(null);
  const timerRef = useRef(null);
  const hideRef = useRef(null);
  const itemRef = useRef(null);

  function handleMouseEnter() {

    clearTimeout(hideRef.current);

    if (timerRef.current) return;

    timerRef.current = setTimeout(() => {

      timerRef.current = null;

      if (!itemRef.current) return;

      const rect = itemRef.current.getBoundingClientRect();
      const estimatedHeight = 120;
      const top = rect.bottom + estimatedHeight > window.innerHeight
        ? rect.top - estimatedHeight - 4
        : rect.bottom + 4;
      const left = Math.min(rect.left, window.innerWidth - 328);

      setTooltip({ top, left });
    }, 2000);
  }

  function handleMouseLeave() {
    hideRef.current = setTimeout(() => {
      clearTimeout(timerRef.current);
      timerRef.current = null;
      setTooltip(null);
    }, 100);
  }

  return (
    <>
    <div
      ref={itemRef}
      className={`item${isCurrentClipboard ? " current-clipboard" : ""}`}
      onClick={() => onCopy(item.content)}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <span className={`text${isHidden ? " hidden" : ""}`}>{item.content}</span>
      {isConfirming ? (
        <span className="delete-confirm" onClick={(e) => e.stopPropagation()}>
          Delete?
          <button
            className="action confirm-yes"
            onClick={(e) => {
              e.stopPropagation();
              onConfirmDelete(item.id);
            }}
            title="Confirm delete"
          >
            &#x2713;
          </button>
          <button
            className="action confirm-no"
            onClick={(e) => {
              e.stopPropagation();
              onCancelDelete();
            }}
            title="Cancel"
          >
            &#x2715;
          </button>
        </span>
      ) : (
        <>
          <button
            className={`action${isPinned ? " starred" : ""}`}
            onClick={(e) => {
              e.stopPropagation();
              onPin(item.content);
            }}
            title="Pin"
          >
            &#x2605;
          </button>
          <button
            className="action"
            onClick={(e) => {
              e.stopPropagation();
              onRequestDelete(item.id);
            }}
            title="Delete"
          >
            &#x2715;
          </button>
        </>
      )}
    </div>
    {tooltip && ReactDOM.createPortal(
      <div
        className="hover-tooltip"
        style={{ top: tooltip.top, left: tooltip.left }}
        onMouseEnter={() => clearTimeout(hideRef.current)}
        onMouseLeave={() => setTooltip(null)}
      >
        {item.content}
      </div>,
      document.body
    )}
    </>
  );
}
