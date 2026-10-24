/** @vitest-environment jsdom */
import { act, cleanup, render } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
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
        return disconnected.obsStatus!();
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
