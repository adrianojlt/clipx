# Task 11 - Extract PinnedItem Component

**Severity:** High
**Category:** React / Component Size
**Depends on:** Task 09 (service layer) - strongly recommended first. Task 10 optional but good to do before this.

## Why This Is a Problem

The pinned item rendering in App.jsx is the most complex part of the UI. Each pinned item has a drag handle, description display, hidden content toggle, inline edit mode, unpin confirmation, and delete confirmation. All of this is rendered inline in the parent component, making App.jsx hard to read and the pinned item logic impossible to reason about in isolation.

## Files to Touch

- `src/components/PinnedItem.jsx` (create this new file)
- `src/App.jsx` (replace inline JSX with `<PinnedItem />`)

## What PinnedItem Should Receive as Props

```js
{
    item,                    // { id, content, description, hidden, created_at }
    dragIndicator,           // { position, id } or null - for drop indicator rendering
    editingId,               // id of item currently being edited (or null)
    editingValue,            // current text in the description edit input
    confirmUnpinId,          // id awaiting unpin confirmation (or null)
    onCopy,                  // (content) => void
    onMouseDown,             // (e, id) => void - starts drag
    onToggleHidden,          // (id) => void
    onStartEdit,             // (id, currentDescription) => void
    onEditChange,            // (newValue) => void
    onSaveEdit,              // (id) => void
    onCancelEdit,            // () => void
    onRequestUnpin,          // (id) => void
    onConfirmUnpin,          // (id) => void
    onCancelUnpin,           // () => void
}
```

## Step 1 - Create src/components/PinnedItem.jsx

Extract the pinned item JSX from App.jsx into this component. The component handles:
- Drag handle (mousedown to start drag)
- Drop indicator (shown above or below based on dragIndicator)
- Description display (or edit input when editingId matches)
- Hidden content display (blurred or shown based on item.hidden)
- Action buttons: toggle hidden, edit, unpin/confirm, delete

Match the exact class names and structure from App.jsx - do not change appearance.

## Step 2 - Update App.jsx

In the pinned list rendering, replace the inline item JSX with:

```jsx
import PinnedItem from "./components/PinnedItem";

// Inside the pinned list:
{pinned.map(item => (
    <PinnedItem
        key={item.id}
        item={item}
        dragIndicator={dragIndicator}
        editingId={editingId}
        editingValue={editingValue}
        confirmUnpinId={confirmUnpinId}
        onCopy={handleCopy}
        onMouseDown={handleMouseDown}
        onToggleHidden={handleToggleHidden}
        onStartEdit={(id, desc) => { setEditingId(id); setEditingValue(desc); }}
        onEditChange={setEditingValue}
        onSaveEdit={handleSaveDescription}
        onCancelEdit={() => { setEditingId(null); setEditingValue(""); }}
        onRequestUnpin={(id) => setConfirmUnpinId(id)}
        onConfirmUnpin={handleUnpin}
        onCancelUnpin={() => setConfirmUnpinId(null)}
    />
))}
```

## How to Verify

```bash
pnpm dev
```

Pinned tab must look and behave identically. Test:
- Drag to reorder items
- Click to copy a pinned item
- Edit a description and save
- Toggle content hidden/visible
- Unpin with confirmation
- Cancel unpin confirmation

No visual changes expected.
