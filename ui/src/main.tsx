import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./styles.css";
import { App } from "./App";
import { createInitialBackend } from "./host";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App backend={createInitialBackend(window.__AUDIO_ROUTER_HOST__, window.chrome?.webview, window.__AUDIO_ROUTER_SESSION_ID__)} />
  </StrictMode>,
);
