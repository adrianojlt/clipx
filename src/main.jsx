import React from "react";
import ReactDOM from "react-dom/client";
import "./theme.css";
import App from "./App";
import "./styles.css";
import { applyTheme } from "./theme";

async function bootstrap() {
  await applyTheme();
  ReactDOM.createRoot(document.getElementById("root")).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
}

bootstrap();
