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

  return (
    <main className="container">
      <h1>Clipboard Manager</h1>
    </main>
  );
}

export default App;
