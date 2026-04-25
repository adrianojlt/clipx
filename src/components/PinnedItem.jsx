export default function PinnedItem({
    item,
    isCurrentClipboard,
    isDragging,
    dragIndicator,
    editingId,
    editingValue,
    confirmUnpinId,
    onCopy,
    onMouseDown,
    onToggleHidden,
    onStartEdit,
    onEditChange,
    onSaveEdit,
    onCancelEdit,
    onRequestUnpin,
    onConfirmUnpin,
    onCancelUnpin,
}) {
    const isEditing = editingId === item.id;
    const isConfirming = confirmUnpinId === item.id;

    return (
        <div
            data-id={item.id}
            className={`item-wrapper${isDragging ? " dragging" : ""}`}
        >
            {dragIndicator?.targetId === item.id && dragIndicator.position === "before" && (
                <div className="drop-indicator" />
            )}
            <div
                className={`item${isCurrentClipboard ? " current-clipboard" : ""}`}
                onClick={() => onCopy(item.content)}
            >
                <span
                    className="drag-handle"
                    onMouseDown={(e) => onMouseDown(e, item.id)}
                    title="Drag to reorder"
                >
                    &#x2630;
                </span>
                <div className="pinned-text">
                    {isEditing ? (
                        <input
                            className="description-edit"
                            value={editingValue}
                            autoFocus
                            onChange={e => onEditChange(e.target.value)}
                            onKeyDown={e => {
                                if (e.key === "Enter") onSaveEdit(item.id);
                                if (e.key === "Escape") { e.stopPropagation(); onCancelEdit(); }
                            }}
                            onClick={e => e.stopPropagation()}
                        />
                    ) : (
                        <span className="description">{item.description}</span>
                    )}
                    <span className={`content-text${item.hidden ? " hidden" : ""}`}>
                        {item.content}
                    </span>
                </div>
                {!isConfirming && (
                    <button
                        className={`action eye-toggle${item.hidden ? " content-hidden" : ""}`}
                        onClick={e => { e.stopPropagation(); onToggleHidden(item.id); }}
                        title={item.hidden ? "Show content" : "Hide content"}
                    >
                        {item.hidden ? "\u25CB" : "\u25C9"}
                    </button>
                )}
                {!isConfirming && (
                    <button
                        className="action"
                        onClick={e => { e.stopPropagation(); onStartEdit(item.id, item.description); }}
                        title="Edit description"
                    >
                        {"\u270E"}
                    </button>
                )}
                {isConfirming ? (
                    <span className="delete-confirm" onClick={e => e.stopPropagation()}>
                        Remove?
                        <button
                            className="action confirm-yes"
                            onClick={e => { e.stopPropagation(); onConfirmUnpin(item.id); }}
                            title="Confirm remove"
                        >
                            &#x2713;
                        </button>
                        <button
                            className="action confirm-no"
                            onClick={e => { e.stopPropagation(); onCancelUnpin(); }}
                            title="Cancel"
                        >
                            &#x2715;
                        </button>
                    </span>
                ) : (
                    <button
                        className="action"
                        onClick={(e) => { e.stopPropagation(); onRequestUnpin(item.id); }}
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
    );
}
