import { useState, useRef, useEffect, useCallback } from "react";
import { logError } from "../services/clipboardService";

export function useDragReorder({ listRef, itemsRef, setItems, reorderFn }) {
  const [draggingId, setDraggingId] = useState(null);
  const [dragIndicator, setDragIndicator] = useState(null);
  const cleanupRef = useRef(null);

  useEffect(() => {
    return () => {
      if (cleanupRef.current) {
        cleanupRef.current();
        cleanupRef.current = null;
      }
    };
  }, []);

  const handleMouseDown = useCallback((e, id) => {

    e.preventDefault();
    e.stopPropagation();

    setDraggingId(id);

    let currentIndicator = null;

    const onMouseMove = (ev) => {

      if (!listRef.current) return;

      const listRect = listRef.current.getBoundingClientRect();
      const relY = ev.clientY - listRect.top + listRef.current.scrollTop;
      const children = Array.from(listRef.current.children);

      let closest = null;
      let closestPos = "after";
      let minDist = Infinity;

      for (const child of children) {

        const rect = child.getBoundingClientRect();
        const childTop = rect.top - listRect.top + listRef.current.scrollTop;
        const childCenter = childTop + rect.height / 2;
        const dist = Math.abs(relY - childCenter);

        if (dist < minDist) {
          minDist = dist;
          closest = child;
          closestPos = relY < childCenter ? "before" : "after";
        }
      }

      if (closest) {
        const targetId = Number(closest.dataset.id);
        if (targetId !== id) {
          currentIndicator = { targetId, position: closestPos };
          setDragIndicator(currentIndicator);
        } else {
          currentIndicator = null;
          setDragIndicator(null);
        }
      }
    };

    const onMouseUp = async () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
      cleanupRef.current = null;

      const currentItems = itemsRef.current;
      const draggedIndex = currentItems.findIndex((item) => item.id === id);

      if (draggedIndex === -1) {
        setDraggingId(null);
        setDragIndicator(null);
        return;
      }

      let targetIndex = draggedIndex;

      if (currentIndicator) {
        targetIndex = currentItems.findIndex((item) => item.id === currentIndicator.targetId);
        if (targetIndex !== -1 && currentIndicator.position === "after") targetIndex += 1;
      }

      if (targetIndex !== draggedIndex && targetIndex !== -1) {

        const newItems = [...currentItems];
        const [draggedItem] = newItems.splice(draggedIndex, 1);
        const insertIndex = draggedIndex < targetIndex ? targetIndex - 1 : targetIndex;

        newItems.splice(insertIndex, 0, draggedItem);

        setItems(newItems);

        itemsRef.current = newItems;

        try {
          const ids = newItems.map((item) => item.id);
          await reorderFn(ids);
        } catch (e) {
          await logError("error", `Failed to reorder: ${e}`);
        }
      }

      setDraggingId(null);
      setDragIndicator(null);
    };

    cleanupRef.current = () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
    };
    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  }, [listRef, itemsRef, setItems, reorderFn]);

  return { draggingId, dragIndicator, handleMouseDown };
}
