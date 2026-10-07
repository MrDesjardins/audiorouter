// Playwright dispatches this path to a disposable real Rust backend over
// stdio. This entry is excluded from the production Vite build.
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { createLiveBackendFromTransport } from "./backend";
import type { RpcTransport } from "@audiorouter/contracts";
import "./styles.css";

const transport: RpcTransport = {
  async send(request) {
    const response = await fetch("/__e2e_rpc", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
    });
    if (!response.ok) throw new Error(`Offline test transport failed (${response.status})`);
    return response.json();
  },
};
createRoot(document.getElementById("root")!).render(
  <App backend={createLiveBackendFromTransport(transport, "e2e-session")} />,
);
