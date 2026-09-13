import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./styles.css";
import { App } from "./App";
import { createInitialBackend } from "./host";
import { invoke as tauriInvoke, isTauri } from "@tauri-apps/api/core";

const globalTauriCore = (window.__TAURI__ as { core?: unknown } | undefined)?.core;
const tauriCore: unknown = isTauri()
  ? { invoke: (command: string, args?: Record<string, unknown>) => tauriInvoke(command, args) }
  : globalTauriCore;

if (window.__AUDIO_ROUTER_FRONTEND_PROBE__) {
  void Promise.resolve()
    .then(() => tauriInvoke("rpc_request", {
      request: { jsonrpc: "2.0", id: "shell-probe", method: "system.describe" },
    }))
    .catch(() => undefined);
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App backend={createInitialBackend(window.__AUDIO_ROUTER_HOST__, tauriCore || window.chrome?.webview, window.__AUDIO_ROUTER_SESSION_ID__, window.location.origin)} />
  </StrictMode>,
);
