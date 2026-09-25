/** @vitest-environment jsdom */

import { cleanup, fireEvent, render, within } from "@testing-library/react";
import { createElement } from "react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import type { Session } from "@audiorouter/contracts";
import { demoSession } from "./fixtures";
import { AudioFileNodeControls, deletedConnectionIds, deletedNodeIds, edgeSignalForConnection, eqBandCoordinates, libraryDropPosition, signalStrokeWidth, telemetrySignalActive, testSignalHasPhysicalOutputPath } from "./SessionFlowCanvas";
import { SessionFlowCanvas } from "./SessionFlowCanvas";

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

afterEach(cleanup);

const testSignalSession: Session = {
  ...demoSession,
  nodes: [
    { id: "signal", kind: "testSignal", typeVersion: 1, name: "Test Signal", enabled: true, bypass: false, parameters: { frequencyHz: 440, levelDb: -30, durationMs: 10_000 }, ports: [{ name: "main", direction: "output", channels: 2 }] },
    { id: "gain", kind: "gain", typeVersion: 1, name: "Gain", enabled: true, bypass: false, parameters: { gainDb: 0 }, ports: [{ name: "in", direction: "input", channels: 2 }, { name: "out", direction: "output", channels: 2 }] },
    { id: "output", kind: "physicalOutput", typeVersion: 1, name: "Output", enabled: true, bypass: false, parameters: {}, ports: [{ name: "main", direction: "input", channels: 2 }] },
  ],
  edges: [
    { id: "signal-gain", sourceNode: "signal", sourcePort: "main", destinationNode: "gain", destinationPort: "in", matrix: [1, 0, 0, 1], enabled: true },
    { id: "gain-output", sourceNode: "gain", sourcePort: "out", destinationNode: "output", destinationPort: "main", matrix: [1, 0, 0, 1], enabled: true },
  ],
};

describe("Test Signal playback path", () => {
  it("shows measured Test Signal and file playback even when capture privacy mute is latched", () => {
    const diagnostics = { audio: { state: "available", reason: "" }, privacyMute: { muted: true }, nodeTelemetry: [{ nodeId: "output", meter: { peakDb: -15, rmsDb: -18 } }] } as Parameters<typeof edgeSignalForConnection>[2];
    for (const edge of testSignalSession.edges) expect(edgeSignalForConnection(edge, testSignalSession, diagnostics, true, true).state).toBe("active");
    const fileSession: Session = { ...testSignalSession, nodes: testSignalSession.nodes.map((node) => node.id === "signal" ? { ...node, kind: "audioFile" } : node) };
    expect(edgeSignalForConnection(fileSession.edges[1], fileSession, diagnostics, true, true).state).toBe("active");
    const captureSession: Session = { ...testSignalSession, nodes: testSignalSession.nodes.map((node) => node.id === "signal" ? { ...node, kind: "physicalInput" } : node) };
    for (const edge of captureSession.edges) expect(edgeSignalForConnection(edge, captureSession, diagnostics, true, true).state).toBe("muted");
  });

  it("keeps Play disabled until a committed graph reaches an enabled physical output", () => {
    expect(testSignalHasPhysicalOutputPath(testSignalSession, "signal")).toBe(true);
    expect(testSignalHasPhysicalOutputPath({ ...testSignalSession, edges: [] }, "signal")).toBe(false);
    expect(testSignalHasPhysicalOutputPath({ ...testSignalSession, nodes: testSignalSession.nodes.map((node) => node.id === "output" ? { ...node, enabled: false } : node) }, "signal")).toBe(false);
  });
});

describe("Audio file source controls", () => {
  it("keeps the compact node transport wired to the selected source", () => {
    const node = { id: "file", kind: "audioFile" as const, typeVersion: 1 as const, name: "Calibration sample", enabled: true, bypass: false, parameters: { mediaId: "media-1", fileName: "voice.wav", loop: false }, ports: [{ name: "out", direction: "output" as const, channels: 2 as const }] };
    const transport = vi.fn();
    const { getByRole, queryByRole } = render(createElement(AudioFileNodeControls, { node, state: "paused", onTransport: transport }));
    fireEvent.click(getByRole("button", { name: "Play Calibration sample" }));
    expect(transport).toHaveBeenCalledWith("file", "play");
    expect((getByRole("button", { name: "Stop Calibration sample" }) as HTMLButtonElement).disabled).toBe(false);
    expect(queryByRole("button", { name: "Pause Calibration sample" })).toBeNull();
    cleanup();
    const playing = render(createElement(AudioFileNodeControls, { node, state: "playing", onTransport: transport }));
    expect((playing.getByRole("button", { name: "Stop Calibration sample" }) as HTMLButtonElement).disabled).toBe(false);
  });
});

describe("canvas library drop positions", () => {
  it("maps EQ bands across the audible frequency range", () => {
    expect(eqBandCoordinates(20, 0).x).toBeCloseTo(8);
    expect(eqBandCoordinates(20_000, 0).x).toBeCloseTo(192);
    expect(eqBandCoordinates(1_000, 12).y).toBeLessThan(eqBandCoordinates(1_000, -12).y);
  });

  it("only marks a link active for a real bounded meter signal", () => {
    expect(telemetrySignalActive({ nodeId: "meter", kind: "meter", meter: null, processor: null, plugin: null })).toBe(false);
    expect(telemetrySignalActive({ nodeId: "meter", kind: "meter", meter: { peakDb: -60, rmsDb: -70, clippedSamples: 0, channelPeakDb: [], channelRmsDb: [], channelClippedSamples: [] }, processor: null, plugin: null })).toBe(false);
    expect(telemetrySignalActive({ nodeId: "meter", kind: "meter", meter: { peakDb: -12, rmsDb: -18, clippedSamples: 0, channelPeakDb: [], channelRmsDb: [], channelClippedSamples: [] }, processor: null, plugin: null })).toBe(true);
  });

  it("maps fresh backend volume to a bounded thick directional-flow presentation", () => {
    const session = {
      ...demoSession,
      edges: [
        { id: "mic-voice", sourceNode: "mic", sourcePort: "out", destinationNode: "voice", destinationPort: "in", matrix: [], enabled: true },
        { id: "voice-headphones", sourceNode: "voice", sourcePort: "out", destinationNode: "headphones", destinationPort: "in", matrix: [], enabled: true },
      ],
    };
    const diagnostics = {
      build: "test", backend: "control-plane" as const, storage: "memory" as const,
      audio: { state: "available" as const, reason: "" }, nativeAdapter: "running" as const,
      nativeAdapterKind: null, nativeSessionId: null,
      schedulerTelemetry: { activeGeneration: 4, activeSampleRateHz: 48_000, inputOverruns: 0, inputUnderruns: 0, outputOverruns: 0, outputUnderruns: 0, processedQuanta: 1, repairedSamples: 0, xruns: 0, processingTimeNsTotal: 0, processingTimeNsMax: 0, deadlineMisses: 0, deadlineLatenessNsTotal: 0, deadlineLatenessNsMax: 0 },
      nodeTelemetry: [{ nodeId: "headphones", kind: "physicalOutput", meter: { peakDb: -8, rmsDb: -18, clippedSamples: 0, channelPeakDb: [-8, -8], channelRmsDb: [-18, -18], channelClippedSamples: [0, 0] }, processor: null, plugin: null }],
      privacyMute: { muted: false, persistence: "memory" as const }, recovery: { safeMode: false, recentCrashes: 0, persistence: "memory" as const }, eventLog: { latestSequence: 0, retained: 0 }, redacted: true as const,
    };
    const edge = session.edges[0];
    const active = edgeSignalForConnection(edge, session, diagnostics, true, true);
    expect(active).toMatchObject({ active: true, state: "active", levelDb: -18 });
    expect(signalStrokeWidth(active.levelDb)).toBeGreaterThan(2.5);
    expect(signalStrokeWidth(-60)).toBe(2.5);
    expect(signalStrokeWidth(-100)).toBe(2.5);
    expect(signalStrokeWidth(20)).toBeLessThanOrEqual(12.5);
    expect(edgeSignalForConnection(edge, session, diagnostics, false, true).state).toBe("stopped");
    expect(edgeSignalForConnection(edge, session, diagnostics, true, false).state).toBe("stale");
    expect(edgeSignalForConnection(edge, session, { ...diagnostics, privacyMute: { muted: true, persistence: "memory" } }, true, true).state).toBe("muted");
    expect(edgeSignalForConnection(edge, session, { ...diagnostics, nodeTelemetry: [{ ...diagnostics.nodeTelemetry[0], meter: { ...diagnostics.nodeTelemetry[0].meter!, rmsDb: -80 } }] }, true, true).state).toBe("silent");
    expect(edgeSignalForConnection({ ...edge, enabled: false }, session, diagnostics, true, true).state).toBe("disabled");

  });

  it("keeps meter inference bounded to an unambiguous downstream chain", () => {
    const branched = {
      ...demoSession,
      nodes: [...demoSession.nodes, { id: "other", kind: "gain" as const, typeVersion: 1 as const, name: "Other", enabled: true, bypass: false, parameters: {}, ports: [{ name: "in", direction: "input" as const, channels: 1 as const }, { name: "out", direction: "output" as const, channels: 1 as const }] }],
      edges: [
        { id: "mic-voice", sourceNode: "mic", sourcePort: "out", destinationNode: "voice", destinationPort: "in", matrix: [], enabled: true },
        { id: "voice-headphones", sourceNode: "voice", sourcePort: "out", destinationNode: "headphones", destinationPort: "in", matrix: [], enabled: true },
        { id: "voice-other", sourceNode: "voice", sourcePort: "out", destinationNode: "other", destinationPort: "in", matrix: [], enabled: true },
      ],
    };
    const diag = { audio: { state: "available" as const, reason: "" }, privacyMute: { muted: false }, nodeTelemetry: [{ nodeId: "headphones", meter: { rmsDb: -18, peakDb: -8 } }] } as Parameters<typeof edgeSignalForConnection>[2];
    expect(edgeSignalForConnection(branched.edges[0], branched, diag, true, true).state).toBe("unmetered");
  });

  it("converts viewport coordinates into bounded canvas coordinates", () => {
    expect(libraryDropPosition(240, 180, { left: 100, top: 50 })).toEqual({ x: 120, y: 110 });
  });

  it("falls back to the canvas origin for non-finite event coordinates", () => {
    expect(libraryDropPosition(Number.NaN, Number.POSITIVE_INFINITY, { left: 100, top: 50 })).toEqual({ x: 0, y: 0 });
  });

  it("extracts only non-empty edge identities for draft deletion", () => {
    expect(deletedConnectionIds([{ id: "edge-1" }, { id: "" }, { id: "edge-2" }])).toEqual(["edge-1", "edge-2"]);
  });

  it("extracts only non-empty node identities for draft deletion", () => {
    expect(deletedNodeIds([{ id: "node-1" }, { id: "" }, { id: "node-2" }])).toEqual(["node-1", "node-2"]);
  });

  it("routes a library drop to the backend draft callback at canvas coordinates", () => {
    const onAddLibraryNode = vi.fn(() => "compressor-1");
    const { getByLabelText } = render(createElement(SessionFlowCanvas, {
      session: demoSession,
      selectedNodeId: "mic",
      onSelect: vi.fn(),
      onConnect: vi.fn(),
      onAddLibraryNode,
    }));
    const canvas = getByLabelText("Signal-flow graph");
    vi.spyOn(canvas, "getBoundingClientRect").mockReturnValue({
      left: 100,
      top: 50,
      right: 900,
      bottom: 650,
      width: 800,
      height: 600,
      x: 100,
      y: 50,
      toJSON: () => ({}),
    });

    const drop = new Event("drop", { bubbles: true });
    Object.defineProperties(drop, {
      clientX: { value: 240 },
      clientY: { value: 180 },
      dataTransfer: {
        value: {
          types: ["application/x-audiorouter-library-kind"],
          getData: () => "compressor",
        },
      },
    });
    fireEvent(canvas, drop);

    expect(onAddLibraryNode).toHaveBeenCalledWith("compressor", { x: 120, y: 110 });
  });

  it("rejects stale or malformed library drop kinds before backend mutation", () => {
    const onAddLibraryNode = vi.fn(() => "invalid-1");
    const onConnect = vi.fn();
    const { getByLabelText } = render(createElement(SessionFlowCanvas, {
      session: demoSession,
      selectedNodeId: "mic",
      onSelect: vi.fn(),
      onConnect,
      onAddLibraryNode,
    }));
    const canvas = getByLabelText("Signal-flow graph");
    const drop = new Event("drop", { bubbles: true });
    Object.defineProperty(drop, "dataTransfer", {
      value: {
        types: ["application/x-audiorouter-library-kind"],
        getData: () => "stale-processor-kind",
      },
    });

    fireEvent(canvas, drop);

    expect(onAddLibraryNode).not.toHaveBeenCalled();
    expect(onConnect).not.toHaveBeenCalled();
  });

  it("offers a keyboard-accessible click path for adding a processor", () => {
    const onAddLibraryNode = vi.fn(() => "gate-1");
    const { getByRole } = render(createElement(SessionFlowCanvas, {
      session: demoSession,
      selectedNodeId: "mic",
      onSelect: vi.fn(),
      onConnect: vi.fn(),
      onAddLibraryNode,
    }));

    fireEvent.click(getByRole("button", { name: "Gate" }));

    expect(onAddLibraryNode).toHaveBeenCalledWith("gate", { x: 840, y: 0 });
  });

  it("offers device nodes and distinguishes deferred managed virtual buses", () => {
    const { getByLabelText } = render(createElement(SessionFlowCanvas, {
      session: demoSession,
      selectedNodeId: "mic",
      onSelect: vi.fn(),
      onConnect: vi.fn(),
      onAddLibraryNode: vi.fn(() => "processor-1"),
    }));
    const shelf = within(getByLabelText("Drag processors to canvas"));

    expect(shelf.getByRole("button", { name: "Gain" })).toBeTruthy();
    expect(shelf.getByRole("button", { name: "Input device" })).toBeTruthy();
    expect(shelf.getByRole("button", { name: "Output device" })).toBeTruthy();
    expect(shelf.getByRole("button", { name: "Virtual capture sink" })).toBeTruthy();
    expect(shelf.getByRole("button", { name: "Virtual render source" })).toBeTruthy();
    expect(shelf.getByRole("button", { name: "Recorder" })).toBeTruthy();
  });

  it("keeps deferred managed virtual entries visible but unavailable", () => {
    const onConnect = vi.fn();
    const { getByLabelText } = render(createElement(SessionFlowCanvas, {
      session: demoSession,
      selectedNodeId: "mic",
      onSelect: vi.fn(),
      onConnect,
      onAddLibraryNode: vi.fn(() => "processor-1"),
    }));
    const shelf = within(getByLabelText("Drag processors to canvas"));
    expect(shelf.getByRole("button", { name: "Virtual capture sink" })).toHaveProperty("disabled", true);
    expect(shelf.getByRole("button", { name: "Virtual render source" })).toHaveProperty("disabled", true);
    expect(onConnect).not.toHaveBeenCalled();
  });

  it.each([
    ["Input device", "physicalInput"],
    ["Output device", "physicalOutput"],
    ["Mixer", "mixer"],
    ["Compressor", "compressor"],
    ["Recorder", "recorder"],
  ])("places %s from the shelf through the draft adapter", (label, kind) => {
    const onAddLibraryNode = vi.fn(() => `${kind}-1`);
    const { getByLabelText } = render(createElement(SessionFlowCanvas, {
      session: demoSession,
      selectedNodeId: "mic",
      onSelect: vi.fn(),
      onConnect: vi.fn(),
      onAddLibraryNode,
    }));
    const canvas = getByLabelText("Signal-flow graph");
    vi.spyOn(canvas, "getBoundingClientRect").mockReturnValue({
      left: 100, top: 50, right: 900, bottom: 650, width: 800, height: 600,
      x: 100, y: 50, toJSON: () => ({}),
    });
    const shelf = within(getByLabelText("Drag processors to canvas"));
    const button = shelf.getByRole("button", { name: label });
    const dataTransfer = {
      types: ["application/x-audiorouter-library-kind"],
      setData: vi.fn(),
      getData: vi.fn(() => kind),
    };
    fireEvent.dragStart(button, { dataTransfer });
    const drop = new Event("drop", { bubbles: true });
    Object.defineProperties(drop, {
      clientX: { value: 240 },
      clientY: { value: 180 },
      dataTransfer: { value: dataTransfer },
    });
    fireEvent(canvas, drop);

    expect(onAddLibraryNode).toHaveBeenCalledWith(kind, { x: 120, y: 110 });
    expect(JSON.parse(window.localStorage.getItem("audiorouter.ui.layout.demo-session") ?? "null")).toMatchObject({
      [`${kind}-1`]: { x: 120, y: 110 },
    });
  });

  it("retains the drop position when the app creates a virtual bus node", () => {
    window.localStorage.clear();
    const onConnect = vi.fn(() => "virtual-capture-sink-1");
    const { getByLabelText } = render(createElement(SessionFlowCanvas, {
      session: demoSession,
      selectedNodeId: "mic",
      onSelect: vi.fn(),
      onConnect,
      onAddLibraryNode: vi.fn(),
    }));
    const canvas = getByLabelText("Signal-flow graph");
    vi.spyOn(canvas, "getBoundingClientRect").mockReturnValue({
      left: 100, top: 50, right: 900, bottom: 650, width: 800, height: 600,
      x: 100, y: 50, toJSON: () => ({}),
    });
    const drop = new Event("drop", { bubbles: true });
    Object.defineProperties(drop, {
      clientX: { value: 240 },
      clientY: { value: 180 },
      dataTransfer: { value: { types: ["application/x-audiorouter-library-kind"], getData: () => "virtualCaptureSink" } },
    });
    fireEvent(canvas, drop);

    expect(onConnect).toHaveBeenCalledWith(expect.objectContaining({ sourceHandle: "virtualCaptureSink" }), { x: 120, y: 110 });
    expect(JSON.parse(window.localStorage.getItem("audiorouter.ui.layout.demo-session") ?? "null")).toEqual({
      "virtual-capture-sink-1": { x: 120, y: 110 },
    });
  });

  it("preserves both layout entries when processors are added rapidly", () => {
    window.localStorage.clear();
    let nextId = 1;
    const onAddLibraryNode = vi.fn(() => `gain-${nextId++}`);
    const { getByLabelText } = render(createElement(SessionFlowCanvas, {
      session: demoSession,
      selectedNodeId: "mic",
      onSelect: vi.fn(),
      onConnect: vi.fn(),
      onAddLibraryNode,
    }));
    const shelf = within(getByLabelText("Drag processors to canvas"));

    fireEvent.click(shelf.getByRole("button", { name: "Gain" }));
    fireEvent.click(shelf.getByRole("button", { name: "Gain" }));

    expect(JSON.parse(window.localStorage.getItem("audiorouter.ui.layout.demo-session") ?? "null")).toEqual({
      "gain-1": { x: 840, y: 0 },
      "gain-2": { x: 0, y: 230 },
    });
  });

  it("labels input, native tool, VST, and output nodes with a distinct family badge", () => {
    const sessionWithPlugin = {
      ...demoSession,
      nodes: [
        ...demoSession.nodes,
        {
          id: "reacomp",
          kind: "plugin" as const,
          typeVersion: 1 as const,
          name: "ReaComp",
          enabled: false,
          bypass: false,
          parameters: { path: "C:\\Plugins\\ReaComp.vst3", format: "vst3", fingerprint: "abc", classId: "default" },
          ports: [{ name: "in", direction: "input" as const, channels: 1 as const }, { name: "out", direction: "output" as const, channels: 1 as const }],
        },
      ],
    };
    const { getByLabelText } = render(createElement(SessionFlowCanvas, {
      session: sessionWithPlugin,
      selectedNodeId: "mic",
      onSelect: vi.fn(),
      onConnect: vi.fn(),
    }));

    const mic = within(getByLabelText("Microphone, physicalInput"));
    expect(mic.getByText("Input")).toBeTruthy();
    expect(mic.getByText("Physical Input")).toBeTruthy();

    const gain = within(getByLabelText("Voice gain, gain"));
    expect(gain.getByText("Native")).toBeTruthy();
    expect(gain.getByText("Gain", { selector: ".node-kind" })).toBeTruthy();

    const headphones = within(getByLabelText("Headphones, physicalOutput"));
    expect(headphones.getByText("Output")).toBeTruthy();

    const plugin = within(getByLabelText("ReaComp, plugin"));
    expect(plugin.getByText("VST")).toBeTruthy();
    expect(plugin.getByText("VST3 Plugin")).toBeTruthy();
  });
});
