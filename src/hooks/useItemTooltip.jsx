import { useState, useRef, useEffect } from "react";
import ReactDOM from "react-dom";

const TOOLTIP_GAP = 4;
const TOOLTIP_EST_WIDTH = 328;
const TOOLTIP_EST_HEIGHT = 120;

export function useItemTooltip(itemRef, content) {

  const [tooltip, setTooltip] = useState(null);
  const timerRef = useRef(null);
  const hideRef = useRef(null);

  useEffect(() => {
    return () => {
      clearTimeout(timerRef.current);
      clearTimeout(hideRef.current);
    };
  }, []);

  function handleMouseEnter() {

    clearTimeout(hideRef.current);

    if (timerRef.current) return;

    timerRef.current = setTimeout(() => {

      timerRef.current = null;

      if (!itemRef.current) return;

      const rect = itemRef.current.getBoundingClientRect();

      const top = rect.bottom + TOOLTIP_EST_HEIGHT > window.innerHeight
        ? rect.top - TOOLTIP_EST_HEIGHT - TOOLTIP_GAP
        : rect.bottom + TOOLTIP_GAP;

      const left = Math.min(rect.left, window.innerWidth - TOOLTIP_EST_WIDTH);

      setTooltip({ top, left });
    }, 2000);
  }

  function handleMouseLeave() {

    hideRef.current = setTimeout(() => {

      clearTimeout(timerRef.current);
      timerRef.current = null;

      setTooltip(null);
    }, 100);
  }

  const tooltipPortal = tooltip && ReactDOM.createPortal(
    <div
      className="hover-tooltip"
      style={{ top: tooltip.top, left: tooltip.left }}
      onMouseEnter={() => clearTimeout(hideRef.current)}
      onMouseLeave={() => setTooltip(null)}
    >
      {content}
    </div>,
    document.body
  );

  return { handleMouseEnter, handleMouseLeave, tooltipPortal };
}
