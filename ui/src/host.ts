import type { RpcTransport } from "@audiorouter/contracts";
import { createDisconnectedBackend, createLiveBackendFromTransport, type UiBackend } from "./backend";

/** The narrow object a native shell may inject before loading the UI bundle. */
export type AudioRouterHostBridge = {
  transport: RpcTransport;
  sessionId: string;
};

declare global {
  interface Window {
    __AUDIO_ROUTER_HOST__?: unknown;
  }
}

function isHostBridge(value: unknown): value is AudioRouterHostBridge {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as { transport?: unknown; sessionId?: unknown };
  return typeof candidate.sessionId === "string" &&
    candidate.sessionId.length > 0 &&
    typeof candidate.transport === "object" &&
    candidate.transport !== null &&
    typeof (candidate.transport as { send?: unknown }).send === "function";
}

/** Select the injected native backend, or remain safely disconnected. */
export function createInitialBackend(host: unknown): UiBackend {
  if (!isHostBridge(host)) return createDisconnectedBackend();
  return createLiveBackendFromTransport(host.transport, host.sessionId);
}
