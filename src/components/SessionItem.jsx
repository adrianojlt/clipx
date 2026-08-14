import { useState, useEffect } from "react";
import ContextMenu from "./ContextMenu";
import { useContextMenu } from "../hooks/useContextMenu";

export default function SessionItem({
  item,
  index,
  isActive,
  isDragging,
  dragIndicator,
  confirmDeleteId,
  onActivate,
  onMouseDown,
  onRename,
  onRequestDelete,
  onConfirmDelete,
  onCancelDelete,
}) {
  const isConfirming = confirmDeleteId === item.id;
  const [isEditing, setIsEditing] = useState(false);
  const [editValue, setEditValue] = useState(item.name);
  const menuItems = item.is_global
    ? []
    : [{ label: "Rename", onClick: () => { setEditValue(item.name); setIsEditing(true); } }];
  const { contextMenu, setContextMenu, onContextMenu } = useContextMenu(menuItems);

  useEffect(() => {
    setEditValue(item.name);
  }, [item.name]);

  const saveName = () => {
    const name = editValue.trim();
    setIsEditing(false);
    if (!name || name === item.name) {
      setEditValue(item.name);
      return;
    }
    onRename(item.id, name);
  };

  return (
    <>
    <div data-id={item.id} className={`item-wrapper${isDragging ? " dragging" : ""}`}>
      {dragIndicator?.targetId === item.id && dragIndicator.position === "before" && (
        <div className="drop-indicator" />
      )}
      <div
        className={`item${isActive ? " session-active" : ""}`}
        onClick={() => { if (!isEditing) onActivate(item.id); }}
        onMouseDown={(e) => { if (e.button === 2) e.preventDefault(); }}
        onContextMenu={onContextMenu}
      >
        {item.is_global ? (
          <span className="drag-handle inert" title="Favorites cannot be reordered">
            &#x2630;
          </span>
        ) : (
          <span
            className="drag-handle"
            onMouseDown={(e) => onMouseDown(e, item.id)}
            title="Drag to reorder"
          >
            &#x2630;
          </span>
        )}
        <span className="item-number">{index}</span>
        {isEditing ? (
          <input
            className="session-name-edit"
            value={editValue}
            autoFocus
            onChange={(e) => setEditValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") saveName();
              if (e.key === "Escape") {
                e.stopPropagation();
                setEditValue(item.name);
                setIsEditing(false);
              }
            }}
            onBlur={saveName}
            onClick={(e) => e.stopPropagation()}
          />
        ) : (
          <span className="session-name">{item.name}</span>
        )}
        <span className="session-count">{item.item_count}</span>
        {isConfirming ? (
          <span className="delete-confirm" onClick={(e) => e.stopPropagation()}>
            Remove?
            <button
              className="action confirm-yes"
              onClick={(e) => {
                e.stopPropagation();
                onConfirmDelete(item.id);
              }}
              title="Confirm remove"
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
              className={`action session-activate${isActive ? " active" : ""}`}
              onClick={(e) => {
                e.stopPropagation();
                onActivate(item.id);
              }}
            >
              &#x25B6;
            </button>
            {item.is_global ? (
              <button
                className="action inert"
                disabled
                aria-label="Favorites cannot be deleted"
                onClick={(e) => e.stopPropagation()}
                title="Favorites cannot be deleted"
              >
                &#x2715;
              </button>
            ) : (
              <button
                className="action"
                onClick={(e) => {
                  e.stopPropagation();
                  onRequestDelete(item.id);
                }}
                title="Delete session"
              >
                &#x2715;
              </button>
            )}
          </>
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
        items={menuItems}
        onClose={() => setContextMenu(null)}
      />
    )}
    </>
  );
}
