/** @vitest-environment jsdom */

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { createDisconnectedBackend } from "./backend";
import { DraftConnectionList, insertMixerActionId, removeMixerActionId } from "./DraftConnectionList";
import { appendDraftConnection, insertDraftMixer } from "./draft";
import { demoSession } from "./fixtures";
import { BackendConnectionContext } from "./backendConnectionContext";
import { GraphList } from "./GraphList";
import type { ProcessorDescriptor } from "./processorCatalog";

function connectedPreviewBackend() {
  return { ...createDisconnectedBackend(), connected: true };
}

beforeAll(() => {
  Object.defineProperty(globalThis, "ResizeObserver", {
    configurable: true,
    value: class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  });
});

afterEach(() => cleanup());

describe("keyboard connection dialog", () => {
  it("adds a real processor draft through the canvas drag-and-drop shelf", async () => {
    render(<App backend={connectedPreviewBackend()} />);
    const dropSource = await screen.findByRole("button", { name: /^Gain$/ });
    const canvas = screen.getByLabelText("Signal-flow graph");
    const values = new Map<string, string>();
    const dataTransfer = {
      types: ["application/x-audiorouter-library-kind"],
      effectAllowed: "copy",
      setData: (type: string, value: string) => values.set(type, value),
      getData: (type: string) => values.get(type) ?? "",
    };
    fireEvent.dragStart(dropSource, { dataTransfer });
    fireEvent.drop(canvas, { dataTransfer });
    await waitFor(() => expect(screen.getByText("Gain 1 added to the draft. Review and plan the changes before committing.")).toBeTruthy());
  });

  it("binds only explicitly selected active endpoints and preserves the no-defaults boundary", async () => {
    const capture = {
      id: "capture-active",
      name: "Active capture",
      direction: "capture" as const,
      state: "active" as const,
      defaultRoles: [],
      format: { sampleRateHz: 48000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 },
      periods: { default100ns: 100000, minimum100ns: 30000 },
    };
    const renderDevice = { ...capture, id: "render-active", name: "Active render", direction: "render" as const };
    const inactive = { id: "capture-inactive", name: "Inactive capture", direction: "capture" as const, state: "unplugged" as const, defaultRoles: [] };
    const prepareNativeEndpoint = vi.fn(async (sessionId: string, captureEndpointId: string, renderEndpointId: string) => ({
      sessionId,
      state: "configured-stopped" as const,
      captureEndpointId,
      renderEndpointId,
    }));
    const startSession = vi.fn(async () => ({ sessionId: "demo-session", state: "running" as const, runtime: "fake" as const, generation: 1 }));
    const backend = { ...connectedPreviewBackend(), listDevices: async () => [inactive, capture, renderDevice], prepareNativeEndpoint, startSession };

    render(<App backend={backend} />);

    const endpointPanel = screen.getByRole("region", { name: "Endpoint binding" });
    const captureSelect = await screen.findByRole("combobox", { name: "Native capture endpoint" });
    const renderSelect = screen.getByRole("combobox", { name: "Native render endpoint" });
    expect(within(captureSelect).queryByRole("option", { name: /Inactive capture/ })).toBeNull();
    await waitFor(() => expect((captureSelect as HTMLSelectElement).value).toBe("capture-active"));
    await waitFor(() => expect((renderSelect as HTMLSelectElement).value).toBe("render-active"));

    fireEvent.click(screen.getByRole("button", { name: "Prepare native endpoints" }));
    await waitFor(() => expect(prepareNativeEndpoint).toHaveBeenCalledWith("demo-session", "capture-active", "render-active"));
    expect(await screen.findByText("Prepared configured-stopped; start the session to activate audio.")).toBeTruthy();
    expect(screen.getByText(/Endpoint defaults, volume, and mute are never changed/)).toBeTruthy();
    fireEvent.click(within(endpointPanel).getByRole("button", { name: "Start session" }));
    await waitFor(() => expect(startSession).toHaveBeenCalledWith("demo-session", expect.any(String)));
  });

  it("opens with focus, wraps focus, retains validation errors, and restores focus", async () => {
    const backend = connectedPreviewBackend();
    render(<App backend={backend} />);

    const opener = screen.getByRole("button", { name: "Keyboard connection dialog" });
    opener.focus();
    fireEvent.click(opener);

    const dialog = await screen.findByRole("dialog", { name: "Keyboard connection" });
    const source = within(dialog).getByRole("combobox", { name: "Keyboard source output port" });
    expect(document.activeElement).toBe(source);

    const close = within(dialog).getByRole("button", { name: "Close keyboard connection dialog" });
    const cancel = within(dialog).getByRole("button", { name: "Cancel" });
    cancel.focus();
    fireEvent.keyDown(window, { key: "Tab" });
    expect(document.activeElement).toBe(close);
    close.focus();
    fireEvent.keyDown(window, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(cancel);

    const add = within(dialog).getByRole("button", { name: "Add connection to draft" });
    fireEvent.click(add);
    expect(screen.getByRole("dialog", { name: "Keyboard connection" })).toBeTruthy();
    expect(screen.getByText("Choose an output and input port first.")).toBeTruthy();

    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Keyboard connection" })).toBeNull());
    await waitFor(() => expect(document.activeElement).toBe(opener));
  });

  it("closes after a valid draft connection", async () => {
    const backend = connectedPreviewBackend();
    render(<App backend={backend} />);

    fireEvent.click(screen.getByRole("button", { name: "Keyboard connection dialog" }));
    const dialog = await screen.findByRole("dialog", { name: "Keyboard connection" });
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Keyboard source output port" }), { target: { value: "mic::out" } });
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Keyboard destination input port" }), { target: { value: "voice::in" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Add connection to draft" }));

    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Keyboard connection" })).toBeNull());
    expect(screen.getByText("Connection added to the draft. Review and plan the changes before committing.")).toBeTruthy();
  });

  it("renders named canvas handles for connected graph editing", async () => {
    render(<App backend={connectedPreviewBackend()} />);

    expect(await screen.findByLabelText("Microphone out output")).toBeTruthy();
    expect(await screen.findByLabelText("Voice gain in input")).toBeTruthy();
  });

  it("renders the read-only persisted recovery checkpoint panel", async () => {
    const backend = {
      ...createDisconnectedBackend(),
      connected: true,
      listRecordingRecovery: async () => ({
        items: [{ recordingId: "take-recovery", status: "invalid" as const }],
        nextCursor: null,
      }),
    };
    render(<App backend={backend} />);
    expect(await screen.findByRole("heading", { name: "Recovery checkpoints" })).toBeTruthy();
    expect(await screen.findByText("take-recovery")).toBeTruthy();
    expect(await screen.findByText(/invalid/)).toBeTruthy();
    expect(screen.getByText("This list is read-only. Recovery inspection does not open, repair, play, or delete audio files.")).toBeTruthy();
  });

  it("exposes revisioned virtual-route editing in the connected UI", async () => {
    const replaceVirtualRoutes = vi.fn(async (baseRevision: number, routes: unknown[]) => ({
      state: "applied" as const,
      revision: baseRevision + 1,
      routes: routes as [],
    }));
    const backend = {
      ...createDisconnectedBackend(),
      connected: true,
      listVirtualRoutes: async () => ({ revision: 2, routes: [] }),
      replaceVirtualRoutes,
    };
    render(<App backend={backend} />);
    expect(await screen.findByRole("heading", { name: "Virtual-bus routes" })).toBeTruthy();
    expect((screen.getByRole("textbox", { name: "Virtual-route base revision" }) as HTMLInputElement).value).toBe("2");
    fireEvent.click(screen.getByRole("button", { name: "Replace routes" }));
    await waitFor(() => expect(replaceVirtualRoutes).toHaveBeenCalledWith(2, [], expect.any(String)));
    expect(await screen.findByText("Virtual routes applied at revision 3.")).toBeTruthy();
  });

  it("creates an unarmed recorder with the explicit UI configuration", async () => {
    const createRecorder = vi.fn(async (params: { recorderId: string }) => ({
      sessionId: demoSession.id,
      nodeId: null,
      recorderId: params.recorderId,
      format: "wavPcm24" as const,
      path: "C:\\Audio\\take.wav",
      state: "idle" as const,
      armed: false as const,
    }));
    const backend = { ...connectedPreviewBackend(), createRecorder };
    render(<App backend={backend} />);
    fireEvent.change(await screen.findByRole("textbox", { name: "Recorder ID" }), { target: { value: "voice-take" } });
    fireEvent.click(screen.getByRole("button", { name: "Create recorder" }));
    await waitFor(() => expect(createRecorder).toHaveBeenCalledWith(expect.objectContaining({ recorderId: "voice-take", format: "wavPcm24", channels: 2, sampleRate: 48000, sequence: 1, dither: true })));
    expect(await screen.findByText(/Recorder voice-take created unarmed/)).toBeTruthy();
  });

  it("offers bounded slider and precise entry for numeric processor parameters", async () => {
    const processor: ProcessorDescriptor = {
      id: "gain",
      version: 1,
      category: "effect",
      availability: { status: "available" },
      latencySamples: 0,
      parameters: [{ name: "gainDb", type: "number", unit: "dB", minimum: -60, maximum: 24, default: 0 }],
    };
    const backend = { ...createDisconnectedBackend(), connected: true, listProcessors: async () => [processor] };
    render(<App backend={backend} />);
    fireEvent.click(await screen.findByLabelText("Voice gain, gain"));

    const slider = await screen.findByRole("slider", { name: "gainDb slider" });
    const precise = screen.getByRole("spinbutton", { name: "gainDb precise value" });
    expect(slider.getAttribute("min")).toBe("-60");
    expect(slider.getAttribute("max")).toBe("24");
    expect((precise as HTMLInputElement).value).toBe("0");
    fireEvent.change(slider, { target: { value: "-6" } });
    expect((precise as HTMLInputElement).value).toBe("-6");
    expect(screen.getByText("Draft effect: gainDb: 0 → -6. Plan changes to validate and commit.")).toBeTruthy();
  });

  it("commits a dropped gate parameter through the graph backend", async () => {
    const planGraph = vi.fn(async (candidate: typeof demoSession) => ({
      planId: "gate-plan",
      baseRevision: candidate.revision,
      expiresInMs: 30_000,
      diff: [],
      affectedDestinations: [],
      warnings: [],
      requiredScopes: ["graphWrite"],
    }));
    const commitGraph = vi.fn(async () => ({ sessionId: demoSession.id, revision: demoSession.revision + 1 }));
    const gate = {
      id: "gate",
      version: 1,
      category: "dynamics" as const,
      availability: { status: "available" as const },
      latencySamples: 0,
      parameters: [
        { name: "thresholdDb", type: "number" as const, unit: "dB", minimum: -80, maximum: 0, default: -45 },
      ],
    };
    const backend = {
      ...connectedPreviewBackend(),
      listProcessors: async () => [gate],
      planGraph,
      commitGraph,
    };
    render(<App backend={backend} />);

    const dropSource = await screen.findByRole("button", { name: /^Gate$/ });
    const canvas = screen.getByLabelText("Signal-flow graph");
    const values = new Map<string, string>();
    const dataTransfer = {
      types: ["application/x-audiorouter-library-kind"],
      effectAllowed: "copy",
      setData: (type: string, value: string) => values.set(type, value),
      getData: (type: string) => values.get(type) ?? "",
    };
    fireEvent.dragStart(dropSource, { dataTransfer });
    fireEvent.drop(canvas, { dataTransfer });
    fireEvent.click(await screen.findByLabelText("Gate 1, gate"));
    fireEvent.change(await screen.findByRole("spinbutton", { name: "thresholdDb precise value" }), { target: { value: "-30" } });
    fireEvent.click(screen.getByRole("button", { name: "Plan changes" }));

    await waitFor(() => expect(commitGraph).toHaveBeenCalledWith("gate-plan", demoSession.revision, expect.any(String)));
    expect(planGraph).toHaveBeenCalledWith(expect.objectContaining({
      nodes: expect.arrayContaining([
        expect.objectContaining({ kind: "gate", parameters: expect.objectContaining({ thresholdDb: -30 }) }),
      ]),
    }));
    expect(await screen.findByText("Committed revision 8. Reconnect to refresh the authoritative view.")).toBeTruthy();
  });

  it("expands an authoritative EQ preset into a draft node", async () => {
    const backend = {
      ...connectedPreviewBackend(),
      listPresets: async () => ({
        voiceChains: [],
        eq: [{ id: "hum50Hz", version: 1, name: "50 Hz hum notch", description: "Narrow 50 Hz notch starting point for mains hum." }],
      }),
    };
    render(<App backend={backend} />);
    fireEvent.click(await screen.findByRole("button", { name: "Add EQ to draft" }));
    expect(screen.getByText("Parametric EQ 1 added to the draft. Review and plan the changes before committing.")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Parametric EQ 1" })).toBeTruthy();
  });

  it("expands a voice-chain preset into ordinary draft processors", async () => {
    const backend = {
      ...connectedPreviewBackend(),
      listPresets: async () => ({
        voiceChains: [{ id: "voiceGateAndCompression", version: 1, name: "Voice gate and compression", description: "Voice neutral with a conservative gate and compression." }],
        eq: [],
      }),
    };
    render(<App backend={backend} />);
    fireEvent.click(await screen.findByRole("button", { name: "Add voice chain to draft" }));
    expect(screen.getByText("Gate 1, Compressor 1, Limiter 1 added to the draft. Review and plan the changes before committing.")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Gate 1" })).toBeTruthy();
  });

  it("persists tidy layout positions as presentation state", async () => {
    render(<App backend={connectedPreviewBackend()} />);

    fireEvent.click(await screen.findByRole("button", { name: "Tidy layout" }));
    expect(JSON.parse(window.localStorage.getItem("audiorouter.ui.layout.demo-session") ?? "null")).toMatchObject({
      mic: { x: 0, y: 0 },
      voice: { x: 260, y: 0 },
    });
  });

  it("exposes previewable mixer topology actions from the draft connection list", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const inserted = insertDraftMixer(connected, "edge-1");
    const onRemove = vi.fn();
    const onToggle = vi.fn();
    render(<BackendConnectionContext.Provider value={true}><DraftConnectionList session={inserted} onRemove={onRemove} onToggle={onToggle} /></BackendConnectionContext.Provider>);

    fireEvent.click(screen.getAllByRole("button", { name: /Insert mixer on/ })[0]);
    expect(onRemove).toHaveBeenCalledWith(insertMixerActionId("edge-1"));
    fireEvent.click(screen.getByRole("button", { name: "Remove and reconnect Mixer 1" }));
    expect(onRemove).toHaveBeenCalledWith(removeMixerActionId("mixer-1"));
  });

  it("executes mixer topology previews through the connected App draft boundary", async () => {
    render(<App backend={connectedPreviewBackend()} />);
    fireEvent.click(screen.getByRole("button", { name: "Keyboard connection dialog" }));
    const dialog = await screen.findByRole("dialog", { name: "Keyboard connection" });
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Keyboard source output port" }), { target: { value: "mic::out" } });
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Keyboard destination input port" }), { target: { value: "voice::in" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Add connection to draft" }));

    fireEvent.click(screen.getAllByRole("button", { name: /Insert mixer on/ })[0]);
    expect(screen.getByText("Mixer inserted into the draft. Review and plan the changes before committing.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Remove and reconnect Mixer 1" }));
    expect(screen.getByText("Mixer removed and its single path reconnected in the draft. Review and plan the changes before committing.")).toBeTruthy();
  });

  it("inserts a built-in processor directly from a connected draft path", async () => {
    render(<App backend={connectedPreviewBackend()} />);
    fireEvent.click(screen.getByRole("button", { name: "Keyboard connection dialog" }));
    const dialog = await screen.findByRole("dialog", { name: "Keyboard connection" });
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Keyboard source output port" }), { target: { value: "mic::out" } });
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Keyboard destination input port" }), { target: { value: "voice::in" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Add connection to draft" }));

    fireEvent.click(screen.getByRole("button", { name: "Insert Gate" }));
    expect(screen.getByText("Gate 1 inserted into the draft. Review and plan the changes before committing.")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Gate 1" })).toBeTruthy();
    expect(screen.getAllByRole("button", { name: "Insert Parametric EQ" })).toHaveLength(2);
    expect(screen.getAllByRole("button", { name: "Insert Compressor" })).toHaveLength(2);
    expect(screen.getAllByRole("button", { name: "Insert Limiter" })).toHaveLength(2);
  });

  it("disables topology mutations when no backend connection context exists", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const inserted = insertDraftMixer(connected, "edge-1");
    render(<DraftConnectionList session={inserted} onRemove={vi.fn()} onToggle={vi.fn()} />);

    expect(screen.getAllByRole("button", { name: /Insert mixer on/ })[0]).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "Remove and reconnect Mixer 1" })).toHaveProperty("disabled", true);
  });

  it("keeps topology actions available in the structured list view", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const onRemove = vi.fn();
    const onToggle = vi.fn();
    render(<BackendConnectionContext.Provider value={true}><GraphList session={connected} selectedNodeId="mic" onSelect={vi.fn()} onRemoveConnection={onRemove} onToggleConnection={onToggle} /></BackendConnectionContext.Provider>);

    fireEvent.click(screen.getByRole("button", { name: "Insert mixer on Microphone to Voice gain" }));
    expect(onRemove).toHaveBeenCalledWith(insertMixerActionId("edge-1"));
  });

  it("renders backend route provenance as an accessible path list", async () => {
    const inspectRoute = vi.fn(async () => ({
      destinationNode: "voice",
      reachable: true,
      complete: true,
      paths: [{ nodes: ["mic", "voice"], edges: ["edge-1"], channelMaps: [[1]], latencySamples: 48 }],
    }));
    render(<App backend={{ ...connectedPreviewBackend(), inspectRoute }} />);

    const routePanel = screen.getByRole("region", { name: "Receives audio from" });
    fireEvent.click(within(routePanel).getByRole("button", { name: "Refresh" }));
    await waitFor(() => expect(inspectRoute).toHaveBeenCalledWith("mic"));
    expect(await within(routePanel).findByRole("list", { name: "Reported audio paths" })).toBeTruthy();
    expect(within(routePanel).getByText(/Path 1: Microphone \[enabled\] → Voice gain \[enabled\]/)).toBeTruthy();
    expect(within(routePanel).getByText(/Channel map: \[1\]/)).toBeTruthy();
  });

  it("validates and explicitly commits a stopped session import", async () => {
    const imported = { ...demoSession, id: "imported-session", name: "Imported voice setup" };
    const planSessionImport = vi.fn(async () => ({ planId: "import-plan", expiresInMs: 300000, session: imported }));
    const commitSessionImport = vi.fn(async () => ({ session: imported, state: "stopped" as const, imported: true as const }));
    const backend = { ...connectedPreviewBackend(), planSessionImport, commitSessionImport };
    render(<App backend={backend} />);

    const transferPanel = screen.getByRole("region", { name: "Session transfer" });
    const file = new File([JSON.stringify(demoSession)], "voice.audiorouter.json", { type: "application/json" });
    fireEvent.change(within(transferPanel).getByLabelText("Import session configuration"), { target: { files: [file] } });
    await waitFor(() => expect(planSessionImport).toHaveBeenCalledWith(demoSession));
    expect(await within(transferPanel).findByText(/Validated import: Imported voice setup/)).toBeTruthy();

    fireEvent.click(within(transferPanel).getByRole("button", { name: "Commit stopped import" }));
    await waitFor(() => expect(commitSessionImport).toHaveBeenCalledWith("import-plan", expect.any(String)));
    expect(await within(transferPanel).findByText(/Imported stopped session Imported voice setup/)).toBeTruthy();
  });
});
