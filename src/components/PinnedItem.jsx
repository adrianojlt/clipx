import { useState, useEffect, useRef } from "react";
import ContextMenu from "./ContextMenu";
import { useItemTooltip } from "../hooks/useItemTooltip";
import { useContextMenu } from "../hooks/useContextMenu";

export default function PinnedItem({
  item,
  index,
  isCurrentClipboard,
  isDragging,
  dragIndicator,
  sessions,
  onCopy,
  onMouseDown,
  onToggleHidden,
  onSaveDescription,
  onUnpin,
  onPinToSession,
}) {
  const [isEditing, setIsEditing] = useState(false);
  const [editValue, setEditValue] = useState(item.description);
  const [isConfirming, setIsConfirming] = useState(false);
  const itemRef = useRef(null);
  const sessionOptions = (sessions || []).filter((s) => !s.is_global);
  const { handleMouseEnter, handleMouseLeave, tooltipPortal } = useItemTooltip(itemRef, item.content);
  const { contextMenu, setContextMenu, onContextMenu } = useContextMenu(sessionOptions);

  useEffect(() => {
    setEditValue(item.description);
  }, [item.description]);

  return (
    <>
    <div data-id={item.id} className={`item-wrapper${isDragging ? " dragging" : ""}`}>
      {dragIndicator?.targetId === item.id && dragIndicator.position === "before" && (
        <div className="drop-indicator" />
      )}
      <div
        ref={itemRef}
        className={`item${isCurrentClipboard ? " current-clipboard" : ""}`}
        onClick={() => onCopy(item.content)}
        onMouseDown={(e) => { if (e.button === 2) e.preventDefault(); }}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
        onContextMenu={onContextMenu}
      >
        <span
          className="drag-handle"
          onMouseDown={(e) => onMouseDown(e, item.id)}
          title="Drag to reorder"
        >
          &#x2630;
        </span>
        <span className="item-number">{index}</span>
        <div className="pinned-text">
          {isEditing ? (
            <input
              className="description-edit"
              value={editValue}
              autoFocus
              onChange={(e) => setEditValue(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  onSaveDescription(item.id, editValue);
                  setIsEditing(false);
                }
                if (e.key === "Escape") {
                  e.stopPropagation();
                  setEditValue(item.description);
                  setIsEditing(false);
                }
              }}
              onClick={(e) => e.stopPropagation()}
            />
          ) : (
            <span className="description">{item.description}</span>
          )}
          <span className={`content-text${item.hidden ? " hidden" : ""}`}>{item.content}</span>
        </div>
        {!isConfirming && (
          <button
            className={`action eye-toggle${item.hidden ? " content-hidden" : ""}`}
            onClick={(e) => {
              e.stopPropagation();
              onToggleHidden(item.id);
            }}
            title={item.hidden ? "Show content" : "Hide content"}
          >
            {item.hidden ? "○" : "◉"}
          </button>
        )}
        {!isConfirming && (
          <button
            className="action"
            onClick={(e) => {
              e.stopPropagation();
              setEditValue(item.description);
              setIsEditing(true);
            }}
            title="Edit description"
          >
            {"✎"}
          </button>
        )}
        {isConfirming ? (
          <span className="delete-confirm" onClick={(e) => e.stopPropagation()}>
            Remove?
            <button
              className="action confirm-yes"
              onClick={(e) => {
                e.stopPropagation();
                onUnpin(item.id);
                setIsConfirming(false);
              }}
              title="Confirm remove"
            >
              &#x2713;
            </button>
            <button
              className="action confirm-no"
              onClick={(e) => {
                e.stopPropagation();
                setIsConfirming(false);
              }}
              title="Cancel"
            >
              &#x2715;
            </button>
          </span>
        ) : (
          <button
            className="action"
            onClick={(e) => {
              e.stopPropagation();
              setIsConfirming(true);
            }}
            title="Unpin"
          >
            &#x2715;
          </button>
        )}
      </div>
      {dragIndicator?.targetId === item.id && dragIndicator.position === "after" && (
        <div className="drop-indicator" />
      )}
    </div>
    {contextMenu && (
      <ContextMenu
        x={contextMenu.x}
        y={contextMenu.y}
        items={sessionOptions.map((s) => ({
          label: s.name,
          onClick: () => {
            onPinToSession(item.content, s.id, item.description);
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
