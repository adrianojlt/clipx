# Task 10 - Extract HistoryItem Component

**Severity:** High
**Category:** React / Component Size
**Depends on:** Task 09 (service layer) - strongly recommended first

## Why This Is a Problem

App.jsx renders the history list entirely inline. Each item's JSX - with its pin button, delete button, delete confirmation, and highlighted current clipboard content - is embedded in the parent component. This makes App.jsx harder to read and the item rendering impossible to test in isolation.

## Files to Touch

- `src/components/HistoryItem.jsx` (create this new file)
- `src/components/HistoryItem.css` (create if you want to move item-specific styles)
- `src/App.jsx` (replace inline JSX with `<HistoryItem />`)

## What HistoryItem Should Receive as Props

```js
// Props for HistoryItem:
{
    item,                    // { id, content, created_at }
    isCurrentClipboard,      // boolean - whether to highlight this item
    confirmDeleteId,         // the id currently awaiting confirmation (or null)
    onCopy,                  // (content) => void
    onPin,                   // (content) => void
    onRequestDelete,         // (id) => void  - shows confirmation
    onConfirmDelete,         // (id) => void  - actual delete
    onCancelDelete,          // () => void
}
```

## Step 1 - Create src/components/HistoryItem.jsx

```jsx
import React from "react";

export default function HistoryItem({
    item,
    isCurrentClipboard,
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
            className={`item ${isCurrentClipboard ? "current-clipboard" : ""}`}
            onClick={() => onCopy(item.content)}
        >
            <span className="content-text">{item.content}</span>
            <div className="item-actions">
                {isConfirming ? (
                    <>
                        <button onClick={(e) => { e.stopPropagation(); onConfirmDelete(item.id); }}>
                            Confirm
                        </button>
                        <button onClick={(e) => { e.stopPropagation(); onCancelDelete(); }}>
                            Cancel
                        </button>
                    </>
                ) : (
                    <>
                        <button onClick={(e) => { e.stopPropagation(); onPin(item.content); }}>
                            Pin
                        </button>
                        <button onClick={(e) => { e.stopPropagation(); onRequestDelete(item.id); }}>
                            Delete
                        </button>
                    </>
                )}
            </div>
        </div>
    );
}
```

Match the exact JSX and class names from App.jsx - do not change appearance.

## Step 2 - Update App.jsx

In the history list rendering, replace the inline item JSX with:

```jsx
import HistoryItem from "./components/HistoryItem";

// Inside the list:
{filteredHistory.map(item => (
    <HistoryItem
        key={item.id}
        item={item}
        isCurrentClipboard={item.content === currentClipboard}
        confirmDeleteId={confirmDeleteHistoryId}
        onCopy={handleCopy}
        onPin={handlePin}
        onRequestDelete={(id) => setConfirmDeleteHistoryId(id)}
        onConfirmDelete={handleDeleteHistory}
        onCancelDelete={() => setConfirmDeleteHistoryId(null)}
    />
))}
```

## How to Verify

```bash
pnpm dev
```

History tab must look and behave identically. Test: copy items, see them appear, click to copy back, pin one, delete one with confirmation. No visual changes expected.
