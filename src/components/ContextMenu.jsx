import { useEffect, useRef } from "react";
import ReactDOM from "react-dom";

export default function ContextMenu({ x, y, items, onClose }) {
  const menuRef = useRef(null);

  useEffect(() => {
    function handleMouseDown(e) {
      if (menuRef.current && !menuRef.current.contains(e.target)) onClose();
    }
    function handleKey(e) {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    }
    document.addEventListener("mousedown", handleMouseDown);
    document.addEventListener("keydown", handleKey, true);
    return () => {
      document.removeEventListener("mousedown", handleMouseDown);
      document.removeEventListener("keydown", handleKey, true);
    };
  }, [onClose]);

  return ReactDOM.createPortal(
    <div ref={menuRef} className="context-menu" style={{ top: y, left: x }}>
      {items.map((item, i) => (
        <div
          key={i}
          className="context-menu-item"
          onMouseDown={(e) => e.stopPropagation()}
          onClick={() => {
            item.onClick();
            onClose();
          }}
        >
          {item.label}
        </div>
      ))}
    </div>,
    document.body
  );
}
