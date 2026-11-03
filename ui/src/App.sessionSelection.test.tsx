/** @vitest-environment jsdom */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
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
  it("opens a setup finding by node ID and focuses Properties without stopping audio", async () => {
    const { backend } = await backendWithSavedActiveSession();
    const stopSession = vi.fn(backend.stopSession);
    const node = demoSession.nodes[1];
    render(
      <App
        backend={{
          ...backend,
          stopSession,
          checkSetup: async () => ({
            sessionsChecked: 2,
            devicesChecked: 1,
            truncated: 0,
            notChecked: [],
            findings: [
              {
                code: "deviceMissing",
                severity: "problem",
                sessionId: "main-session",
                sessionName: "Stale name",
                nodeId: node.id,
                nodeName: "Stale tool name",
                message: "Missing test device",
                fix: "Inspect the tool",
              },
            ],
          }),
        }}
      />,
    );
    await waitFor(() => expect(backend.snapshot.mock.calls.some(([id]) => id === "main-session")).toBe(true));
    fireEvent.click(screen.getByRole("tab", { name: /Status/ }));
    fireEvent.click(await screen.findByRole("button", { name: "Open affected tool" }));
    await waitFor(() =>
      expect(screen.getByRole("tab", { name: /Properties/ }).getAttribute("aria-selected")).toBe("true"),
    );
    await waitFor(() => expect(document.activeElement).toBe(screen.getByRole("tab", { name: /Properties/ })));
    expect(stopSession).not.toHaveBeenCalled();
    expect(screen.getByText(/Opened the affected tool/)).toBeTruthy();
  });
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

describe("session switch interruption consent", () => {
  async function fixture() {
    const { backend } = await backendWithSavedActiveSession();
    let running = ["main-session", "desktop-session"];
    const snapshot = backend.snapshot;
    backend.snapshot = vi.fn(async (id?: string) => {
      const value = await snapshot(id);
      return { ...value, status: { ...value.status, activeSessionIds: [...running] } };
    });
    const stop = vi.fn(async (id: string) => {
      running = running.filter((value) => value !== id);
      return { sessionId: id, state: "stopped" as const, runtime: "fake" as const, recorders: [] };
    });
    backend.stopSession = stop;
    render(<App backend={backend} />);
    fireEvent.click(await screen.findByRole("tab", { name: "Session" }));
    await waitFor(() =>
      expect((screen.getByLabelText("Choose session") as HTMLSelectElement).value).toBe("main-session"),
    );
    fireEvent.change(screen.getByLabelText("Choose session"), { target: { value: "desktop-session" } });
    await screen.findByRole("dialog", { name: "Stop audio and switch session?" });
    return {
      backend,
      stop,
      setRunning: (ids: string[]) => {
        running = ids;
      },
    };
  }

  it("lists both affected sessions and recording effects; Cancel makes zero stop calls", async () => {
    const { stop } = await fixture();
    const dialog = screen.getByRole("dialog");
    expect(dialog.textContent).toContain("Main session");
    expect(dialog.textContent).toContain("Desktop session");
    expect(dialog.textContent).toContain("recordings will stop and finalize");
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Cancel" }));
    fireEvent.keyDown(document.activeElement!, { key: "Tab" });
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Stop and switch" }));
    fireEvent.keyDown(document.activeElement!, { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(stop).not.toHaveBeenCalled();
    expect((screen.getByLabelText("Choose session") as HTMLSelectElement).value).toBe("main-session");
  });

  it("retains the stop-all policy on confirmation", async () => {
    const { stop } = await fixture();
    fireEvent.click(screen.getByRole("button", { name: "Stop and switch" }));
    await waitFor(() => expect(stop).toHaveBeenCalledTimes(2));
    expect(stop.mock.calls.map(([id]) => id)).toEqual(["main-session", "desktop-session"]);
    await waitFor(() =>
      expect((screen.getByLabelText("Choose session") as HTMLSelectElement).value).toBe("desktop-session"),
    );
  });

  it("asks again when the running set changes before confirmation", async () => {
    const { stop, setRunning } = await fixture();
    setRunning(["newly-running"]);
    fireEvent.click(screen.getByRole("button", { name: "Stop and switch" }));
    await waitFor(() => expect(screen.getByRole("dialog").textContent).toContain("newly-running"));
    expect(stop).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  });

  it("reports completed stops without switching or restarting after a partial failure", async () => {
    const { stop } = await fixture();
    stop.mockResolvedValueOnce({ sessionId: "main-session", state: "stopped", runtime: "fake", recorders: [] });
    stop.mockRejectedValueOnce(new Error("Recorder finalization failed"));
    fireEvent.click(screen.getByRole("button", { name: "Stop and switch" }));
    await screen.findByText(/Session was not switched\. Other sessions or recordings may have changed/);
    expect(stop).toHaveBeenCalledTimes(2);
    expect(screen.getByText(/Stop completed for: Main session\./)).toBeTruthy();
    expect((screen.getByLabelText("Choose session") as HTMLSelectElement).value).toBe("main-session");
  });

  it("does not stop from stale state when the confirmation refresh fails", async () => {
    const { backend, stop } = await fixture();
    backend.snapshot.mockRejectedValueOnce(new Error("Backend disconnected"));
    fireEvent.click(screen.getByRole("button", { name: "Stop and switch" }));
    await screen.findByText(/Session was not switched\. Other sessions or recordings may have changed/);
    expect(stop).not.toHaveBeenCalled();
    expect((screen.getByLabelText("Choose session") as HTMLSelectElement).value).toBe("main-session");
  });
});
