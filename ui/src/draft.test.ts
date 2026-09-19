import { describe, expect, it } from "vitest";
import { appendApplicationCaptureNode, appendDraftConnection, appendEndpointLoopbackNode, appendLibraryNode, appendPluginPlaceholderNode, appendVirtualBusNode, duplicateDraftNode, GAIN_MAX_DB, GAIN_MIN_DB, removeDraftNode, resetNodeDraftParameters, setNodeDraftName, setNodeDraftParameter, setSessionDraftName } from "./draft";
import { demoSession } from "./fixtures";

describe("appendLibraryNode", () => {
  it("adds a bounded Test Signal source with conservative defaults", () => {
    const next = appendLibraryNode(demoSession, "testSignal");
    expect(next.nodes.at(-1)).toMatchObject({
      kind: "testSignal",
      parameters: { frequencyHz: 440, levelDb: -18, durationMs: 1000 },
      ports: [{ name: "out", direction: "output", channels: 2 }],
    });
  });

  it("adds a recorder sink as a stopped graph draft node", () => {
    const next = appendLibraryNode(demoSession, "recorder");
    const recorder = next.nodes.at(-1);
    expect(recorder).toMatchObject({
      kind: "recorder",
      enabled: true,
      bypass: false,
      ports: [
        { name: "in", direction: "input", channels: 1 },
        { name: "out", direction: "output", channels: 1 },
      ],
    });
  });

  it("adds a valid built-in processor without changing revision or edges", () => {
    const next = appendLibraryNode(demoSession, "gain");
    const added = next.nodes.at(-1);
    expect(added).toMatchObject({
      id: "gain-1",
      kind: "gain",
      name: "Gain 1",
      enabled: true,
      bypass: false,
      parameters: { gainDb: 0 },
    });
    expect(added?.ports).toEqual([
      { name: "in", direction: "input", channels: 1 },
      { name: "out", direction: "output", channels: 1 },
    ]);
    expect(next.revision).toBe(demoSession.revision);
    expect(next.edges).toEqual(demoSession.edges);
  });

  it("chooses the next deterministic id when a processor already exists", () => {
    const once = appendLibraryNode(demoSession, "meter");
    const twice = appendLibraryNode(once, "meter");
    expect(twice.nodes.slice(-2).map((node) => node.id)).toEqual(["meter-1", "meter-2"]);
  });

  it("adds supported scan identity as a stopped plugin placeholder", () => {
    const next = appendPluginPlaceholderNode(demoSession, {
      path: "C:\\Plugins\\effect.dll",
      identity: {
        path: "C:\\Plugins\\effect.dll",
        binaryPath: "C:\\Plugins\\effect.dll",
        format: "vst2",
        architecture: "x64",
        fileBytes: 10,
        sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        vendor: "Test vendor",
        version: "1.0",
        classIds: ["test-class"],
        compatibility: "supportedVst2X64Gated",
      },
      error: null,
      errorCode: null,
    });
    expect(next.nodes.at(-1)).toMatchObject({
      kind: "plugin",
      enabled: false,
      parameters: { format: "vst2", fingerprint: expect.any(String), classId: "test-class" },
    });
    expect(next.edges).toEqual(demoSession.edges);
  });

  it("renames without changing identity, revision, or topology", () => {
    const renamed = setNodeDraftName(demoSession, "voice", "  Voice processing  ");
    expect(renamed.nodes.find((node) => node.id === "voice")?.name).toBe("Voice processing");
    expect(renamed.id).toBe(demoSession.id);
    expect(renamed.revision).toBe(demoSession.revision);
    expect(renamed.edges).toEqual(demoSession.edges);
    expect(() => setNodeDraftName(demoSession, "voice", " ")).toThrow("cannot be empty");
    expect(() => setNodeDraftName(demoSession, "voice", "x".repeat(121))).toThrow("120");
  });

  it("removes a node and only its incident edges", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const reduced = removeDraftNode(connected, "voice");
    expect(reduced.nodes.map((node) => node.id)).toEqual(["mic", "headphones"]);
    expect(reduced.edges).toEqual([]);
    expect(reduced.revision).toBe(demoSession.revision);
    expect(() => removeDraftNode(reduced, "voice")).toThrow("Unknown draft node");
  });

  it("keeps one-node drafts valid", () => {
    const single = { ...demoSession, nodes: [demoSession.nodes[0]], edges: [] };
    expect(() => removeDraftNode(single, demoSession.nodes[0].id)).toThrow("at least one node");
  });

  it("duplicates a node without copying edges or changing the revision", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const duplicated = duplicateDraftNode(connected, "voice");
    const copy = duplicated.nodes.at(-1);
    expect(copy).toMatchObject({ id: "voice-copy-1", name: "Voice gain copy 1", kind: "gain", parameters: { gainDb: 0 } });
    expect(duplicated.edges).toEqual(connected.edges);
    expect(duplicated.revision).toBe(demoSession.revision);
    const twice = duplicateDraftNode(duplicated, "voice");
    expect(twice.nodes.at(-1)?.id).toBe("voice-copy-2");
  });

  it("resets supported parameters without changing topology", () => {
    const changed = { ...demoSession, nodes: demoSession.nodes.map((node) => node.id === "voice" ? { ...node, parameters: { gainDb: 8 } } : node) };
    const reset = resetNodeDraftParameters(changed, "voice");
    expect(reset.nodes.find((node) => node.id === "voice")?.parameters).toEqual({ gainDb: 0 });
    expect(reset.revision).toBe(demoSession.revision);
    expect(() => resetNodeDraftParameters(demoSession, "mic")).not.toThrow();
  });

  it("resets every built-in processor to its declared draft defaults", () => {
    const withEq = appendLibraryNode(demoSession, "parametricEq");
    const changed = {
      ...withEq,
      nodes: withEq.nodes.map((node) => node.kind === "parametricEq"
        ? { ...node, parameters: { frequencyHz: 12_000, q: 8, gainDb: 18 } }
        : node),
    };
    const reset = resetNodeDraftParameters(changed, "parametricEq-1");
    expect(reset.nodes.at(-1)?.parameters).toEqual(expect.objectContaining({ frequencyHz: 1000, q: 1, gainDb: 0, band7Enabled: false, band7Type: "peaking" }));
    expect(reset.edges).toEqual(demoSession.edges);
  });

  it("keeps gain drafts inside the documented range", () => {
    expect(setNodeDraftParameter(demoSession, "voice", "gainDb", GAIN_MAX_DB).nodes[1].parameters.gainDb).toBe(24);
    expect(setNodeDraftParameter(demoSession, "voice", "gainDb", GAIN_MIN_DB).nodes[1].parameters.gainDb).toBe(-60);
    expect(() => setNodeDraftParameter(demoSession, "voice", "gainDb", 24.1)).toThrow("between -60 and 24");
  });

  it("normalizes and bounds session names", () => {
    expect(setSessionDraftName(demoSession, "  Streaming setup  ").name).toBe("Streaming setup");
    expect(() => setSessionDraftName(demoSession, " ")).toThrow("cannot be empty");
    expect(() => setSessionDraftName(demoSession, "x".repeat(121))).toThrow("120");
  });
});

describe("appendEndpointLoopbackNode", () => {
  it("adds an exact, stopped endpoint-loopback source without changing topology", () => {
    const next = appendEndpointLoopbackNode(demoSession, "  endpoint-render-1  ");
    expect(next.nodes.at(-1)).toMatchObject({
      id: "endpoint-loopback-1",
      kind: "endpointLoopback",
      enabled: false,
      parameters: { endpointId: "endpoint-render-1" },
      ports: [{ name: "out", direction: "output", channels: 2 }],
    });
    expect(next.revision).toBe(demoSession.revision);
    expect(next.edges).toEqual(demoSession.edges);
  });

  it("rejects an absent exact endpoint identity", () => {
    expect(() => appendEndpointLoopbackNode(demoSession, "  ")).toThrow("exact render endpoint");
  });
});

describe("appendVirtualBusNode", () => {
  it("adds stopped source and sink nodes bound to the exact bus identity", () => {
    const source = appendVirtualBusNode(demoSession, " bus-1 ", "renderSource");
    const sink = appendVirtualBusNode(source, "bus-1", "captureSink");
    expect(source.nodes.at(-1)).toMatchObject({ kind: "virtualRenderSource", enabled: false, parameters: { busId: "bus-1" }, ports: [{ name: "out", direction: "output", channels: 2 }] });
    expect(sink.nodes.at(-1)).toMatchObject({ kind: "virtualCaptureSink", enabled: false, parameters: { busId: "bus-1" }, ports: [{ name: "in", direction: "input", channels: 2 }] });
    expect(sink.edges).toEqual(demoSession.edges);
  });

  it("connects virtual source and sink nodes to the surrounding graph", () => {
    const withSource = appendVirtualBusNode(demoSession, "bus-1", "renderSource");
    const withBoth = appendVirtualBusNode(withSource, "bus-1", "captureSink");
    const routed = appendDraftConnection(
      appendDraftConnection(
        withBoth,
        "virtual-render-source-1",
        "out",
        "headphones",
        "in",
      ),
      "mic",
      "out",
      "virtual-capture-sink-1",
      "in",
    );
    expect(routed.edges).toEqual(expect.arrayContaining([
      expect.objectContaining({ sourceNode: "virtual-render-source-1", destinationNode: "headphones" }),
      expect.objectContaining({ sourceNode: "mic", destinationNode: "virtual-capture-sink-1" }),
    ]));
  });

  it("rejects an absent bus identity", () => {
    expect(() => appendVirtualBusNode(demoSession, " ", "captureSink")).toThrow("existing virtual bus identity");
  });
});

describe("destination connection creation", () => {
  it.each([
    ["physical output", () => demoSession, "headphones", "in"],
    ["processor", () => appendLibraryNode(demoSession, "compressor"), "compressor-1", "in"],
    ["recorder", () => appendLibraryNode(demoSession, "recorder"), "recorder-1", "in"],
    ["existing virtual capture sink", () => appendVirtualBusNode(demoSession, "cable-bus", "captureSink"), "virtual-capture-sink-1", "in"],
  ])("connects a source into a %s destination", (_label, makeSession, destinationNode, destinationPort) => {
    const next = appendDraftConnection(makeSession(), "mic", "out", destinationNode, destinationPort);
    expect(next.edges).toEqual(expect.arrayContaining([
      expect.objectContaining({ sourceNode: "mic", sourcePort: "out", destinationNode, destinationPort }),
    ]));
  });
});

describe("appendApplicationCaptureNode", () => {
  it("creates a stopped capture source bound to the observed process identity", () => {
    const next = appendApplicationCaptureNode(demoSession, {
      processId: 42,
      executable: "game.exe",
      executablePath: "C:\\Games\\game.exe",
      creationTime100ns: "123456789",
      audioActivity: "active",
      captureCapability: "observed",
      audioSessionCount: 1,
      activeAudioSessionCount: 1,
      captureSessionCount: 0,
      renderSessionCount: 1,
      audioDisplayNames: ["Game"],
    });
    expect(next.nodes.at(-1)).toMatchObject({
      id: "application-capture-1",
      kind: "applicationCapture",
      enabled: false,
      parameters: {
        executable: "game.exe",
        executablePath: "C:\\Games\\game.exe",
        processPolicy: "selectedInstance",
        processId: 42,
        creationTime100ns: "123456789",
      },
      ports: [{ name: "out", direction: "output", channels: 2 }],
    });
  });
});
