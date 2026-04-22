import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";

function App() {
  useEffect(() => {
    const onKey = async (e) => {
      if (e.key === "Escape") await getCurrentWindow().hide();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    const titleBar = document.querySelector(".title-bar");
    if (!titleBar) return;

    const onMouseDown = async () => {
      await getCurrentWindow().startDragging();
    };

    titleBar.addEventListener("mousedown", onMouseDown);
    return () => titleBar.removeEventListener("mousedown", onMouseDown);
  }, []);

  return (
    <main className="container">
      <div className="title-bar">
        <h1>Clipboard Manager</h1>
      </div>
    </main>
  );
}

export default App;
