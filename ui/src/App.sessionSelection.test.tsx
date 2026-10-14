/** @vitest-environment jsdom */
import { act, cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import type { EventsSubscribeResult, Session, StateEvent } from "@audiorouter/contracts";
import { App } from "./App";
import { createDisconnectedBackend } from "./backend";
import { demoSession } from "./fixtures";

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

const later = <T,>(value: () => T, ms = 4) => new Promise<T>((resolve) => setTimeout(() => resolve(value()), ms));

/**
 * A backend like the real one: the shell's default session is
 * `desktop-session`, the database remembers another active session, every
 * `sessions.active.set` emits `session.selectionChanged`, and calls take a
 * few milliseconds.
 */
async function backendWithSavedActiveSession() {
  const disconnected = createDisconnectedBackend();
  const initial = await disconnected.snapshot();
  const sessions: Record<string, Session> = {
    "desktop-session": { ...structuredClone(demoSession), id: "desktop-session", name: "Desktop session" },
    "main-session": { ...structuredClone(demoSession), id: "main-session", name: "Main session", revision: 50 },
  };
  let active = "main-session";
  let sequence = 10;
  const events: StateEvent[] = [];
  const backend = {
    ...disconnected,
    connected: true,
    snapshot: vi.fn((sessionId = "desktop-session") =>
      later(() =>
        structuredClone({
          ...initial,
          status: { ...initial.status, eventCursor: { backendEpoch: 1, latestSequence: sequence } },
          session: sessions[sessionId] ?? sessions["desktop-session"],
        }),
      ),
    ),
    listSessions: vi.fn(() => later(() => Object.values(sessions).map((session) => structuredClone(session)))),
    getActiveSession: vi.fn(() => later(() => ({ sessionId: active }))),
    setActiveSession: vi.fn((sessionId: string) =>
      later(() => {
        active = sessionId;
        sequence += 1;
        events.push({
          sequence,
          backendEpoch: 1,
          resourceRevision: 0,
          operationId: null,
          category: "session.selectionChanged",
          sessionId,
        });
        return { sessionId };
      }),
    ),
    subscribe: vi.fn((afterSequence = 0): Promise<EventsSubscribeResult> =>
      later(() => ({
        backendEpoch: 1,
        events: events.filter((event) => event.sequence > afterSequence),
        nextSequence: sequence,
      })),
    ),
  };
  return { backend, active: () => active };
}

describe("startup session selection", () => {
  it("opens the saved active session and does not flip between sessions", async () => {
    // A fresh WebView profile (first launch, or cleared data): nothing remembered.
    const { backend, active } = await backendWithSavedActiveSession();
    await act(async () => {
      render(<App backend={backend} />);
    });
    // Let several event polls (1 s apart) run.
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 3500));
    });
    await waitFor(() => expect(active()).toBe("main-session"));
    // Startup must not overwrite the saved active session with the shell's default.
    expect(backend.setActiveSession.mock.calls.map(([id]) => id)).not.toContain("desktop-session");
    // No back-and-forth: once the main session is loaded, the default is never requested again.
    const loaded = backend.snapshot.mock.calls.map(([id]) => id ?? "desktop-session");
    const firstMain = loaded.indexOf("main-session");
    expect(firstMain).toBeGreaterThanOrEqual(0);
    expect(loaded.slice(firstMain)).not.toContain("desktop-session");
    expect(backend.setActiveSession.mock.calls.length).toBeLessThanOrEqual(1);
  }, 10_000);

  it("keeps a remembered session and makes it active once", async () => {
    window.localStorage.setItem("audiorouter.ui.last-session", "desktop-session");
    const { backend, active } = await backendWithSavedActiveSession();
    await act(async () => {
      render(<App backend={backend} />);
    });
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 3500));
    });
    await waitFor(() => expect(active()).toBe("desktop-session"));
    expect(backend.setActiveSession.mock.calls.map(([id]) => id)).toEqual(["desktop-session"]);
  }, 10_000);
});
