import { afterEach, describe, expect, it, vi } from "vitest";
import { AudioRouterRpcError, createAudioRouterClient, isValidCorrelationId, newCorrelationId, type JsonRpcRequest } from "@audiorouter/contracts";
import { createLiveBackendFromTransport } from "./backend";
import { TauriRpcTransport } from "./host";
import { formatRpcFailure, onRpcFailure, reportRpcFailure, resetRpcFailureRepeats } from "./clientDiagnostics";

afterEach(() => resetRpcFailureRepeats());

describe("request correlation IDs (P2-3)", () => {
  it("generates short Crockford IDs and validates the shared charset", () => {
    const id = newCorrelationId();
    expect(id).toMatch(/^[0-9A-HJKMNP-TV-Z]{8}$/);
    expect(newCorrelationId(() => new Uint8Array(8))).toBe("00000000");
    expect(isValidCorrelationId("Deck-42")).toBe(true);
    for (const bad of ["", "x".repeat(33), "has space", "under_score", 7, null]) expect(isValidCorrelationId(bad)).toBe(false);
  });

  it("sends a fresh requestId with every request and names it on failure", async () => {
    const sent: JsonRpcRequest[] = [];
    const failures: unknown[] = [];
    const ids = ["AAAA1111", "BBBB2222", "bad id"];
    const client = createAudioRouterClient({
      async send(request) {
        sent.push(request);
        if (request.method === "sessions.play") return { jsonrpc: "2.0", id: request.id ?? null, error: { code: -32001, message: "private detail C:\\Users\\x", data: { code: "permissionDenied", fieldPath: null, resourceIds: [], retryable: false, remediation: "" } } };
        if (request.method === "status.get" && request.requestId !== "AAAA1111") throw new Error("pipe closed");
        return { jsonrpc: "2.0", id: request.id ?? null, result: {} };
      },
    }, { newRequestId: () => ids.shift() ?? "CCCC3333", onRequestFailed: (failure) => failures.push(failure) });
    await client.request("status.get", undefined);
    expect(sent[0].requestId).toBe("AAAA1111");
    const error = await client.request("sessions.play", { sessionId: "s" } as never).catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(AudioRouterRpcError);
    expect((error as AudioRouterRpcError).requestId).toBe("BBBB2222");
    expect(sent[1].requestId).toBe("BBBB2222");
    // An invalid generated ID is replaced, and a transport failure is reported without a code.
    await expect(client.request("status.get", undefined)).rejects.toThrow("pipe closed");
    expect(isValidCorrelationId(sent[2].requestId)).toBe(true);
    expect(sent[2].requestId).not.toBe("bad id");
    expect(failures).toEqual([
      { method: "sessions.play", requestId: "BBBB2222", code: -32001, kind: "permissionDenied" },
      { method: "status.get", requestId: sent[2].requestId },
    ]);
    expect(JSON.stringify(failures)).not.toContain("private");
  });

  it("passes the requestId through the desktop shell bridge unchanged", async () => {
    const invoke = vi.fn(async (_command: string, args?: Record<string, unknown>) => ({ jsonrpc: "2.0", id: (args?.request as JsonRpcRequest).id, result: {} }));
    const client = createAudioRouterClient(new TauriRpcTransport({ invoke }), { newRequestId: () => "TAURI-1" });
    await client.request("status.get", undefined);
    expect(invoke).toHaveBeenCalledWith("rpc_request", { request: expect.objectContaining({ method: "status.get", requestId: "TAURI-1" }) });
  });

  it("records failed live-backend requests as client diagnostic rows with the ID", async () => {
    const rows: string[] = [];
    const stop = onRpcFailure((row) => rows.push(row));
    let seen: JsonRpcRequest | undefined;
    const backend = createLiveBackendFromTransport({
      async send(request) {
        seen = request;
        return { jsonrpc: "2.0", id: request.id ?? null, error: { code: -32000, message: "C:\\private", data: { code: "unavailable", fieldPath: null, resourceIds: [], retryable: true, remediation: "" } } };
      },
    }, "session");
    await expect(backend.listClients()).rejects.toBeInstanceOf(AudioRouterRpcError);
    stop();
    expect(isValidCorrelationId(seen?.requestId)).toBe(true);
    expect(rows).toEqual([`RPC failed: clients.list (unavailable) [req ${seen?.requestId}]`]);
  });

  it("formats rows without error text and collapses repeats", () => {
    expect(formatRpcFailure({ method: "graph.commit", requestId: "K7Q2M9XD", code: -32602 })).toBe("RPC failed: graph.commit (code -32602) [req K7Q2M9XD]");
    expect(formatRpcFailure({ method: "C:\\x y", requestId: "no good", code: 1, kind: "<script>" })).toBe("RPC failed: request (code 1)");
    expect(formatRpcFailure({ method: "status.get", requestId: "A1" })).toBe("RPC failed: status.get (no response) [req A1]");
    const listener = vi.fn();
    const stop = onRpcFailure(listener);
    reportRpcFailure({ method: "system.diagnostics", requestId: "A1", code: -32000 }, 1_000);
    reportRpcFailure({ method: "system.diagnostics", requestId: "A2", code: -32000 }, 2_000);
    reportRpcFailure({ method: "system.diagnostics", requestId: "A3", code: -32000 }, 40_000);
    stop();
    reportRpcFailure({ method: "graph.commit", requestId: "A4", code: -32000 }, 50_000);
    expect(listener.mock.calls.map(([row]) => row)).toEqual([
      "RPC failed: system.diagnostics (code -32000) [req A1]",
      "RPC failed: system.diagnostics (code -32000) [req A3]",
    ]);
  });
});
