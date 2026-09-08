import type { JsonRpcRequest, JsonRpcResponse, RpcTransport } from "@audiorouter/contracts";
import { createDisconnectedBackend, createLiveBackendFromTransport, type UiBackend } from "./backend";

/** The narrow object a native shell may inject before loading the UI bundle. */
export type AudioRouterHostBridge = {
  transport: RpcTransport;
  sessionId: string;
};

export interface WebView2Webview {
  postMessage(message: unknown): void;
  addEventListener(type: "message", listener: (event: { data: unknown }) => void): void;
  removeEventListener(type: "message", listener: (event: { data: unknown }) => void): void;
}

type PendingRequest = {
  resolve: (response: JsonRpcResponse) => void;
  reject: (error: Error) => void;
  timeout: ReturnType<typeof setTimeout>;
};

function requestKey(id: JsonRpcRequest["id"]): string {
  return `${typeof id}:${String(id)}`;
}

/** Bounded JSON-RPC transport for a WebView2 `chrome.webview` bridge. */
export class WebView2RpcTransport implements RpcTransport {
  private readonly pending = new Map<string, PendingRequest>();
  private readonly listener: (event: { data: unknown }) => void;
  private closed = false;

  constructor(private readonly webview: WebView2Webview, private readonly timeoutMs = 5000, private readonly maxPending = 64) {
    if (!Number.isInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 60_000) throw new Error("WebView2 transport timeout is out of bounds");
    if (!Number.isInteger(maxPending) || maxPending < 1 || maxPending > 256) throw new Error("WebView2 transport pending limit is out of bounds");
    this.listener = (event) => this.receive(event.data);
    webview.addEventListener("message", this.listener);
  }

  send(request: JsonRpcRequest): Promise<JsonRpcResponse> {
    if (this.closed) return Promise.reject(new Error("WebView2 transport is closed"));
    if (!isWebView2Request(request)) return Promise.reject(new Error("WebView2 transport rejected the request shape"));
    if (this.pending.size >= this.maxPending) return Promise.reject(new Error("WebView2 transport pending request limit reached"));
    const key = requestKey(request.id);
    if (this.pending.has(key)) return Promise.reject(new Error("WebView2 request ID is already pending"));
    return new Promise<JsonRpcResponse>((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pending.delete(key);
        reject(new Error("WebView2 request timed out"));
      }, this.timeoutMs);
      this.pending.set(key, { resolve, reject, timeout });
      try {
        this.webview.postMessage({ type: "audiorouter.rpc.request", request });
      } catch (error) {
        clearTimeout(timeout);
        this.pending.delete(key);
        reject(error instanceof Error ? error : new Error("WebView2 request could not be sent"));
      }
    });
  }

  dispose(): void {
    if (this.closed) return;
    this.closed = true;
    this.webview.removeEventListener("message", this.listener);
    for (const [key, pending] of this.pending) {
      clearTimeout(pending.timeout);
      pending.reject(new Error("WebView2 transport disposed"));
      this.pending.delete(key);
    }
  }

  private receive(value: unknown): void {
    if (typeof value !== "object" || value === null || (value as { type?: unknown }).type !== "audiorouter.rpc.response") return;
    const response = (value as { response?: unknown }).response;
    if (typeof response !== "object" || response === null || (response as { jsonrpc?: unknown }).jsonrpc !== "2.0") return;
    const id = (response as { id?: unknown }).id;
    if ((typeof id !== "string" && typeof id !== "number") || !isJsonRpcResponse(response)) return;
    const key = requestKey(id);
    const pending = this.pending.get(key);
    if (!pending) return;
    this.pending.delete(key);
    clearTimeout(pending.timeout);
    pending.resolve(response as JsonRpcResponse);
  }
}

function isJsonRpcResponse(value: object): value is JsonRpcResponse {
  const hasResult = "result" in value;
  const error = (value as { error?: unknown }).error;
  const hasError = error !== undefined;
  if (hasResult === hasError) return false;
  if (!hasError) return true;
  if (typeof error !== "object" || error === null) return false;
  const candidate = error as { code?: unknown; message?: unknown };
  return typeof candidate.code === "number" && Number.isFinite(candidate.code) && typeof candidate.message === "string";
}

function isWebView2Request(value: unknown): value is JsonRpcRequest {
  if (typeof value !== "object" || value === null) return false;
  const request = value as { jsonrpc?: unknown; id?: unknown; method?: unknown };
  return request.jsonrpc === "2.0" &&
    typeof request.method === "string" &&
    request.method.length > 0 &&
    request.method.length <= 256 &&
    ((typeof request.id === "string" && request.id.length > 0 && request.id.length <= 128) ||
      (typeof request.id === "number" && Number.isFinite(request.id) && Number.isSafeInteger(request.id)));
}

declare global {
  interface Window {
    __AUDIO_ROUTER_HOST__?: unknown;
    __AUDIO_ROUTER_SESSION_ID__?: unknown;
    chrome?: { webview?: unknown };
  }
}

function isHostBridge(value: unknown): value is AudioRouterHostBridge {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as { transport?: unknown; sessionId?: unknown };
  return typeof candidate.sessionId === "string" &&
    candidate.sessionId.length > 0 &&
    candidate.sessionId.length <= 128 &&
    typeof candidate.transport === "object" &&
    candidate.transport !== null &&
    typeof (candidate.transport as { send?: unknown }).send === "function";
}

function isWebView2Webview(value: unknown): value is WebView2Webview {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Partial<WebView2Webview>;
  return typeof candidate.postMessage === "function" &&
    typeof candidate.addEventListener === "function" &&
    typeof candidate.removeEventListener === "function";
}

/** Select the injected native backend, or remain safely disconnected. */
export function createInitialBackend(host: unknown, webview: unknown = undefined, sessionId: unknown = undefined): UiBackend {
  if (isHostBridge(host)) return createLiveBackendFromTransport(host.transport, host.sessionId);
  if (isWebView2Webview(webview) && typeof sessionId === "string" && sessionId.length > 0 && sessionId.length <= 128) {
    return createLiveBackendFromTransport(new WebView2RpcTransport(webview), sessionId);
  }
  return createDisconnectedBackend();
}
