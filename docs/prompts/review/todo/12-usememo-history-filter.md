# Task 12 - Memoize the History Search Filter

**Severity:** Low
**Category:** React / Performance
**Depends on:** Nothing - independent, very small change

## Why This Is a Problem

In App.jsx, the history search filter runs on every render:

```js
history.filter(item =>
    item.content.toLowerCase().includes(historySearch.toLowerCase())
)
```

This recalculates even when neither `history` nor `historySearch` changed - for example when the drag indicator position updates during pinned item reordering. For small history lists this is negligible. As history grows it wastes work unnecessarily.

`useMemo` caches the result and only recalculates when the dependencies change.

## Files to Touch

- `src/App.jsx`

## Exact Change

Add `useMemo` to the existing React import (it is probably not imported yet):

```js
import React, { useState, useEffect, useRef, useMemo } from "react";
```

Find where the filter is used in the JSX (likely inside the history list render). Extract it to a memoized value at the top of the component:

```js
const filteredHistory = useMemo(
    () => history.filter(item =>
        item.content.toLowerCase().includes(historySearch.toLowerCase())
    ),
    [history, historySearch]
);
```

Then in the JSX, replace the inline filter call with `filteredHistory`.

## How to Verify

```bash
pnpm dev
```

Search in the history tab must work identically. Type in the search box - the list should filter as you type. Clear the search - all items return. No visual changes.
