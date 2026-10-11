/** @vitest-environment jsdom */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import type { DiagnosticsSnapshot, EventsSubscribeResult } from "@audiorouter/contracts";
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

async function fixture(savedSession = demoSession) {
  const disconnected = createDisconnectedBackend();
  const initial = await disconnected.snapshot();
  let committed = structuredClone(savedSession);
  let planned = committed;
  let wake: (result: EventsSubscribeResult) => void = () => {};
  const pending = new Promise<EventsSubscribeResult>((resolve) => {
    wake = resolve;
  });
  const backend = {
    ...disconnected,
    connected: true,
    snapshot: vi.fn(async () => structuredClone({ ...initial, session: committed })),
    listSessions: vi.fn(async () => [structuredClone(committed)]),
    subscribe: vi.fn(() => pending),
    refreshDiagnostics: vi.fn(async (): Promise<DiagnosticsSnapshot> => ({
      ...initial.diagnostics,
      nativeSessionId: demoSession.id,
      nativeAdapter: "configured-stopped",
      nativeAdapterKind: "endpoint",
    })),
    planGraph: vi.fn(async (candidate: typeof demoSession) => {
      planned = structuredClone(candidate);
      return {
        planId: "reviewed-plan",
        baseRevision: candidate.revision,
        expiresInMs: 60000,
        diff: [],
        affectedDestinations: [],
        warnings: [],
        requiredScopes: [],
      };
    }),
    commitGraph: vi.fn(async () => {
      committed = { ...planned, revision: planned.revision + 1 };
      return { sessionId: committed.id, revision: committed.revision };
    }),
    startSession: vi.fn(async () => ({
      sessionId: demoSession.id,
      state: "running" as const,
      generation: 1,
      runtime: "native" as const,
    })),
    rebindNativeEndpoint: vi.fn(async (sessionId: string, captureEndpointId: string, renderEndpointId: string) => ({
      sessionId,
      state: "configured-stopped" as const,
      captureEndpointId,
      renderEndpointId,
    })),
    stopSession: vi.fn(async () => ({
      sessionId: demoSession.id,
      state: "stopped" as const,
      runtime: "native" as const,
      recorders: [],
    })),
  };
  return {
    backend,
    changeSaved: () => {
      committed = { ...committed, name: "Changed externally", revision: committed.revision + 1 };
    },
    refresh: () => wake({ backendEpoch: 1, events: [], nextSequence: 1, resyncRequired: true }),
  };
}

const sessionPanel = () => within(document.querySelector<HTMLElement>(".right-workbench")!);
const messageBar = () => within(document.querySelector<HTMLElement>(".global-action-message")!);
function addGain() {
  fireEvent.click(screen.getByRole("tab", { name: "Tools" }));
  fireEvent.click(sessionPanel().getByRole("button", { name: /^Gain / }));
}
// The top bar's Play (the removed hidden lifecycle panel had the same handler).
function start() {
  fireEvent.click(within(document.querySelector(".topbar") as HTMLElement).getByRole("button", { name: /^Play$/ }));
}

async function renderReady(element: Parameters<typeof render>[0]) {
  let view!: ReturnType<typeof render>;
  await act(async () => {
    view = render(element);
  });
  return view;
}

describe("session refresh and playback regressions", () => {
  it("rebinds a stopped native worker to the selected endpoints before Play", async () => {
    const { backend } = await fixture();
    window.localStorage.setItem(
      `audiorouter.ui.endpoint-binding.${demoSession.id}`,
      JSON.stringify({ captureEndpointId: "voicemeeter-b1", renderEndpointId: "focusrite" }),
    );
    await renderReady(<App backend={backend} />);
    await waitFor(() => expect(backend.listSessions).toHaveBeenCalled());
    start();
    await waitFor(() => expect(backend.startSession).toHaveBeenCalled());
    expect(backend.rebindNativeEndpoint).toHaveBeenCalledWith(demoSession.id, "voicemeeter-b1", "focusrite");
    expect(backend.rebindNativeEndpoint.mock.invocationCallOrder[0]).toBeLessThan(
      backend.startSession.mock.invocationCallOrder[0],
    );
  });

  const pathNode = (
    id: string,
    kind: "physicalInput" | "physicalOutput" | "gain",
    endpointId?: string,
  ): (typeof demoSession.nodes)[number] => ({
    id,
    kind,
    typeVersion: 1,
    name: id,
    enabled: true,
    bypass: false,
    parameters: endpointId ? { endpointId } : kind === "gain" ? { gainDb: 0 } : {},
    ports:
      kind === "physicalInput"
        ? [{ name: "out", direction: "output", channels: 2 }]
        : kind === "physicalOutput"
          ? [{ name: "in", direction: "input", channels: 2 }]
          : [
              { name: "in", direction: "input", channels: 2 },
              { name: "out", direction: "output", channels: 2 },
            ],
  });
  const pathEdge = (id: string, sourceNode: string, destinationNode: string) => ({
    id,
    sourceNode,
    sourcePort: "out",
    destinationNode,
    destinationPort: "in",
    matrix: [1, 0, 0, 1],
    enabled: true,
  });
  const voiceAndGame = (scarlett?: string): typeof demoSession => ({
    ...demoSession,
    name: "Patrick Main Session",
    nodes: [
      pathNode("mic", "physicalInput", "pd200x"),
      pathNode("voice", "gain"),
      pathNode("cable-a", "physicalOutput", "cable-a-input"),
      pathNode("cable-b", "physicalInput", "cable-b-output"),
      pathNode("game-eq", "gain"),
      pathNode("scarlett", "physicalOutput", scarlett),
    ],
    edges: [
      pathEdge("e1", "mic", "voice"),
      pathEdge("e2", "voice", "cable-a"),
      pathEdge("e3", "cable-b", "game-eq"),
      pathEdge("e4", "game-eq", "scarlett"),
    ],
  });

  it("plays a saved session with independent paths through one multi-path preparation", async () => {
    const { backend } = await fixture(voiceAndGame("focusrite"));
    const prepareNativePaths = vi.fn(async (sessionId: string) => ({
      sessionId,
      generation: 1,
      state: "configured-stopped" as const,
      pathCount: 2,
      sourceNodeIds: ["mic", "cable-b"],
      branchNodeIds: ["cable-a", "scarlett"],
      renderEndpointIds: ["cable-a-input", "focusrite"],
    }));
    const prepareNativeEndpoint = vi.fn();
    await renderReady(<App backend={{ ...backend, prepareNativePaths, prepareNativeEndpoint }} />);
    await waitFor(() => expect(backend.listSessions).toHaveBeenCalled());
    await screen.findByRole("heading", { name: "Patrick Main Session" });
    start();
    await waitFor(() => expect(backend.startSession).toHaveBeenCalledWith(demoSession.id, expect.any(String)));
    expect(prepareNativePaths).toHaveBeenCalledWith(demoSession.id);
    expect(prepareNativePaths.mock.invocationCallOrder[0]).toBeLessThan(
      backend.startSession.mock.invocationCallOrder[0],
    );
    expect(prepareNativeEndpoint).not.toHaveBeenCalled();
    expect(backend.rebindNativeEndpoint).not.toHaveBeenCalled();
  });

  it("plays a saved Test Signal route to its chosen output without any input device", async () => {
    const tone = {
      ...pathNode("tone", "physicalInput"),
      kind: "testSignal" as const,
      name: "Tone",
      parameters: { frequencyHz: 440 },
    };
    const { backend } = await fixture({
      ...demoSession,
      name: "Tone check",
      nodes: [tone, pathNode("speakers", "physicalOutput", "focusrite")],
      edges: [pathEdge("e1", "tone", "speakers")],
    });
    const prepareNativePaths = vi.fn(async (sessionId: string) => ({
      sessionId,
      generation: 1,
      state: "configured-stopped" as const,
      pathCount: 1,
      sourceNodeIds: ["tone"],
      branchNodeIds: ["speakers"],
      renderEndpointIds: ["focusrite"],
    }));
    const prepareNativeEndpoint = vi.fn();
    await renderReady(<App backend={{ ...backend, prepareNativePaths, prepareNativeEndpoint }} />);
    await screen.findByRole("heading", { name: "Tone check" });
    start();
    await waitFor(() => expect(backend.startSession).toHaveBeenCalledWith(demoSession.id, expect.any(String)));
    expect(prepareNativePaths).toHaveBeenCalledWith(demoSession.id);
    expect(prepareNativeEndpoint).not.toHaveBeenCalled();
  });

  it("names the device node that still needs a device before a multi-path Play", async () => {
    const { backend } = await fixture(voiceAndGame());
    const prepareNativePaths = vi.fn();
    await renderReady(<App backend={{ ...backend, prepareNativePaths }} />);
    await waitFor(() => expect(backend.listSessions).toHaveBeenCalled());
    await screen.findByRole("heading", { name: "Patrick Main Session" });
    start();
    await messageBar().findByText(/Choose the device for scarlett in Properties/);
    expect(prepareNativePaths).not.toHaveBeenCalled();
    expect(backend.startSession).not.toHaveBeenCalled();
  });

  it("applies live bypass without Save or Stop and preserves an unsaved name", async () => {
    const saved = voiceAndGame("focusrite");
    const { backend } = await fixture(saved);
    let running = false;
    const snapshot = backend.snapshot;
    backend.snapshot = vi.fn(async () => {
      const value = await snapshot();
      return { ...value, status: { ...value.status, activeSessionIds: running ? [saved.id] : [] } };
    });
    const startSession = backend.startSession;
    backend.startSession = vi.fn(async () => {
      running = true;
      return startSession();
    });
    const commit = backend.commitGraph;
    backend.commitGraph = vi.fn(async () => ({
      ...(await commit()),
      activation: {
        state: "running" as const,
        generation: 2,
        runtime: "native" as const,
        native: { state: "applied" as const, adapter: "multi-input" },
      },
    }));
    const prepareNativePaths = vi.fn(async (sessionId: string) => ({
      sessionId,
      generation: 1,
      state: "configured-stopped" as const,
      pathCount: 2,
      sourceNodeIds: ["mic", "cable-b"],
      branchNodeIds: ["cable-a", "scarlett"],
      renderEndpointIds: ["cable-a-input", "focusrite"],
    }));
    const view = await renderReady(<App backend={{ ...backend, prepareNativePaths }} />);
    await screen.findByRole("heading", { name: "Patrick Main Session" });
    start();
    await messageBar().findByText(/Audio session is running/);
    fireEvent.click(screen.getByRole("tab", { name: "Properties" }));
    fireEvent.click(view.container.querySelector('[data-id="game-eq"]')!);
    const name = await screen.findByLabelText("Node name");
    fireEvent.change(name, { target: { value: "Unsaved name" } });
    fireEvent.blur(name);
    fireEvent.click(screen.getByLabelText("Bypass"));
    await waitFor(() =>
      expect(backend.commitGraph, document.querySelector(".global-action-message")?.textContent).toHaveBeenCalledTimes(
        1,
      ),
    );
    const submitted = backend.planGraph.mock.calls[0][0];
    expect(submitted.nodes.find((node) => node.id === "game-eq")).toMatchObject({ name: "game-eq", bypass: true });
    await messageBar().findByText(/applied to the playing audio/);
    expect((screen.getByLabelText("Node name") as HTMLInputElement).value).toBe("Unsaved name");
    expect((screen.getByLabelText("Bypass") as HTMLInputElement).checked).toBe(true);
    expect(backend.stopSession).not.toHaveBeenCalled();
    expect(backend.startSession).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByLabelText("Bypass"));
    await waitFor(() => expect(backend.commitGraph).toHaveBeenCalledTimes(2));
    expect(backend.planGraph.mock.calls[1][0].nodes.find((node) => node.id === "game-eq")?.bypass).toBe(false);
  });

  it("hydrates a real saved session even when its revision is below the preview revision", async () => {
    const { backend } = await fixture({ ...demoSession, revision: 0, name: "Real saved session" });
    await renderReady(<App backend={backend} />);
    fireEvent.click(screen.getByRole("tab", { name: "Session" }));
    await waitFor(() =>
      expect((sessionPanel().getByLabelText("Choose session") as HTMLSelectElement).selectedOptions[0].text).toBe(
        "Real saved session",
      ),
    );
    expect(screen.getByRole("heading", { name: "Real saved session" })).toBeTruthy();
  });

  it("preserves local edits and shows a conflict when an external commit arrives", async () => {
    const { backend, changeSaved, refresh } = await fixture();
    await renderReady(<App backend={backend} />);
    await waitFor(() => expect(backend.subscribe).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("tab", { name: "Session" }));
    fireEvent.click(sessionPanel().getByRole("button", { name: "Rename" }));
    fireEvent.change(sessionPanel().getByLabelText("Session name"), { target: { value: "My unsaved name" } });
    changeSaved();
    await act(async () => refresh());
    await messageBar().findByText(/Your draft is preserved/);
    expect((sessionPanel().getByLabelText("Session name") as HTMLInputElement).value).toBe("My unsaved name");
    fireEvent.click(sessionPanel().getByRole("button", { name: "Revert edits" }));
    expect((sessionPanel().getByLabelText("Session name") as HTMLInputElement).value).toBe("Changed externally");
  });

  it("previews an unsaved graph without saving it and preserves it across snapshots", async () => {
    const { backend, refresh } = await fixture();
    const view = await renderReady(<App backend={backend} />);
    await waitFor(() => expect(backend.listSessions).toHaveBeenCalled());
    addGain();
    const count = view.container.querySelectorAll(".react-flow__node").length;
    start();
    await waitFor(() =>
      expect(backend.startSession).toHaveBeenCalledWith(
        demoSession.id,
        expect.any(String),
        expect.objectContaining({ nodes: expect.arrayContaining([expect.objectContaining({ kind: "gain" })]) }),
      ),
    );
    await messageBar().findByText(/Temporary preview is running.*saved session is unchanged/);
    await act(async () => refresh());
    await waitFor(() => expect(backend.snapshot.mock.calls.length).toBeGreaterThan(1));
    expect(view.container.querySelectorAll(".react-flow__node").length).toBe(count);
    expect(messageBar().getByText(/Temporary preview is running/)).toBeTruthy();
    expect((await backend.snapshot()).session.revision).toBe(demoSession.revision);
  });

  it("commits a changed graph, immediately reconciles its revision, and starts without losing nodes", async () => {
    const { backend, refresh } = await fixture();
    const view = await renderReady(<App backend={backend} />);
    await waitFor(() => expect(backend.listSessions).toHaveBeenCalled());
    addGain();
    const count = view.container.querySelectorAll(".react-flow__node").length;
    fireEvent.click(screen.getByRole("tab", { name: "Session" }));
    fireEvent.click(within(document.querySelector<HTMLElement>(".topbar")!).getByRole("button", { name: "Save" }));
    await messageBar().findByText(/Route saved/);
    start();
    await waitFor(() => expect(backend.startSession).toHaveBeenCalledWith(demoSession.id, expect.any(String)));
    await messageBar().findByText(/Audio session is running/);
    await act(async () => refresh());
    expect(view.container.querySelectorAll(".react-flow__node").length).toBe(count);
    expect(messageBar().getByText(/Audio session is running/)).toBeTruthy();
  });

  it("points users to the device in Properties without starting a simulated session when audio is unprepared", async () => {
    const { backend } = await fixture();
    backend.refreshDiagnostics = vi.fn(createDisconnectedBackend().refreshDiagnostics);
    await renderReady(<App backend={backend} />);
    await waitFor(() => expect(backend.listSessions).toHaveBeenCalled());
    start();
    await messageBar().findByText(/No audio started. Select the speaker or headphone device/);
    expect(screen.getByRole("tab", { name: "Tools" }).getAttribute("aria-selected")).toBe("true");
    expect(screen.queryByRole("tab", { name: "Devices" })).toBeNull();
    expect(backend.startSession).not.toHaveBeenCalled();
  });

  it("stops an unexpected simulated runtime and never calls it audio success", async () => {
    const { backend } = await fixture();
    await renderReady(
      <App
        backend={{
          ...backend,
          startSession: async () => ({ sessionId: demoSession.id, state: "running", generation: 1, runtime: "fake" }),
        }}
      />,
    );
    await waitFor(() => expect(backend.listSessions).toHaveBeenCalled());
    start();
    await messageBar().findByText(/No audio played/);
    expect(backend.stopSession).toHaveBeenCalledWith(demoSession.id, expect.any(String));
    expect(screen.queryByText(/Audio session is running/)).toBeNull();
  });
});

describe("switching sessions", () => {
  const other = { ...structuredClone(demoSession), id: "other-session", name: "Other Session" };
  // One backend audio adapter, attached to whichever session prepared it, as
  // in the control plane: it stays attached after Stop until detached.
  async function twoSessions() {
    const { backend } = await fixture();
    const native = { sessionId: demoSession.id as string | null, adapter: "configured-stopped" as string };
    let running: string | null = null;
    const snapshot = backend.snapshot;
    const diagnostics = backend.refreshDiagnostics;
    const audio = {
      ...backend,
      listSessions: vi.fn(async () => [structuredClone(demoSession), structuredClone(other)]),
      snapshot: vi.fn(async () => {
        const value = await snapshot();
        return { ...value, status: { ...value.status, activeSessionIds: running ? [running] : [] } };
      }),
      refreshDiagnostics: vi.fn(async (): Promise<DiagnosticsSnapshot> => ({
        ...(await diagnostics()),
        nativeSessionId: native.sessionId,
        nativeAdapter: (native.sessionId
          ? native.adapter
          : "implemented-not-activated") as DiagnosticsSnapshot["nativeAdapter"],
        nativeAdapterKind: native.sessionId ? "endpoint" : null,
      })),
      prepareNativeEndpoint: vi.fn(async (sessionId: string, captureEndpointId: string, renderEndpointId: string) => {
        if (native.sessionId && native.sessionId !== sessionId)
          throw new Error("native endpoint worker limit reached or session is already attached");
        native.sessionId = sessionId;
        native.adapter = "configured-stopped";
        return { sessionId, state: "configured-stopped" as const, captureEndpointId, renderEndpointId };
      }),
      detachNativeEndpoint: vi.fn(async (sessionId: string) => {
        if (native.sessionId !== sessionId) throw new Error("native endpoint worker is not attached to this session");
        native.sessionId = null;
        return { sessionId, state: "detached" as const };
      }),
      startSession: vi.fn(async (sessionId: string) => {
        if (native.sessionId !== sessionId) throw new Error("native worker is already attached");
        running = sessionId;
        native.adapter = "running";
        return { sessionId, state: "running" as const, generation: 1, runtime: "native" as const };
      }),
      stopSession: vi.fn(async (sessionId: string) => {
        if (running === sessionId) running = null;
        native.adapter = "configured-stopped";
        return { sessionId, state: "stopped" as const, runtime: "native" as const, recorders: [] };
      }),
    };
    for (const id of [demoSession.id, other.id])
      window.localStorage.setItem(
        `audiorouter.ui.endpoint-binding.${id}`,
        JSON.stringify({ captureEndpointId: "microphone", renderEndpointId: "headphones" }),
      );
    return { audio, native };
  }
  const chooseSession = (id: string) => {
    fireEvent.click(screen.getByRole("tab", { name: "Session" }));
    fireEvent.change(screen.getByLabelText("Choose session"), { target: { value: id } });
  };

  it("stops the running session and releases its audio, then plays the new one on Play", async () => {
    const { audio, native } = await twoSessions();
    await renderReady(<App backend={audio} />);
    await waitFor(() => expect(audio.listSessions).toHaveBeenCalled());
    start();
    await messageBar().findByText(/Audio session is running/);
    chooseSession(other.id);
    await messageBar().findByText(/Stopped the running session\. Press Play to start "Other Session"/);
    expect(audio.stopSession).toHaveBeenCalledWith(demoSession.id, expect.any(String));
    expect(audio.detachNativeEndpoint).toHaveBeenCalledWith(demoSession.id);
    expect(audio.startSession).toHaveBeenCalledTimes(1);
    expect(native.sessionId).toBeNull();
    start();
    await messageBar().findByText(/Audio session is running/);
    expect(audio.startSession).toHaveBeenLastCalledWith(other.id, expect.any(String));
    // Going back works the same way.
    chooseSession(demoSession.id);
    await messageBar().findByText(/Stopped the running session\. Press Play to start/);
    start();
    await waitFor(() => expect(audio.startSession).toHaveBeenLastCalledWith(demoSession.id, expect.any(String)));
  });

  it("Play releases a stopped adapter that another session left attached", async () => {
    const { audio, native } = await twoSessions();
    native.sessionId = other.id;
    await renderReady(<App backend={audio} />);
    await waitFor(() => expect(audio.listSessions).toHaveBeenCalled());
    start();
    await messageBar().findByText(/Audio session is running/);
    expect(audio.detachNativeEndpoint).toHaveBeenCalledWith(other.id);
    expect(audio.detachNativeEndpoint.mock.invocationCallOrder[0]).toBeLessThan(
      audio.startSession.mock.invocationCallOrder[0],
    );
  });

  it("Play does not take over audio another session is playing", async () => {
    const { audio, native } = await twoSessions();
    native.sessionId = other.id;
    native.adapter = "running";
    await renderReady(<App backend={audio} />);
    await waitFor(() => expect(audio.listSessions).toHaveBeenCalled());
    start();
    await messageBar().findByText(/"Other Session" is playing\. Stop it first/);
    expect(audio.detachNativeEndpoint).not.toHaveBeenCalled();
    expect(audio.startSession).not.toHaveBeenCalled();
  });

  it("keeps the session when the running one cannot be stopped", async () => {
    const { audio } = await twoSessions();
    audio.stopSession = vi.fn(async () => {
      throw new Error("stop refused");
    });
    await renderReady(<App backend={audio} />);
    await waitFor(() => expect(audio.listSessions).toHaveBeenCalled());
    start();
    await messageBar().findByText(/Audio session is running/);
    chooseSession(other.id);
    await messageBar().findByText(/stop refused/);
    expect((screen.getByLabelText("Choose session") as HTMLSelectElement).value).toBe(demoSession.id);
  });
});

describe("Reconnect", () => {
  it("shows that it is reconnecting and reports the result", async () => {
    const { backend } = await fixture();
    await renderReady(<App backend={backend} />);
    await waitFor(() => expect(backend.listSessions).toHaveBeenCalled());
    const calls = backend.snapshot.mock.calls.length;
    fireEvent.click(
      within(document.querySelector(".topbar") as HTMLElement).getByRole("button", { name: "Reconnect" }),
    );
    await messageBar().findByText(/Reconnected\. Sessions, devices and audio status are up to date\./);
    expect(backend.snapshot.mock.calls.length).toBeGreaterThan(calls);
  });

  it("reports a failed reconnect instead of doing nothing visible", async () => {
    const { backend } = await fixture();
    await renderReady(<App backend={backend} />);
    await waitFor(() => expect(backend.listSessions).toHaveBeenCalled());
    backend.snapshot.mockImplementation(async () => {
      throw new Error("pipe closed");
    });
    fireEvent.click(
      within(document.querySelector(".topbar") as HTMLElement).getByRole("button", { name: "Reconnect" }),
    );
    await messageBar().findByText(/Reconnect failed/, undefined, { timeout: 15000 });
  }, 20000);
});
