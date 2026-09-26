import { describe, expect, it } from "vitest";
import type { ApplicationInfo, Session } from "@audiorouter/contracts";
import { addSourceToOccupiedOutput, appendApplicationCaptureNode, applicationCaptureChoices, applicationOnlyRouteSource, independentPaths, needsNativePaths, unboundDeviceNodes, isParameterOnlyChange, pluginCatalog, STANDARD_PLUGIN_FOLDERS, mixerInputs, mixerRouteSources, pruneInactiveUpstream, mixerInputVolumeKey, mixedApplicationRouteOtherSources, rebindApplicationCaptureNode, appendDraftConnection, appendEndpointLoopbackNode, appendLibraryNode, appendPluginPlaceholderNode, appendVirtualBusNode, duplicateDraftNode, GAIN_MAX_DB, GAIN_MIN_DB, removeDraftNode, resetNodeDraftParameters, setNodeDraftName, setNodeDraftParameter, setSessionDraftName } from "./draft";
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
        { name: "in", direction: "input", channels: 2 },
        { name: "out", direction: "output", channels: 2 },
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
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ]);
    expect(next.revision).toBe(demoSession.revision);
    expect(next.edges).toEqual(demoSession.edges);
  });

  it("chooses the next deterministic id when a processor already exists", () => {
    const once = appendLibraryNode(demoSession, "meter");
    const twice = appendLibraryNode(once, "meter");
    expect(twice.nodes.slice(-2).map((node) => node.id)).toEqual(["meter-1", "meter-2"]);
  });

  it("adds a supported scan identity as an enabled plugin node", () => {
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
      enabled: true,
      name: "effect 1",
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

  it("inserts a visible mixer when a second source is connected to the occupied physical output", () => {
    const baseRoute = appendDraftConnection(demoSession, "mic", "out", "headphones", "in");
    const signal = appendLibraryNode(baseRoute, "testSignal");
    const routed = addSourceToOccupiedOutput(signal, "edge-1", "testSignal-1", "out");
    const mixer = routed.nodes.find((node) => node.kind === "mixer");
    expect(mixer).toBeDefined();
    expect(routed.edges.filter((edge) => edge.destinationNode === mixer?.id && edge.destinationPort === "in")).toHaveLength(2);
    expect(routed.edges).toEqual(expect.arrayContaining([
      expect.objectContaining({ sourceNode: "mic", destinationNode: mixer?.id }),
      expect.objectContaining({ sourceNode: "testSignal-1", destinationNode: mixer?.id }),
      expect.objectContaining({ sourceNode: mixer?.id, destinationNode: "headphones" }),
    ]));
  });
});

describe("application capture identity", () => {
  const app = (processId: number, executable: string, executablePath: string, creationTime100ns: string, audioSessionCount = 0): ApplicationInfo => ({
    processId, executable, executablePath, creationTime100ns,
    audioActivity: audioSessionCount > 0 ? "active" : "none",
    captureCapability: "notObserved",
    audioSessionCount, activeAudioSessionCount: audioSessionCount, captureSessionCount: 0, renderSessionCount: audioSessionCount,
    audioDisplayNames: [],
  });

  it("lists running applications without a microphone session and folds same-path helpers", () => {
    const discordRoot = app(36808, "Discord.exe", "C:\\Discord\\app-1.0.9259\\Discord.exe", "100");
    const discordHelper = app(26164, "Discord.exe", "C:\\Discord\\app-1.0.9259\\Discord.exe", "101");
    const browser = app(9, "browser.exe", "C:\\Browser\\browser.exe", "50", 1);
    const choices = applicationCaptureChoices([discordHelper, browser, discordRoot]);
    expect(choices.withAudio).toEqual([browser]);
    expect(choices.other).toEqual([discordRoot]);
  });

  it("shows one entry for an application whose helper processes use audio", () => {
    const root = app(36808, "Discord.exe", "C:\\Discord\\app-1.0.9259\\Discord.exe", "100");
    const voice = app(51156, "Discord.exe", "C:\\Discord\\app-1.0.9259\\Discord.exe", "105", 1);
    const media = app(53964, "Discord.exe", "C:\\Discord\\app-1.0.9259\\Discord.exe", "104", 2);
    expect(applicationCaptureChoices([voice, media, root])).toEqual({ withAudio: [root], other: [] });
  });

  it("identifies a route whose only enabled source is one bound application", () => {
    const withApp = appendApplicationCaptureNode({ ...demoSession, nodes: demoSession.nodes.filter((node) => !["physicalInput", "testSignal", "audioFile"].includes(node.kind)) }, app(7, "Discord.exe", "C:\\Discord\\app-1\\Discord.exe", "1"));
    expect(applicationOnlyRouteSource(withApp)?.kind).toBe("applicationCapture");
    expect(applicationOnlyRouteSource(demoSession)).toBeNull();
    const withMic = { ...withApp, nodes: [...withApp.nodes, { ...withApp.nodes.at(-1)!, id: "mic", kind: "physicalInput" as const, parameters: {} }] };
    expect(applicationOnlyRouteSource(withMic)).toBeNull();
    const disabledMic = { ...withMic, nodes: withMic.nodes.map((node) => node.id === "mic" ? { ...node, enabled: false } : node) };
    expect(applicationOnlyRouteSource(disabledMic)?.kind).toBe("applicationCapture");
    const appId = withApp.nodes.at(-1)!.id;
    expect(mixedApplicationRouteOtherSources(withMic, appId).map((node) => node.id)).toEqual(["mic"]);
    expect(mixedApplicationRouteOtherSources(disabledMic, appId)).toEqual([]);
  });

  it("rebinds a node in place, keeping its id, connections, and custom settings", () => {
    const withNode = appendApplicationCaptureNode(demoSession, app(1284, "Discord.exe", "C:\\Discord\\app-1.0.9258\\Discord.exe", "100"));
    const nodeId = withNode.nodes.at(-1)!.id;
    const custom = setNodeDraftParameter(withNode, nodeId, "muted", true);
    const next = rebindApplicationCaptureNode(custom, nodeId, app(36808, "Discord.exe", "C:\\Discord\\app-1.0.9259\\Discord.exe", "200"));
    const node = next.nodes.find((candidate) => candidate.id === nodeId)!;
    expect(next.nodes).toHaveLength(custom.nodes.length);
    expect(next.edges).toEqual(custom.edges);
    expect(node.name).toBe("Discord.exe capture 1");
    expect(node.parameters).toEqual({
      muted: true,
      executable: "Discord.exe",
      executablePath: "C:\\Discord\\app-1.0.9259\\Discord.exe",
      processPolicy: "selectedInstance",
      processId: 36808,
      creationTime100ns: "200",
    });
  });

  it("renames only generated names and drops a stale instance identity", () => {
    const withNode = appendApplicationCaptureNode(demoSession, app(1, "game.exe", "C:\\game.exe", "1"));
    const nodeId = withNode.nodes.at(-1)!.id;
    const renamed = rebindApplicationCaptureNode(withNode, nodeId, { ...app(2, "chat.exe", "C:\\chat.exe", "2"), creationTime100ns: null });
    const node = renamed.nodes.find((candidate) => candidate.id === nodeId)!;
    expect(node.name).toBe("chat.exe capture 1");
    expect(node.parameters).toEqual({ executable: "chat.exe", executablePath: "C:\\chat.exe", processPolicy: "allVerifiedInstances" });
    const custom = setNodeDraftName(withNode, nodeId, "Voice chat");
    expect(rebindApplicationCaptureNode(custom, nodeId, app(3, "chat.exe", "C:\\chat.exe", "3")).nodes.find((candidate) => candidate.id === nodeId)!.name).toBe("Voice chat");
    expect(() => rebindApplicationCaptureNode(withNode, withNode.nodes[0].id, app(3, "chat.exe", "C:\\chat.exe", "3"))).toThrow(/application capture node/);
  });
});

describe("appendApplicationCaptureNode", () => {
  it("creates an enabled capture source bound to the observed process identity", () => {
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
      enabled: true,
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

describe("per-source volume", () => {
  const mixerSession = (): Session => {
    let next = appendLibraryNode(demoSession, "mixer");
    next = appendLibraryNode(next, "volume");
    const mixerId = next.nodes.find((node) => node.kind === "mixer")!.id;
    const volumeId = next.nodes.find((node) => node.kind === "volume")!.id;
    next = appendDraftConnection(next, "mic", "out", volumeId, "in");
    next = appendDraftConnection(next, volumeId, "out", mixerId, "in");
    return next;
  };

  it("adds a Volume tool at 100 % and validates its range", () => {
    const session = mixerSession();
    const volume = session.nodes.find((node) => node.kind === "volume")!;
    expect(volume.parameters).toEqual({ percent: 100 });
    expect(setNodeDraftParameter(session, volume.id, "percent", 110).nodes.find((node) => node.id === volume.id)!.parameters.percent).toBe(110);
    expect(() => setNodeDraftParameter(session, volume.id, "percent", 201)).toThrow(/0 and 200/);
  });

  it("lists Mixer inputs with a default of 100 % and stores per-input volume by upstream node", () => {
    const session = mixerSession();
    const mixer = session.nodes.find((node) => node.kind === "mixer")!;
    const [input] = mixerInputs(session, mixer.id);
    expect(input.upstream.kind).toBe("volume");
    expect(input.percent).toBe(100);
    const key = mixerInputVolumeKey(input.upstream.id);
    expect(key).toBe(`inputVolume:${input.upstream.id}`);
    const updated = setNodeDraftParameter(session, mixer.id, key, 50);
    expect(mixerInputs(updated, mixer.id)[0].percent).toBe(50);
    expect(() => setNodeDraftParameter(session, mixer.id, key, 101)).toThrow(/0 and 100/);
    expect(mixerInputs(session, "missing")).toEqual([]);
  });
});

describe("Mixer route sources", () => {
  const app = (processId: number): ApplicationInfo => ({ processId, executable: "Discord.exe", executablePath: "C:\\Discord\\app-1\\Discord.exe", creationTime100ns: "1", audioActivity: "active", captureCapability: "notObserved", audioSessionCount: 1, activeAudioSessionCount: 1, captureSessionCount: 0, renderSessionCount: 1, audioDisplayNames: [] });
  const route = (): Session => {
    let next = appendLibraryNode({ ...demoSession, nodes: demoSession.nodes.filter((node) => node.kind !== "gain"), edges: [] }, "mixer");
    next = appendLibraryNode(next, "testSignal");
    next = appendApplicationCaptureNode(next, app(7));
    next = appendLibraryNode(next, "volume");
    const id = (kind: string) => next.nodes.find((node) => node.kind === kind)!.id;
    next = appendDraftConnection(next, id("applicationCapture"), "out", id("volume"), "in");
    next = appendDraftConnection(next, id("volume"), "out", id("mixer"), "in");
    next = appendDraftConnection(next, "mic", "out", id("mixer"), "in");
    next = appendDraftConnection(next, id("testSignal"), "out", id("mixer"), "in");
    return next;
  };

  it("walks processor chains to the real sources in Mixer connection order", () => {
    expect(mixerRouteSources(route())?.map((node) => node.kind)).toEqual(["applicationCapture", "physicalInput", "testSignal"]);
  });

  it("prunes disabled sources that nothing feeds, like the engine", () => {
    const session = route();
    const withoutTone = { ...session, nodes: session.nodes.map((node) => node.kind === "testSignal" ? { ...node, enabled: false } : node) };
    expect(mixerRouteSources(withoutTone)?.map((node) => node.kind)).toEqual(["applicationCapture", "physicalInput"]);
    expect(pruneInactiveUpstream(withoutTone).edges.some((edge) => edge.sourceNode.startsWith("testSignal"))).toBe(false);
    expect(pruneInactiveUpstream(session)).toBe(session);
  });

  it("returns null without exactly one enabled Mixer", () => {
    expect(mixerRouteSources(demoSession)).toBeNull();
  });
});

describe("isParameterOnlyChange", () => {
  it("accepts slider-style edits and rejects topology, flag, and name changes", () => {
    const withVolume = appendLibraryNode(demoSession, "volume");
    const volumeId = withVolume.nodes.at(-1)!.id;
    const saved = appendDraftConnection(withVolume, "mic", "out", volumeId, "in");
    expect(isParameterOnlyChange(saved, saved)).toBe(false);
    expect(isParameterOnlyChange(saved, setNodeDraftParameter(saved, volumeId, "percent", 50))).toBe(true);
    expect(isParameterOnlyChange(saved, appendLibraryNode(saved, "gain"))).toBe(false);
    expect(isParameterOnlyChange(saved, setNodeDraftName(saved, volumeId, "Discord level"))).toBe(false);
    expect(isParameterOnlyChange(saved, { ...saved, nodes: saved.nodes.map((node) => node.id === volumeId ? { ...node, enabled: false } : node) })).toBe(false);
    expect(saved.edges.length).toBeGreaterThan(0);
    expect(isParameterOnlyChange(saved, { ...setNodeDraftParameter(saved, volumeId, "percent", 50), edges: saved.edges.slice(1) })).toBe(false);
  });
});

describe("Input Switch routes", () => {
  it("treats an Input Switch as the convergence node for Play", () => {
    let next = appendLibraryNode({ ...demoSession, nodes: demoSession.nodes.filter((node) => node.kind !== "gain"), edges: [] }, "inputSwitch");
    next = appendApplicationCaptureNode(next, { processId: 9, executable: "Spotify.exe", executablePath: "C:\\Spotify\\Spotify.exe", creationTime100ns: "3", audioActivity: "active", captureCapability: "notObserved", audioSessionCount: 1, activeAudioSessionCount: 1, captureSessionCount: 0, renderSessionCount: 1, audioDisplayNames: [] });
    const switchId = next.nodes.find((node) => node.kind === "inputSwitch")!.id;
    const appId = next.nodes.find((node) => node.kind === "applicationCapture")!.id;
    next = appendDraftConnection(next, appId, "out", switchId, "a");
    next = appendDraftConnection(next, "mic", "out", switchId, "b");
    expect(next.nodes.find((node) => node.id === switchId)!.parameters).toEqual({ selected: "a", fade: "normal" });
    expect(mixerRouteSources(next)?.map((node) => node.kind)).toEqual(["applicationCapture", "physicalInput"]);
    expect(() => appendDraftConnection(next, "mic", "out", switchId, "a")).toThrow(/already has a connection/);
  });
});

describe("pluginCatalog", () => {
  const entry = (path: string, sha256: string, format: "vst3" | "vst2", compatibility: "supportedVst3X64" | "supportedVst2X64Gated" | "unsupportedFormat") => ({
    path,
    identity: { path, binaryPath: path, format, architecture: "x64" as const, fileBytes: 1, sha256, vendor: "Cockos", version: "1", classIds: [], compatibility },
    error: null,
    errorCode: null,
  });
  it("lists supported plugins once each, sorted by name, with format and folder", () => {
    const catalog = pluginCatalog([
      { directory: "C:\\A", entries: [entry("C:\\A\\ReaEQ.dll", "aa", "vst2", "supportedVst2X64Gated"), entry("C:\\A\\old32.dll", "bb", "vst2", "unsupportedFormat")] },
      { directory: "C:\\B", entries: [entry("C:\\A\\ReaEQ.dll", "aa", "vst2", "supportedVst2X64Gated"), entry("C:\\B\\Compressor.vst3", "cc", "vst3", "supportedVst3X64")] },
    ]);
    expect(catalog.map((item) => [item.name, item.format, item.folder])).toEqual([["Compressor", "VST3", "C:\\B"], ["ReaEQ", "VST2", "C:\\A"]]);
    expect(STANDARD_PLUGIN_FOLDERS).toContain("C:\\Program Files\\Common Files\\VST3");
  });
});

describe("independent paths", () => {
  const node = (id: string, kind: Session["nodes"][number]["kind"], extra: Partial<Session["nodes"][number]> = {}): Session["nodes"][number] => ({
    id, kind, typeVersion: 1, name: id, enabled: true, bypass: false, parameters: {}, ports: [{ name: "in", direction: "input", channels: 2 }, { name: "out", direction: "output", channels: 2 }], ...extra,
  });
  const edge = (id: string, sourceNode: string, destinationNode: string): Session["edges"][number] => ({
    id, sourceNode, sourcePort: "out", destinationNode, destinationPort: "in", matrix: [1, 0, 0, 1], enabled: true,
  });
  const twoPaths: Session = {
    ...demoSession,
    nodes: [node("mic", "physicalInput"), node("voice", "gain"), node("cable-a", "physicalOutput"), node("cable-b", "physicalInput"), node("eq", "parametricEq"), node("scarlett", "physicalOutput"), node("spare", "gain")],
    edges: [edge("e1", "mic", "voice"), edge("e2", "voice", "cable-a"), edge("e3", "cable-b", "eq"), edge("e4", "eq", "scarlett")],
  };

  it("groups connected nodes into paths and leaves unconnected nodes out", () => {
    expect(independentPaths(twoPaths).map((path) => path.map((item) => item.id))).toEqual([["mic", "voice", "cable-a"], ["cable-b", "eq", "scarlett"]]);
    expect(needsNativePaths(twoPaths)).toBe(true);
  });

  it("uses the multi-path worker for one path with two outputs but not for a plain chain", () => {
    const single: Session = { ...twoPaths, nodes: twoPaths.nodes.slice(0, 3), edges: twoPaths.edges.slice(0, 2) };
    expect(needsNativePaths(single)).toBe(false);
    const monitored: Session = { ...single, nodes: [...single.nodes, node("monitor", "physicalOutput")], edges: [...single.edges, edge("e5", "voice", "monitor")] };
    expect(needsNativePaths(monitored)).toBe(true);
  });

  it("lists device nodes that still need a saved endpoint", () => {
    const bound: Session = { ...twoPaths, nodes: twoPaths.nodes.map((item) => item.id === "scarlett" ? item : { ...item, parameters: item.kind.startsWith("physical") ? { endpointId: `${item.id}-endpoint` } : item.parameters }) };
    expect(unboundDeviceNodes(bound).map((item) => item.id)).toEqual(["scarlett"]);
  });
});
