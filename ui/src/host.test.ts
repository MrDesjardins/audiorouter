import { describe, expect, it } from "vitest";
import type { JsonRpcRequest, JsonRpcResponse } from "@audiorouter/contracts";
import { createInitialBackend } from "./host";

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
});
