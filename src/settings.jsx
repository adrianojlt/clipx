import React from "react";
import ReactDOM from "react-dom/client";
import "./theme.css";
import Settings from "./components/Settings";
import "./styles.css";
import { applyTheme } from "./theme";

async function bootstrap() {
  await applyTheme();
  ReactDOM.createRoot(document.getElementById("root")).render(
    <React.StrictMode>
      <Settings />
    </React.StrictMode>
  );
}

bootstrap();
