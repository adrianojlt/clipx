import { useRef } from "react";
import ContextMenu from "./ContextMenu";
import { useItemTooltip } from "../hooks/useItemTooltip";
import { useContextMenu } from "../hooks/useContextMenu";

export default function HistoryItem({
  item,
  index,
  isCurrentClipboard,
  isPinned,
  isHidden,
  confirmDeleteId,
  sessions,
  onCopy,
  onPin,
  onPinToSession,
  onRequestDelete,
  onConfirmDelete,
  onCancelDelete,
}) {
  const isConfirming = confirmDeleteId === item.id;
  const itemRef = useRef(null);
  const sessionOptions = (sessions || []).filter((s) => !s.is_global);
  const { handleMouseEnter, handleMouseLeave, tooltipPortal } = useItemTooltip(itemRef, item.content);
  const { contextMenu, setContextMenu, onContextMenu } = useContextMenu(sessionOptions);

  return (
    <>
    <div
      ref={itemRef}
      className={`item${isCurrentClipboard ? " current-clipboard" : ""}`}
      style={{ position: "relative" }}
      onClick={() => onCopy(item.content)}
      onMouseDown={(e) => { if (e.button === 2) e.preventDefault(); }}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      onContextMenu={onContextMenu}
    >
      <span className="item-number">{index}</span>
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
            title="Pin to Favorites"
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
    {contextMenu && (
      <ContextMenu
        x={contextMenu.x}
        y={contextMenu.y}
        items={sessionOptions.map((s) => ({
          label: s.name,
          onClick: () => {
            onPinToSession(item.content, s.id);
            setContextMenu(null);
          },
        }))}
        onClose={() => setContextMenu(null)}
      />
    )}
    {tooltipPortal}
    </>
  );
}
