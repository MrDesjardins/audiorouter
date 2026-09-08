import { describe, expect, it } from "vitest";
import type { JsonRpcRequest, JsonRpcResponse } from "@audiorouter/contracts";
import { createInitialBackend, WebView2RpcTransport, type WebView2Webview } from "./host";

const response: JsonRpcResponse = { jsonrpc: "2.0", id: 1, result: {} };

describe("native host bridge", () => {
  it("uses an injected typed transport and session identity", () => {
    const transport = { send: async (_request: JsonRpcRequest) => response };
    const backend = createInitialBackend({ transport, sessionId: "session-1" });
    expect(backend.connected).toBe(true);
  });

  it("fails closed to the disconnected preview for malformed injection", () => {
    expect(createInitialBackend(null).connected).toBe(false);
    expect(createInitialBackend({ transport: {}, sessionId: "session-1" }).connected).toBe(false);
    expect(createInitialBackend({ transport: { send: () => response }, sessionId: "" }).connected).toBe(false);
  });

  it("correlates bounded WebView2 requests and ignores unrelated messages", async () => {
    const listeners = new Set<(event: { data: unknown }) => void>();
    const sent: unknown[] = [];
    const webview: WebView2Webview = {
      postMessage: (message) => sent.push(message),
      addEventListener: (_type, listener) => { listeners.add(listener); },
      removeEventListener: (_type, listener) => { listeners.delete(listener); },
    };
    const transport = new WebView2RpcTransport(webview, 1000, 2);
    const pending = transport.send({ jsonrpc: "2.0", id: 7, method: "status.get" });
    expect(sent).toHaveLength(1);
    for (const listener of listeners) listener({ data: { type: "other" } });
    const matchingResponse = { ...response, id: 7 };
    for (const listener of listeners) listener({ data: { type: "audiorouter.rpc.response", response: matchingResponse } });
    await expect(pending).resolves.toEqual(matchingResponse);
    transport.dispose();
  });

  it("requires the configured WebView2 response origin", async () => {
    const listeners = new Set<(event: { data: unknown; origin?: string }) => void>();
    const webview: WebView2Webview = {
      postMessage: () => undefined,
      addEventListener: (_type, listener) => { listeners.add(listener); },
      removeEventListener: (_type, listener) => { listeners.delete(listener); },
    };
    const transport = new WebView2RpcTransport(webview, 1000, 2, "https://app.audiorouter.local");
    const pending = transport.send({ jsonrpc: "2.0", id: 9, method: "status.get" });
    const emit = (origin: string | undefined) => {
      for (const listener of listeners) listener({ origin, data: { type: "audiorouter.rpc.response", response: { ...response, id: 9 } } });
    };
    emit("https://untrusted.example");
    emit(undefined);
    let settled = false;
    void pending.then(() => { settled = true; });
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(settled).toBe(false);
    emit("https://app.audiorouter.local");
    await expect(pending).resolves.toEqual({ ...response, id: 9 });
    transport.dispose();
  });

  it("does not resolve requests from malformed or ambiguous responses", async () => {
    const listeners = new Set<(event: { data: unknown }) => void>();
    const webview: WebView2Webview = {
      postMessage: () => undefined,
      addEventListener: (_type, listener) => { listeners.add(listener); },
      removeEventListener: (_type, listener) => { listeners.delete(listener); },
    };
    const transport = new WebView2RpcTransport(webview, 20);
    const pending = transport.send({ jsonrpc: "2.0", id: 8, method: "status.get" });
    const emit = (response: unknown) => {
      for (const listener of listeners) listener({ data: { type: "audiorouter.rpc.response", response } });
    };
    emit({ jsonrpc: "2.0", id: 8, result: {}, error: { code: -1, message: "ambiguous" } });
    emit({ jsonrpc: "2.0", id: 8, error: { code: "-1", message: "wrong code" } });
    await expect(pending).rejects.toThrow("timed out");
    transport.dispose();
  });

  it("rejects unbounded or non-JSON-RPC outbound requests before posting", async () => {
    const posted: unknown[] = [];
    const webview: WebView2Webview = {
      postMessage: (message) => posted.push(message),
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
    };
    const transport = new WebView2RpcTransport(webview);
    await expect(transport.send({ jsonrpc: "2.0", id: 1.5, method: "status.get" } as JsonRpcRequest)).rejects.toThrow("request shape");
    await expect(transport.send({ jsonrpc: "2.0", id: 1, method: "x".repeat(257) })).rejects.toThrow("request shape");
    expect(posted).toHaveLength(0);
    transport.dispose();
  });

  it("rejects duplicate IDs and cleans pending requests on disposal", async () => {
    const listeners = new Set<(event: { data: unknown }) => void>();
    const webview: WebView2Webview = {
      postMessage: () => undefined,
      addEventListener: (_type, listener) => { listeners.add(listener); },
      removeEventListener: (_type, listener) => { listeners.delete(listener); },
    };
    const transport = new WebView2RpcTransport(webview);
    const first = transport.send({ jsonrpc: "2.0", id: 1, method: "status.get" });
    await expect(transport.send({ jsonrpc: "2.0", id: 1, method: "status.get" })).rejects.toThrow("already pending");
    transport.dispose();
    await expect(first).rejects.toThrow("disposed");
    expect(listeners).toHaveLength(0);
  });

  it("selects the WebView2 transport when the session ID is injected", () => {
    const webview: WebView2Webview = {
      postMessage: () => undefined,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
    };
    const backend = createInitialBackend(null, webview, "session-2", "https://app.audiorouter.local");
    expect(backend.connected).toBe(true);
    expect(createInitialBackend(null, webview, "").connected).toBe(false);
    expect(createInitialBackend(null, webview, "x".repeat(129)).connected).toBe(false);
    expect(createInitialBackend(null, webview, "session-2").connected).toBe(false);
  });
});
