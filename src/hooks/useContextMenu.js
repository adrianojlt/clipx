import { useState } from "react";

export function useContextMenu(sessionOptions) {

  const [contextMenu, setContextMenu] = useState(null);

  function onContextMenu(e) {

    if (sessionOptions.length === 0) {
        return;
    }

    e.preventDefault();
    e.stopPropagation();

    window.getSelection()?.removeAllRanges();

    setContextMenu({ x: e.clientX, y: e.clientY });
  }

  return { contextMenu, setContextMenu, onContextMenu };
}
