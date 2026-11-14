/** @vitest-environment jsdom */
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { AudioRouterRpcError } from "@audiorouter/contracts";
import { App } from "./App";
import { createDisconnectedBackend } from "./backend";

beforeAll(() => {
  Object.defineProperty(window, "DOMMatrixReadOnly", {
    configurable: true,
    value: class {
      m22 = 1;
    },
  });
  Object.defineProperty(globalThis, "ResizeObserver", {
    configurable: true,
    value: class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  });
});
afterEach(() => {
  cleanup();
  window.localStorage.clear();
});

const later = <T,>(value: () => T, ms = 20) => new Promise<T>((resolve) => setTimeout(() => resolve(value()), ms));

describe("start-up order", () => {
  it("copies a startup error before a session can load", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    const original = Object.getOwnPropertyDescriptor(navigator, "clipboard");
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    try {
      render(
        <App
          backend={{
            ...createDisconnectedBackend(),
            connected: true,
            snapshot: vi
              .fn()
              .mockRejectedValue(new AudioRouterRpcError({ code: -32000, message: "Worker unavailable" })),
          }}
        />,
      );
      const copy = await screen.findByRole("button", { name: "Copy error" });
      fireEvent.click(copy);
      await screen.findByRole("button", { name: "Copied" });
      expect(writeText).toHaveBeenCalledWith("Unable to load your saved session: Worker unavailable");
    } finally {
      if (original) Object.defineProperty(navigator, "clipboard", original);
      else Reflect.deleteProperty(navigator, "clipboard");
    }
  });
  // The backend serves one request at a time; anything sent before the
  // snapshot delays the first screen.
  it.each([
    ["a remembered session", true],
    ["a first launch", false],
  ])("sends no app or OBS check before the first snapshot, with %s", async (_label, remembered) => {
    if (remembered) window.localStorage.setItem("audiorouter.ui.last-session", "desktop-session");
    const disconnected = createDisconnectedBackend();
    const initial = await disconnected.snapshot();
    const calls: string[] = [];
    const backend = {
      ...disconnected,
      connected: true,
      getActiveSession: vi.fn(() => later(() => ({ sessionId: initial.session.id }))),
      snapshot: vi.fn(() => {
        calls.push("snapshot");
        return later(() => {
          calls.push("snapshot answered");
          return structuredClone(initial);
        });
      }),
      listApplications: vi.fn(() => {
        calls.push("apps.list");
        return later(() => []);
      }),
      obsStatus: vi.fn(() => {
        calls.push("obs.status");
        return Promise.resolve({
          installed: false,
          connection: "off" as const,
          source: null,
          sceneCollection: null,
          inputs: [],
          passwordSource: null,
          lastChange: { state: "none" as const, changed: 0, restorable: false, error: null },
        });
      }),
    };
    await act(async () => {
      render(<App backend={backend} />);
    });
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 200));
    });
    const answered = calls.indexOf("snapshot answered");
    expect(answered).toBeGreaterThan(0);
    expect(calls.indexOf("apps.list")).toBeGreaterThan(answered);
    expect(calls.indexOf("obs.status")).toBeGreaterThan(answered);
  });
});
