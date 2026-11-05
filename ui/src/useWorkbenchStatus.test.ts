/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { act, cleanup, renderHook } from "@testing-library/react";
import { useWorkbenchStatus } from "./useWorkbenchStatus";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  delete (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
});

it("records one safe category per failure run and retains the last status", async () => {
  vi.useFakeTimers();
  let fail = false;
  const tunnel = { status: "connected", tunnelId: "private-id" };
  const invoke = vi.fn((command: string) => {
    if (fail) return Promise.reject(new TypeError("private path and credentials"));
    return Promise.resolve(command === "mcp_tunnel_status" ? tunnel : []);
  });
  Object.assign(window, { __TAURI_INTERNALS__: { invoke } });
  const diagnostic = vi.fn();
  const { result, unmount } = renderHook(() => useWorkbenchStatus("mcp", diagnostic));
  await act(async () => {});
  expect(result.current.mcpTunnelStatus).toEqual(tunnel);
  fail = true;
  await act(async () => {
    await vi.advanceTimersByTimeAsync(4000);
  });
  expect(result.current.mcpTunnelStatus).toEqual(tunnel);
  expect(diagnostic.mock.calls).toEqual([
    ["MCP activity read failed (TypeError)"],
    ["MCP tunnel status read failed (TypeError)"],
    ["Backend activity read failed (TypeError)"],
  ]);
  fail = false;
  await act(async () => {
    await vi.advanceTimersByTimeAsync(2000);
  });
  fail = true;
  await act(async () => {
    await vi.advanceTimersByTimeAsync(2000);
  });
  expect(diagnostic).toHaveBeenCalledTimes(6);
  unmount();
  await act(async () => {
    await vi.advanceTimersByTimeAsync(4000);
  });
  expect(diagnostic).toHaveBeenCalledTimes(6);
});

it("reports every initial failure without leaking an unsafe error name", async () => {
  const error = new Error("private path and credentials");
  error.name = "C:/private/secret";
  Object.assign(window, { __TAURI_INTERNALS__: { invoke: () => Promise.reject(error) } });
  const diagnostic = vi.fn();
  renderHook(() => useWorkbenchStatus("diagnostics", diagnostic));
  await act(async () => {});
  expect(new Set(diagnostic.mock.calls.map(([message]) => message))).toEqual(
    new Set([
      "MCP setup read failed (Error)",
      "MCP tunnel status read failed (Error)",
      "Backend activity read failed (Error)",
      "MCP activity read failed (Error)",
    ]),
  );
  expect(diagnostic).toHaveBeenCalledTimes(4);
});

it("ignores rejections after switching away", async () => {
  let reject!: (error: Error) => void;
  const pending = new Promise((_resolve, rejectPromise) => {
    reject = rejectPromise;
  });
  Object.assign(window, { __TAURI_INTERNALS__: { invoke: () => pending } });
  const diagnostic = vi.fn();
  const { rerender } = renderHook(({ tab }) => useWorkbenchStatus(tab, diagnostic), {
    initialProps: { tab: "mcp" as "mcp" | "tools" },
  });
  rerender({ tab: "tools" });
  await act(async () => {
    reject(new Error("private"));
  });
  expect(diagnostic).not.toHaveBeenCalled();
});
