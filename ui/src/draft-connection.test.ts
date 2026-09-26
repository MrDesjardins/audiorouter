import { describe, expect, it } from "vitest";
import { appendDraftConnection, appendEqPresetNode, appendLibraryNode, appendVoiceChainPreset, defaultChannelMatrix, insertDraftMixer, insertDraftPluginProcessor, insertDraftProcessor, removeDraftConnection, removeSinglePathDraftMixer, setDraftConnectionEnabled } from "./draft";
import { templateSession } from "./templates";
import { demoSession } from "./fixtures";

describe("appendDraftConnection", () => {
  it("adds a deterministic identity matrix without changing revision", () => {
    const next = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    expect(next.edges).toEqual([{
      id: "edge-1",
      sourceNode: "mic",
      sourcePort: "out",
      destinationNode: "voice",
      destinationPort: "in",
      matrix: [1],
      enabled: true,
    }]);
    expect(next.revision).toBe(demoSession.revision);
  });

  it("creates a bounded mono-to-stereo map and rejects duplicate inputs", () => {
    const next = appendDraftConnection(demoSession, "mic", "out", "headphones", "in");
    expect(next.edges[0].matrix).toEqual([1, 1]);
    expect(() => appendDraftConnection(next, "mic", "out", "headphones", "in")).toThrow("already in the draft");
    expect(() => appendDraftConnection(next, "voice", "out", "headphones", "in")).toThrow("already has a connection");
  });

  it("downmixes a stereo source into a mono destination instead of dropping a channel", () => {
    const stereoSource = { ...demoSession, nodes: demoSession.nodes.map((node) => node.id === "mic" ? { ...node, ports: [{ ...node.ports[0], channels: 2 as 1 | 2 }] } : node) };
    const next = appendDraftConnection(stereoSource, "mic", "out", "voice", "in");
    expect(next.edges[0].matrix).toEqual([0.5, 0.5]);
  });

  it("removes only the requested draft edge", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const restored = removeDraftConnection(connected, "edge-1");
    expect(restored.edges).toEqual([]);
    expect(restored.revision).toBe(demoSession.revision);
    expect(() => removeDraftConnection(restored, "edge-1")).toThrow("Unknown draft connection");
  });

  it("toggles edge state without changing topology or revision", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const disabled = setDraftConnectionEnabled(connected, "edge-1", false);
    expect(disabled.edges[0]).toMatchObject({ id: "edge-1", enabled: false, sourceNode: "mic", destinationNode: "voice" });
    expect(disabled.revision).toBe(demoSession.revision);
    expect(() => setDraftConnectionEnabled(disabled, "missing", true)).toThrow("Unknown draft connection");
  });

  it("inserts and removes a single-path mixer as a previewable topology edit", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const inserted = insertDraftMixer(connected, "edge-1");
    const mixer = inserted.nodes.find((node) => node.kind === "mixer");
    expect(mixer).toMatchObject({ id: "mixer-1", name: "Mixer 1" });
    expect(mixer?.ports.every((port) => port.channels === 1)).toBe(true);
    expect(inserted.edges.map((edge) => [edge.sourceNode, edge.destinationNode])).toEqual([["mic", "mixer-1"], ["mixer-1", "voice"]]);
    const restored = removeSinglePathDraftMixer(inserted, "mixer-1");
    expect(restored.nodes.some((node) => node.id === "mixer-1")).toBe(false);
    expect(restored.edges.map((edge) => [edge.sourceNode, edge.destinationNode])).toEqual([["mic", "voice"]]);
  });

  it("keeps a stereo source width when inserting before a mono destination", () => {
    const stereoSource = { ...demoSession, nodes: demoSession.nodes.map((node) => node.id === "mic" ? { ...node, ports: [{ ...node.ports[0], channels: 2 as 1 | 2 }] } : node) };
    const connected = appendDraftConnection(stereoSource, "mic", "out", "voice", "in");
    const inserted = insertDraftMixer(connected, "edge-1");
    expect(inserted.nodes.find((node) => node.id === "mixer-1")?.ports).toEqual([
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ]);
    expect(inserted.edges.at(-1)?.matrix).toEqual([0.5, 0.5]);
  });

  it("preserves a custom matrix on the downstream preview edge", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const custom = { ...connected, edges: connected.edges.map((edge) => ({ ...edge, matrix: [0.5] })) };
    const inserted = insertDraftMixer(custom, "edge-1");
    expect(inserted.edges.at(-1)?.matrix).toEqual([0.5]);
    expect(removeSinglePathDraftMixer(inserted, "mixer-1").edges[0].matrix).toEqual([0.5]);
  });

  it("inserts a built-in processor on an existing path and preserves its downstream map", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const custom = { ...connected, edges: connected.edges.map((edge) => ({ ...edge, matrix: [0.5] })) };
    const inserted = insertDraftProcessor(custom, "edge-1", "gate");

    expect(inserted.nodes.find((node) => node.kind === "gate")).toMatchObject({
      id: "gate-1",
      name: "Gate 1",
      parameters: expect.objectContaining({ thresholdDb: -45 }),
    });
    expect(inserted.edges.map((edge) => [edge.sourceNode, edge.destinationNode])).toEqual([
      ["mic", "gate-1"],
      ["gate-1", "voice"],
    ]);
    expect(inserted.edges.at(-1)?.matrix).toEqual([0.5]);
    expect(inserted.revision).toBe(demoSession.revision);
  });

  it("inserts a scanned VST plugin on an existing path as an enabled node named after its file", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const custom = { ...connected, edges: connected.edges.map((edge) => ({ ...edge, matrix: [0.5] })) };
    const entry = {
      path: "C:\\Plugins\\ReaComp.vst3",
      identity: {
        path: "C:\\Plugins\\ReaComp.vst3",
        binaryPath: "C:\\Plugins\\ReaComp.vst3",
        format: "vst3" as const,
        architecture: "x64" as const,
        fileBytes: 8192,
        sha256: "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
        vendor: "Cockos",
        version: "1.0",
        classIds: ["reacomp-class"],
        compatibility: "supportedVst3X64" as const,
      },
      error: null,
      errorCode: null,
    };
    const inserted = insertDraftPluginProcessor(custom, "edge-1", entry);

    const plugin = inserted.nodes.find((node) => node.kind === "plugin");
    expect(plugin).toMatchObject({
      id: "plugin-1",
      name: "ReaComp 1",
      enabled: true,
      parameters: expect.objectContaining({ path: "C:\\Plugins\\ReaComp.vst3", format: "vst3" }),
    });
    expect(plugin?.ports.every((port) => port.channels === 1)).toBe(true);
    expect(inserted.edges.map((edge) => [edge.sourceNode, edge.destinationNode])).toEqual([
      ["mic", "plugin-1"],
      ["plugin-1", "voice"],
    ]);
    expect(inserted.edges.at(-1)?.matrix).toEqual([0.5]);
    expect(inserted.revision).toBe(demoSession.revision);
  });

  it("rejects inserting a plugin into an unknown connection", () => {
    const entry = {
      path: "C:\\Plugins\\ReaComp.vst3",
      identity: {
        path: "C:\\Plugins\\ReaComp.vst3",
        binaryPath: "C:\\Plugins\\ReaComp.vst3",
        format: "vst3" as const,
        architecture: "x64" as const,
        fileBytes: 8192,
        sha256: "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
        vendor: "Cockos",
        version: "1.0",
        classIds: ["reacomp-class"],
        compatibility: "supportedVst3X64" as const,
      },
      error: null,
      errorCode: null,
    };
    expect(() => insertDraftPluginProcessor(demoSession, "missing-edge", entry)).toThrow("Unknown draft connection");
  });

  it("inserts every advertised in-house processor on a connected path", () => {
    const kinds = ["gain", "mute", "parametricEq", "graphicEq", "compressor", "gate", "limiter", "delay", "pitch"] as const;
    for (const kind of kinds) {
      const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
      const inserted = insertDraftProcessor(connected, "edge-1", kind);
      const processor = inserted.nodes.find((node) => node.id === `${kind}-1`);
      expect(processor?.ports).toEqual([
        { name: "in", direction: "input", channels: 1 },
        { name: "out", direction: "output", channels: 1 },
      ]);
      expect(inserted.edges.map((edge) => [edge.sourceNode, edge.destinationNode])).toEqual([
        ["mic", processor?.id],
        [processor?.id, "voice"],
      ]);
    }
  });

  it("expands each EQ preset into an inspectable ordinary node", () => {
    const hum = appendEqPresetNode(demoSession, "hum50Hz");
    const humNode = hum.nodes.at(-1);
    expect(humNode).toMatchObject({
      kind: "parametricEq",
      parameters: expect.objectContaining({ band0Enabled: true, band0Type: "notch", band0FrequencyHz: 50, band0Q: 8 }),
    });
    const neutral = appendEqPresetNode(demoSession, "voiceNeutral").nodes.at(-1);
    expect(neutral).toMatchObject({
      kind: "parametricEq",
      parameters: expect.objectContaining({ band0Enabled: false, band0Type: "peaking", band0FrequencyHz: 1000, band0Q: 1 }),
    });
    expect(hum.edges).toEqual(demoSession.edges);
    expect(hum.revision).toBe(demoSession.revision);
  });

  it("inserts the voice chain into one path but leaves a disconnected draft unwired", () => {
    const connected = appendVoiceChainPreset(templateSession("gaming-discord"), "voiceGateAndCompression");
    expect(connected.nodes.map((node) => node.kind)).toEqual(["physicalInput", "gain", "physicalOutput", "gate", "compressor", "limiter"]);
    expect(connected.edges.map((edge) => [edge.sourceNode, edge.destinationNode])).toEqual(expect.arrayContaining([
      ["mic", "gate-1"], ["gate-1", "compressor-1"], ["compressor-1", "limiter-1"], ["limiter-1", "voice"], ["voice", "headphones"],
    ]));
    expect(connected.edges).toHaveLength(5);
    const unwired = appendVoiceChainPreset(demoSession, "voiceNeutral");
    expect(unwired.nodes.at(-1)).toMatchObject({ kind: "limiter", name: "Limiter 1" });
    expect(unwired.edges).toEqual([]);
  });

  it("refuses to remove a mixer with ambiguous topology", () => {
    const mixer = appendLibraryNode(demoSession, "mixer");
    expect(() => removeSinglePathDraftMixer(mixer, "mixer-1")).toThrow("exactly one incoming and one outgoing");
  });

  it("refuses to reconnect a mixer with a malformed channel matrix", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const inserted = insertDraftMixer(connected, "edge-1");
    const malformed = { ...inserted, edges: inserted.edges.map((edge) => edge.sourceNode === "mic" ? { ...edge, matrix: [] } : edge) };
    expect(() => removeSinglePathDraftMixer(malformed, "mixer-1")).toThrow("valid channel matrices");
  });
});

describe("defaultChannelMatrix", () => {
  it("maps mono to mono as identity", () => {
    expect(defaultChannelMatrix(1, 1)).toEqual([1]);
  });

  it("fans mono out to stereo by duplicating the source into both channels", () => {
    expect(defaultChannelMatrix(1, 2)).toEqual([1, 1]);
  });

  it("maps stereo to stereo as identity, not a diagonal-adjacent swap", () => {
    expect(defaultChannelMatrix(2, 2)).toEqual([1, 0, 0, 1]);
  });

  it("downmixes stereo into mono by averaging both source channels", () => {
    expect(defaultChannelMatrix(2, 1)).toEqual([0.5, 0.5]);
  });
});
