export default function HistoryItem({
    item,
    isCurrentClipboard,
    isPinned,
    confirmDeleteId,
    onCopy,
    onPin,
    onRequestDelete,
    onConfirmDelete,
    onCancelDelete,
}) {
    const isConfirming = confirmDeleteId === item.id;

    return (
        <div
            className={`item${isCurrentClipboard ? " current-clipboard" : ""}`}
            onClick={() => onCopy(item.content)}
        >
            <span className="text">{item.content}</span>
            {isConfirming ? (
                <span className="delete-confirm" onClick={e => e.stopPropagation()}>
                    Delete?
                    <button
                        className="action confirm-yes"
                        onClick={e => { e.stopPropagation(); onConfirmDelete(item.id); }}
                        title="Confirm delete"
                    >
                        &#x2713;
                    </button>
                    <button
                        className="action confirm-no"
                        onClick={e => { e.stopPropagation(); onCancelDelete(); }}
                        title="Cancel"
                    >
                        &#x2715;
                    </button>
                </span>
            ) : (
                <>
                    <button
                        className={`action${isPinned ? " starred" : ""}`}
                        onClick={(e) => { e.stopPropagation(); onPin(item.content); }}
                        title="Pin"
                    >
                        &#x2605;
                    </button>
                    <button
                        className="action"
                        onClick={(e) => { e.stopPropagation(); onRequestDelete(item.id); }}
                        title="Delete"
                    >
                        &#x2715;
                    </button>
                </>
            )}
        </div>
    );
}
