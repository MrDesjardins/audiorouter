/** @vitest-environment jsdom */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import type { DiagnosticsSnapshot, EventsSubscribeResult } from "@audiorouter/contracts";
import { App } from "./App";
import { createDisconnectedBackend } from "./backend";
import { demoSession } from "./fixtures";

beforeAll(() => {
  Object.defineProperty(globalThis, "ResizeObserver", { configurable: true, value: class { observe() {} unobserve() {} disconnect() {} } });
});
afterEach(() => { cleanup(); window.localStorage.clear(); });

async function fixture(savedSession = demoSession) {
  const disconnected = createDisconnectedBackend();
  const initial = await disconnected.snapshot();
  let committed = structuredClone(savedSession);
  let planned = committed;
  let wake: (result: EventsSubscribeResult) => void = () => {};
  const pending = new Promise<EventsSubscribeResult>((resolve) => { wake = resolve; });
  const backend = {
    ...disconnected, connected: true,
    snapshot: vi.fn(async () => structuredClone({ ...initial, session: committed })),
    listSessions: vi.fn(async () => [structuredClone(committed)]),
    subscribe: vi.fn(() => pending),
    refreshDiagnostics: vi.fn(async (): Promise<DiagnosticsSnapshot> => ({ ...initial.diagnostics, nativeSessionId: demoSession.id, nativeAdapter: "configured-stopped", nativeAdapterKind: "endpoint" })),
    planGraph: vi.fn(async (candidate: typeof demoSession) => {
      planned = structuredClone(candidate);
      return { planId: "reviewed-plan", baseRevision: candidate.revision, expiresInMs: 60000, diff: [], affectedDestinations: [], warnings: [], requiredScopes: [] };
    }),
    commitGraph: vi.fn(async () => { committed = { ...planned, revision: planned.revision + 1 }; return { sessionId: committed.id, revision: committed.revision }; }),
    startSession: vi.fn(async () => ({ sessionId: demoSession.id, state: "running" as const, generation: 1, runtime: "native" as const })),
    rebindNativeEndpoint: vi.fn(async (sessionId: string, captureEndpointId: string, renderEndpointId: string) => ({ sessionId, state: "configured-stopped" as const, captureEndpointId, renderEndpointId })),
    stopSession: vi.fn(async () => ({ sessionId: demoSession.id, state: "stopped" as const, runtime: "native" as const, recorders: [] })),
  };
  return { backend, changeSaved: () => { committed = { ...committed, name: "Changed externally", revision: committed.revision + 1 }; }, refresh: () => wake({ backendEpoch: 1, events: [], nextSequence: 1, resyncRequired: true }) };
}

const sessionPanel = () => within(document.querySelector<HTMLElement>(".right-workbench")!);
const messageBar = () => within(document.querySelector<HTMLElement>(".global-action-message")!);
function addGain() { fireEvent.click(screen.getByRole("tab", { name: "Tools" })); fireEvent.click(sessionPanel().getByRole("button", { name: /^Gain / })); }
function start() { fireEvent.click(within(screen.getByRole("region", { name: "Session lifecycle" })).getByRole("button", { name: "Start session" })); }

describe("session refresh and playback regressions", () => {
  it("rebinds a stopped native worker to the selected endpoints before Play", async () => {
    const { backend } = await fixture();
    window.localStorage.setItem(`audiorouter.ui.endpoint-binding.${demoSession.id}`, JSON.stringify({ captureEndpointId: "voicemeeter-b1", renderEndpointId: "focusrite" }));
    render(<App backend={backend} />);
    await waitFor(() => expect(backend.listSessions).toHaveBeenCalled());
    start();
    await waitFor(() => expect(backend.startSession).toHaveBeenCalled());
    expect(backend.rebindNativeEndpoint).toHaveBeenCalledWith(demoSession.id, "voicemeeter-b1", "focusrite");
    expect(backend.rebindNativeEndpoint.mock.invocationCallOrder[0]).toBeLessThan(backend.startSession.mock.invocationCallOrder[0]);
  });

  const pathNode = (id: string, kind: "physicalInput" | "physicalOutput" | "gain", endpointId?: string): typeof demoSession.nodes[number] => ({
    id, kind, typeVersion: 1, name: id, enabled: true, bypass: false, parameters: endpointId ? { endpointId } : kind === "gain" ? { gainDb: 0 } : {},
    ports: kind === "physicalInput" ? [{ name: "out", direction: "output", channels: 2 }] : kind === "physicalOutput" ? [{ name: "in", direction: "input", channels: 2 }] : [{ name: "in", direction: "input", channels: 2 }, { name: "out", direction: "output", channels: 2 }],
  });
  const pathEdge = (id: string, sourceNode: string, destinationNode: string) => ({ id, sourceNode, sourcePort: "out", destinationNode, destinationPort: "in", matrix: [1, 0, 0, 1], enabled: true });
  const voiceAndGame = (scarlett?: string): typeof demoSession => ({
    ...demoSession,
    name: "Patrick Main Session",
    nodes: [pathNode("mic", "physicalInput", "pd200x"), pathNode("voice", "gain"), pathNode("cable-a", "physicalOutput", "cable-a-input"), pathNode("cable-b", "physicalInput", "cable-b-output"), pathNode("game-eq", "gain"), pathNode("scarlett", "physicalOutput", scarlett)],
    edges: [pathEdge("e1", "mic", "voice"), pathEdge("e2", "voice", "cable-a"), pathEdge("e3", "cable-b", "game-eq"), pathEdge("e4", "game-eq", "scarlett")],
  });

  it("plays a saved session with independent paths through one multi-path preparation", async () => {
    const { backend } = await fixture(voiceAndGame("focusrite"));
    const prepareNativePaths = vi.fn(async (sessionId: string) => ({ sessionId, generation: 1, state: "configured-stopped" as const, pathCount: 2, sourceNodeIds: ["mic", "cable-b"], branchNodeIds: ["cable-a", "scarlett"], renderEndpointIds: ["cable-a-input", "focusrite"] }));
    const prepareNativeEndpoint = vi.fn();
    render(<App backend={{ ...backend, prepareNativePaths, prepareNativeEndpoint }} />);
    await waitFor(() => expect(backend.listSessions).toHaveBeenCalled());
    await screen.findByRole("heading", { name: "Patrick Main Session" });
    start();
    await waitFor(() => expect(backend.startSession).toHaveBeenCalledWith(demoSession.id, expect.any(String)));
    expect(prepareNativePaths).toHaveBeenCalledWith(demoSession.id);
    expect(prepareNativePaths.mock.invocationCallOrder[0]).toBeLessThan(backend.startSession.mock.invocationCallOrder[0]);
    expect(prepareNativeEndpoint).not.toHaveBeenCalled();
    expect(backend.rebindNativeEndpoint).not.toHaveBeenCalled();
  });

  it("names the device node that still needs a device before a multi-path Play", async () => {
    const { backend } = await fixture(voiceAndGame());
    const prepareNativePaths = vi.fn();
    render(<App backend={{ ...backend, prepareNativePaths }} />);
    await waitFor(() => expect(backend.listSessions).toHaveBeenCalled());
    await screen.findByRole("heading", { name: "Patrick Main Session" });
    start();
    await messageBar().findByText(/Choose the device for scarlett in Properties/);
    expect(prepareNativePaths).not.toHaveBeenCalled();
    expect(backend.startSession).not.toHaveBeenCalled();
  });

  it("hydrates a real saved session even when its revision is below the preview revision", async () => {
    const { backend } = await fixture({ ...demoSession, revision: 0, name: "Real saved session" });
    render(<App backend={backend} />);
    fireEvent.click(screen.getByRole("tab", { name: "Session" }));
    await waitFor(() => expect((sessionPanel().getByLabelText("Choose session") as HTMLSelectElement).selectedOptions[0].text).toBe("Real saved session"));
    expect(screen.getByRole("heading", { name: "Real saved session" })).toBeTruthy();
  });

  it("preserves local edits and shows a conflict when an external commit arrives", async () => {
    const { backend, changeSaved, refresh } = await fixture();
    render(<App backend={backend} />);
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
    const view = render(<App backend={backend} />);
    await waitFor(() => expect(backend.listSessions).toHaveBeenCalled());
    addGain();
    const count = view.container.querySelectorAll(".react-flow__node").length;
    start();
    await waitFor(() => expect(backend.startSession).toHaveBeenCalledWith(demoSession.id, expect.any(String), expect.objectContaining({ nodes: expect.arrayContaining([expect.objectContaining({ kind: "gain" })]) })));
    await messageBar().findByText(/Temporary preview is running.*saved session is unchanged/);
    await act(async () => refresh());
    await waitFor(() => expect(backend.snapshot.mock.calls.length).toBeGreaterThan(1));
    expect(view.container.querySelectorAll(".react-flow__node").length).toBe(count);
    expect(messageBar().getByText(/Temporary preview is running/)).toBeTruthy();
    expect((await backend.snapshot()).session.revision).toBe(demoSession.revision);
  });

  it("commits a changed graph, immediately reconciles its revision, and starts without losing nodes", async () => {
    const { backend, refresh } = await fixture();
    const view = render(<App backend={backend} />);
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

  it("guides users to Devices without starting a simulated session when audio is unprepared", async () => {
    const { backend } = await fixture();
    backend.refreshDiagnostics = vi.fn(createDisconnectedBackend().refreshDiagnostics);
    render(<App backend={backend} />);
    await waitFor(() => expect(backend.listSessions).toHaveBeenCalled());
    start();
    await messageBar().findByText(/No audio started. Select the speaker or headphone device/);
    expect(screen.getByRole("tab", { name: "Tools" }).getAttribute("aria-selected")).toBe("true");
    fireEvent.click(screen.getByRole("tab", { name: "Devices" }));
    expect(screen.getByRole("tab", { name: "Devices" }).getAttribute("aria-selected")).toBe("true");
    expect(backend.startSession).not.toHaveBeenCalled();
  });

  it("stops an unexpected simulated runtime and never calls it audio success", async () => {
    const { backend } = await fixture();
    render(<App backend={{ ...backend, startSession: async () => ({ sessionId: demoSession.id, state: "running", generation: 1, runtime: "fake" }) }} />);
    await waitFor(() => expect(backend.listSessions).toHaveBeenCalled());
    start();
    await messageBar().findByText(/No audio played/);
    expect(backend.stopSession).toHaveBeenCalledWith(demoSession.id, expect.any(String));
    expect(screen.queryByText(/Audio session is running/)).toBeNull();
  });
});
