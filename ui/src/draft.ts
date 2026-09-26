import type { ApplicationInfo, EntityId, NodeKind, PluginScanEntry, Session } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";

export type DraftChange = {
  path: `/nodes/${number}/${"enabled" | "bypass"}` | `/nodes/${number}/parameters/${string}`;
  value: boolean | number | string;
};

/** Gain bounds mirrored from the authoritative DSP/domain contract. */
export const GAIN_MIN_DB = -60;
export const GAIN_MAX_DB = 24;

export type LibraryNodeKind = Extract<NodeKind, "physicalInput" | "physicalOutput" | "testSignal" | "audioFile" | "mixer" | "gain" | "volume" | "bassTreble" | "dehum" | "declick" | "inputSwitch" | "denoise" | "speechDenoise" | "firFilter" | "timeShift" | "mute" | "meter" | "parametricEq" | "compressor" | "gate" | "limiter" | "delay" | "graphicEq" | "pitch" | "recorder">;
export type InsertableProcessorKind = Exclude<LibraryNodeKind, "physicalInput" | "physicalOutput" | "testSignal" | "mixer" | "inputSwitch" | "meter">;
export type EqPresetId = "voiceNeutral" | "hum50Hz" | "hum60Hz";
export type VoiceChainPresetId = "voiceNeutral" | "voiceGateAndCompression";

const parametricEqDefaults: Record<string, boolean | number | string> = {
  frequencyHz: 1000,
  q: 1,
  gainDb: 0,
};
for (let index = 0; index < 16; index += 1) {
  parametricEqDefaults[`band${index}Enabled`] = false;
  parametricEqDefaults[`band${index}Type`] = "peaking";
  parametricEqDefaults[`band${index}FrequencyHz`] = 1000;
  parametricEqDefaults[`band${index}Q`] = 1;
  parametricEqDefaults[`band${index}GainDb`] = 0;
}

const libraryNodeDefinitions: Record<LibraryNodeKind, {
  name: string;
  parameters: Record<string, boolean | number | string>;
  ports: Session["nodes"][number]["ports"];
}> = {
  physicalInput: {
    name: "Physical input",
    parameters: {},
    ports: [{ name: "out", direction: "output", channels: 2 }],
  },
  physicalOutput: {
    name: "Physical output",
    parameters: {},
    ports: [{ name: "in", direction: "input", channels: 2 }],
  },
  testSignal: {
    name: "Test Signal",
    parameters: { frequencyHz: 440, levelDb: -18, durationMs: 1000 },
    ports: [{ name: "out", direction: "output", channels: 2 }],
  },
  audioFile: {
    name: "Audio file",
    parameters: { mediaId: "", loop: false },
    ports: [{ name: "out", direction: "output", channels: 2 }],
  },
  mixer: {
    name: "Mixer",
    parameters: {},
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  volume: {
    name: "Volume",
    parameters: { percent: 100 },
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  timeShift: {
    name: "Time Shift",
    parameters: { bufferSeconds: 60 },
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  firFilter: {
    name: "FIR Filter",
    parameters: { wetPercent: 100, gainDb: 0 },
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  denoise: {
    name: "Denoise",
    parameters: { reductionPercent: 70, floorPercent: 10, learning: false },
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  speechDenoise: {
    name: "Speech Denoise",
    parameters: { strengthPercent: 70 },
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  inputSwitch: {
    name: "Input Switch",
    parameters: { selected: "a", fade: "normal" },
    ports: [
      { name: "a", direction: "input", channels: 2 },
      { name: "b", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  bassTreble: {
    name: "Bass & Treble",
    parameters: { bassDb: 0, trebleDb: 0 },
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  dehum: {
    name: "Dehum",
    parameters: { frequencyHz: 60, amountPercent: 50, harmonics: 4 },
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  declick: {
    name: "Declick",
    parameters: { thresholdPercent: 50 },
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  gain: {
    name: "Gain",
    parameters: { gainDb: 0 },
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  mute: {
    name: "Mute",
    parameters: { muted: false },
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  meter: {
    name: "Meter",
    parameters: {},
    ports: [{ name: "in", direction: "input", channels: 2 }],
  },
  recorder: {
    name: "Recorder",
    parameters: {},
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  parametricEq: {
    name: "Advanced EQ",
    parameters: parametricEqDefaults,
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  compressor: {
    name: "Compressor",
    parameters: { thresholdDb: -18, ratio: 3, attackMs: 10, releaseMs: 150, kneeDb: 6, makeupDb: 0 },
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  gate: {
    name: "Gate",
    parameters: { thresholdDb: -45, rangeDb: 60, hysteresisDb: 3, ratio: 4, attackMs: 5, holdMs: 50, releaseMs: 150 },
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  limiter: {
    name: "Limiter",
    parameters: { ceilingDb: -1, lookaheadMs: 5, releaseMs: 100 },
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  delay: {
    name: "Sync",
    parameters: { delayMs: 0 },
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  graphicEq: {
    name: "Graphic EQ",
    parameters: { band0Db: 0, band1Db: 0, band2Db: 0, band3Db: 0, band4Db: 0, band5Db: 0, band6Db: 0, band7Db: 0, band8Db: 0, band9Db: 0 },
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  pitch: {
    name: "Pitch shift",
    parameters: { semitones: 0, cents: 0 },
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
};

/**
 * True when `draft` differs from `saved` only in node parameter values: same
 * nodes (ids, kinds, names, flags, ports) and identical connections. Such an
 * edit can be applied to running audio without re-preparing devices.
 */
export function isParameterOnlyChange(saved: Session, draft: Session): boolean {
  if (saved.id !== draft.id || saved.name !== draft.name || saved.nodes.length !== draft.nodes.length) return false;
  if (JSON.stringify(saved.edges) !== JSON.stringify(draft.edges)) return false;
  let parameterChanged = false;
  for (let index = 0; index < saved.nodes.length; index += 1) {
    const before = saved.nodes[index];
    const after = draft.nodes[index];
    if (before.id !== after.id || before.kind !== after.kind || before.name !== after.name || before.enabled !== after.enabled || before.bypass !== after.bypass || JSON.stringify(before.ports) !== JSON.stringify(after.ports)) return false;
    if (JSON.stringify(before.parameters) !== JSON.stringify(after.parameters)) parameterChanged = true;
  }
  return parameterChanged;
}

/** Mixer parameter prefix for per-input volume, keyed by the upstream node id (GRAPH-04). */
export const MIXER_INPUT_VOLUME_PREFIX = "inputVolume:";

export function mixerInputVolumeKey(upstreamNodeId: EntityId): string {
  return `${MIXER_INPUT_VOLUME_PREFIX}${upstreamNodeId}`;
}

/** Connected Mixer inputs in connection order, with each input's volume percent (default 100). */
export function mixerInputs(session: Session, mixerId: EntityId): Array<{ edgeId: EntityId; upstream: Session["nodes"][number]; percent: number; enabled: boolean }> {
  const mixer = session.nodes.find((node) => node.id === mixerId);
  if (!mixer || mixer.kind !== "mixer") return [];
  return session.edges
    .filter((edge) => edge.destinationNode === mixerId)
    .flatMap((edge) => {
      const upstream = session.nodes.find((node) => node.id === edge.sourceNode);
      if (!upstream) return [];
      const raw = mixer.parameters[mixerInputVolumeKey(upstream.id)];
      const percent = typeof raw === "number" && Number.isFinite(raw) ? Math.min(100, Math.max(0, raw)) : 100;
      return [{ edgeId: edge.id, upstream, percent, enabled: edge.enabled && upstream.enabled }];
    });
}

/** Adds one supported built-in processor to a draft without mutating its revision or edges. */
export function appendLibraryNode(
  session: Session,
  kind: LibraryNodeKind,
): Session {
  const definition = libraryNodeDefinitions[kind];
  let suffix = 1;
  let id = `${kind}-${suffix}`;
  while (session.nodes.some((node) => node.id === id)) {
    suffix += 1;
    id = `${kind}-${suffix}`;
  }
  return {
    ...session,
    nodes: [
      ...session.nodes,
      {
        id,
        kind,
        typeVersion: 1,
        name: `${definition.name} ${suffix}`,
        enabled: true,
        bypass: false,
        parameters: { ...definition.parameters },
        ports: definition.ports.map((port) => ({ ...port })),
      },
    ],
  };
}

function applicationIdentityParameters(application: ApplicationInfo): Record<string, boolean | number | string> {
  if (!application.executable || application.processId < 1) {
    throw new Error("Only a verified running application can be added to the graph");
  }
  const selectedInstance = application.creationTime100ns !== null;
  const parameters: Record<string, boolean | number | string> = {
    executable: application.executable,
    processPolicy: selectedInstance ? "selectedInstance" : "allVerifiedInstances",
  };
  if (selectedInstance) {
    parameters.processId = application.processId;
    parameters.creationTime100ns = application.creationTime100ns!;
  }
  if (application.executablePath !== null) parameters.executablePath = application.executablePath;
  return parameters;
}

/** Stable picker key for one exact process instance. */
export function applicationChoiceKey(application: ApplicationInfo): string {
  return `${application.processId}-${application.creationTime100ns ?? "unknown"}`;
}

/**
 * Groups application-capture choices. Process loopback records what an
 * application plays, so a process without a current audio session is still a
 * valid source. Processes with audio sessions are listed individually; others
 * fold same-path helpers (Electron/Chromium trees) into their earliest instance.
 */
export function applicationCaptureChoices(applications: readonly ApplicationInfo[]): { withAudio: ApplicationInfo[]; other: ApplicationInfo[] } {
  const eligible = applications.filter((application) => application.executable && application.processId > 0);
  const byLabel = (left: ApplicationInfo, right: ApplicationInfo) => left.executable.localeCompare(right.executable, undefined, { sensitivity: "base" }) || left.processId - right.processId;
  const pathKey = (application: ApplicationInfo) => (application.executablePath ?? application.executable).toLowerCase();
  const audioPaths = new Set(eligible.filter((application) => application.audioSessionCount > 0).map(pathKey));
  // Capture includes the selected process tree, so one entry per executable
  // path is enough: the earliest-created instance is normally the tree root.
  const earliest = new Map<string, ApplicationInfo>();
  for (const application of eligible) {
    const key = pathKey(application);
    const current = earliest.get(key);
    const created = application.creationTime100ns === null ? null : BigInt(application.creationTime100ns);
    const currentCreated = current?.creationTime100ns == null ? null : BigInt(current.creationTime100ns);
    if (!current || (created !== null && (currentCreated === null || created < currentCreated))) earliest.set(key, application);
  }
  const roots = [...earliest.values()].sort(byLabel);
  return { withAudio: roots.filter((application) => audioPaths.has(pathKey(application))), other: roots.filter((application) => !audioPaths.has(pathKey(application))) };
}

/**
 * The single application-capture source of a route whose only enabled source
 * is that application, or null. Such a route runs on the application worker
 * (process loopback to one output) instead of the physical endpoint pair.
 */
const ROUTE_SOURCE_KINDS = new Set(["physicalInput", "applicationCapture", "endpointLoopback", "virtualRenderSource", "testSignal", "audioFile"]);

/** Processor kinds the engine accepts in a linear chain before a Mixer input (mirrors `is_chain_processor`). */
const CHAIN_PROCESSOR_KINDS = new Set<NodeKind>(["gain", "volume", "bassTreble", "dehum", "declick", "denoise", "speechDenoise", "firFilter", "timeShift", "mute", "meter", "parametricEq", "compressor", "gate", "limiter", "delay", "graphicEq", "pitch", "plugin"]);

/** Drop disabled nodes that nothing live feeds, with their edges (mirrors `prune_inactive_upstream`). */
export function pruneInactiveUpstream(session: Session): Session {
  const removed = new Set<string>();
  for (let changed = true; changed;) {
    changed = false;
    for (const node of session.nodes) {
      if (node.enabled || removed.has(node.id)) continue;
      const fed = session.edges.some((edge) => edge.enabled && edge.destinationNode === node.id && !removed.has(edge.sourceNode));
      if (!fed) { removed.add(node.id); changed = true; }
    }
  }
  if (removed.size === 0 || removed.size === session.nodes.length) return session;
  return { ...session, nodes: session.nodes.filter((node) => !removed.has(node.id)), edges: session.edges.filter((edge) => !removed.has(edge.sourceNode) && !removed.has(edge.destinationNode)) };
}

/**
 * The real sources of a single-Mixer route, in the engine's input order
 * (enabled Mixer connections in session order, each walked upstream through
 * processor chains), after disabled sources are pruned. Null when the route
 * has no single enabled Mixer or an input is fed ambiguously.
 */
export function mixerRouteSources(session: Session): Session["nodes"] | null {
  const pruned = pruneInactiveUpstream(session);
  const mixers = pruned.nodes.filter((node) => (node.kind === "mixer" || node.kind === "inputSwitch") && node.enabled && !node.bypass);
  if (mixers.length !== 1) return null;
  const byId = new Map(pruned.nodes.map((node) => [node.id, node]));
  const sources: Session["nodes"] = [];
  for (const edge of pruned.edges.filter((candidate) => candidate.enabled && candidate.destinationNode === mixers[0].id)) {
    let current = byId.get(edge.sourceNode);
    for (let depth = 0; current && CHAIN_PROCESSOR_KINDS.has(current.kind); depth += 1) {
      const node: Session["nodes"][number] = current;
      const feeds = pruned.edges.filter((candidate) => candidate.enabled && candidate.destinationNode === node.id);
      if (feeds.length === 0) break;
      if (feeds.length > 1 || depth >= 16) return null;
      current = byId.get(feeds[0].sourceNode);
    }
    if (!current) return null;
    sources.push(current);
  }
  return sources;
}

/**
 * The independent paths of a route (GRAPH-15): groups of nodes joined by
 * enabled connections, after disabled sources are pruned, in the order their
 * first node appears. Unconnected nodes belong to no path.
 */
export function independentPaths(session: Session): Session["nodes"][] {
  const pruned = pruneInactiveUpstream(session);
  const group = new Map<string, number>();
  const groups: Set<string>[] = [];
  for (const edge of pruned.edges.filter((candidate) => candidate.enabled)) {
    const a = group.get(edge.sourceNode);
    const b = group.get(edge.destinationNode);
    if (a === undefined && b === undefined) {
      groups.push(new Set([edge.sourceNode, edge.destinationNode]));
      group.set(edge.sourceNode, groups.length - 1);
      group.set(edge.destinationNode, groups.length - 1);
    } else if (a === undefined || b === undefined || a === b) {
      const index = (a ?? b) as number;
      groups[index].add(edge.sourceNode).add(edge.destinationNode);
      group.set(edge.sourceNode, index);
      group.set(edge.destinationNode, index);
    } else {
      for (const id of groups[b]) { groups[a].add(id); group.set(id, a); }
      groups[b].clear();
    }
  }
  return groups
    .filter((members) => members.size > 0)
    .map((members) => pruned.nodes.filter((node) => members.has(node.id)))
    .sort((left, right) => pruned.nodes.indexOf(left[0]) - pruned.nodes.indexOf(right[0]));
}

/**
 * Whether Play runs this route on the multi-path worker: it has several
 * independent paths, or one path reads or writes several devices. The backend
 * resolves every device from the node's own saved endpoint.
 */
export function needsNativePaths(session: Session): boolean {
  const paths = independentPaths(session);
  if (paths.length > 1) return true;
  return paths.some((nodes) =>
    nodes.filter((node) => node.enabled && node.kind === "physicalOutput").length > 1
    || nodes.filter((node) => node.enabled && node.kind === "physicalInput").length > 1);
}

/** Enabled device nodes of a route that have no saved endpoint yet. */
export function unboundDeviceNodes(session: Session): Session["nodes"] {
  return independentPaths(session)
    .flat()
    .filter((node) => node.enabled && (node.kind === "physicalInput" || node.kind === "physicalOutput") && typeof node.parameters.endpointId !== "string");
}

/** Enabled sources other than the given application node that keep an application route from running on Play. */
export function mixedApplicationRouteOtherSources(session: Session, applicationNodeId: string): Session["nodes"] {
  return session.nodes.filter((node) => node.enabled && node.id !== applicationNodeId && ROUTE_SOURCE_KINDS.has(node.kind));
}

export function applicationOnlyRouteSource(session: Session): Session["nodes"][number] | null {
  const sources = session.nodes.filter((node) => node.enabled && ROUTE_SOURCE_KINDS.has(node.kind));
  const [source] = sources;
  if (sources.length !== 1 || source.kind !== "applicationCapture") return null;
  if (source.parameters.processPolicy !== "selectedInstance" || typeof source.parameters.processId !== "number" || typeof source.parameters.creationTime100ns !== "string" || typeof source.parameters.executable !== "string") return null;
  return source;
}

/** Rebinds an application-capture node to another running process, keeping the node and its connections. */
export function rebindApplicationCaptureNode(session: Session, nodeId: EntityId, application: ApplicationInfo): Session {
  const nodeIndex = session.nodes.findIndex((node) => node.id === nodeId);
  if (nodeIndex < 0) throw new Error(`Unknown node: ${nodeId}`);
  const node = session.nodes[nodeIndex];
  if (node.kind !== "applicationCapture") throw new Error("Only an application capture node can select an application");
  const identity = applicationIdentityParameters(application);
  const rest = Object.fromEntries(Object.entries(node.parameters).filter(([name]) => !["executable", "executablePath", "processPolicy", "processId", "creationTime100ns"].includes(name)));
  const previousExecutable = typeof node.parameters.executable === "string" ? node.parameters.executable : null;
  const generatedName = previousExecutable !== null && node.name.startsWith(`${previousExecutable} capture `) && /^\d+$/.test(node.name.slice(previousExecutable.length + " capture ".length));
  const name = generatedName ? `${application.executable}${node.name.slice(previousExecutable!.length)}` : node.name;
  const nodes = [...session.nodes];
  nodes[nodeIndex] = { ...node, name, parameters: { ...rest, ...identity } };
  return { ...session, nodes };
}

/** Adds a verified application identity as a process-capture source. */
export function appendApplicationCaptureNode(session: Session, application: ApplicationInfo): Session {
  const parameters = applicationIdentityParameters(application);
  let suffix = 1;
  let id = `application-capture-${suffix}`;
  while (session.nodes.some((node) => node.id === id)) {
    suffix += 1;
    id = `application-capture-${suffix}`;
  }
  return {
    ...session,
    nodes: [...session.nodes, {
      id,
      kind: "applicationCapture",
      typeVersion: 1,
      name: `${application.executable} capture ${suffix}`,
      enabled: true,
      bypass: false,
      parameters,
      ports: [{ name: "out", direction: "output", channels: 2 }],
    }],
  };
}

/** Adds a stopped endpoint-loopback source bound to one exact render endpoint. */
export function appendEndpointLoopbackNode(session: Session, endpointId: string): Session {
  const normalizedEndpointId = endpointId.trim();
  if (!normalizedEndpointId) throw new Error("An exact render endpoint is required");
  let suffix = 1;
  let id = `endpoint-loopback-${suffix}`;
  while (session.nodes.some((node) => node.id === id)) {
    suffix += 1;
    id = `endpoint-loopback-${suffix}`;
  }
  return {
    ...session,
    nodes: [...session.nodes, {
      id,
      kind: "endpointLoopback",
      typeVersion: 1,
      name: `Endpoint loopback ${suffix}`,
      enabled: false,
      bypass: false,
      parameters: { endpointId: normalizedEndpointId },
      ports: [{ name: "out", direction: "output", channels: 2 }],
    }],
  };
}

/** Adds a stopped virtual-bus source or sink bound to an existing bus identity. */
export function appendVirtualBusNode(session: Session, busId: string, direction: "renderSource" | "captureSink"): Session {
  const normalizedBusId = busId.trim();
  if (!normalizedBusId) throw new Error("An existing virtual bus identity is required");
  const kind = direction === "renderSource" ? "virtualRenderSource" : "virtualCaptureSink";
  const prefix = direction === "renderSource" ? "virtual-render-source" : "virtual-capture-sink";
  let suffix = 1;
  let id = `${prefix}-${suffix}`;
  while (session.nodes.some((node) => node.id === id)) {
    suffix += 1;
    id = `${prefix}-${suffix}`;
  }
  return {
    ...session,
    nodes: [...session.nodes, {
      id,
      kind,
      typeVersion: 1,
      name: `${direction === "renderSource" ? "Virtual render source" : "Virtual capture sink"} ${suffix}`,
      enabled: false,
      bypass: false,
      parameters: { busId: normalizedBusId },
      ports: direction === "renderSource"
        ? [{ name: "out", direction: "output", channels: 2 }]
        : [{ name: "in", direction: "input", channels: 2 }],
    }],
  };
}

/**
 * Adds a verified scan result as a plugin node. It stores the path exactly as
 * scanned (the backend matches remembered scans by it and re-verifies the
 * binary hash), is named after the plugin file, and is enabled: plugin code
 * still runs only in an isolated worker once the route plays.
 */
export function appendPluginPlaceholderNode(session: Session, entry: PluginScanEntry): Session {
  const identity = entry.identity;
  if (!identity || !["supportedVst2X64Gated", "supportedVst3X64"].includes(identity.compatibility)) {
    throw new Error("Only a supported x64 scan result can be added to the graph");
  }
  let suffix = 1;
  let id = `plugin-${suffix}`;
  while (session.nodes.some((node) => node.id === id)) {
    suffix += 1;
    id = `plugin-${suffix}`;
  }
  const fileName = (entry.path.split(/[\\/]/).pop() ?? "").replace(/\.(vst3|dll)$/i, "");
  return {
    ...session,
    nodes: [...session.nodes, {
      id,
      kind: "plugin",
      typeVersion: 1,
      name: fileName ? `${fileName} ${suffix}` : `${identity.vendor ?? "Plugin"} ${suffix}`,
      enabled: true,
      bypass: false,
      parameters: {
        path: entry.path,
        format: identity.format,
        fingerprint: identity.sha256,
        classId: identity.classIds[0] ?? "default",
      },
      ports: [
        { name: "in", direction: "input", channels: 1 },
        { name: "out", direction: "output", channels: 1 },
      ],
    }],
  };
}

/** Expands one authoritative EQ preset into an ordinary, inspectable draft node. */
export function appendEqPresetNode(session: Session, presetId: EqPresetId): Session {
  const next = appendLibraryNode(session, "parametricEq");
  const node = next.nodes.at(-1);
  if (!node) throw new Error("Unable to create an EQ preset node");
  const frequencyHz = presetId === "hum50Hz" ? 50 : presetId === "hum60Hz" ? 60 : 1000;
  return {
    ...next,
    nodes: next.nodes.map((candidate) => candidate.id === node.id
      ? {
        ...candidate,
        parameters: {
          ...candidate.parameters,
          band0Enabled: presetId !== "voiceNeutral",
          band0Type: presetId === "voiceNeutral" ? "peaking" : "notch",
          band0FrequencyHz: frequencyHz,
          band0Q: presetId === "voiceNeutral" ? 1 : 8,
          band0GainDb: 0,
        },
      }
      : candidate),
  };
}

/** Expands a voice preset without inventing a route when the draft is disconnected. */
export function appendVoiceChainPreset(session: Session, presetId: VoiceChainPresetId): Session {
  const kinds: InsertableProcessorKind[] = presetId === "voiceGateAndCompression"
    ? ["gate", "compressor", "limiter"]
    : ["limiter"];
  let next = session;
  let edgeId = next.edges.find((edge) => {
    const destination = next.nodes.find((node) => node.id === edge.destinationNode);
    return destination?.kind !== "mixer";
  })?.id;
  const destination = edgeId ? next.edges.find((edge) => edge.id === edgeId)?.destinationNode : undefined;
  const destinationPort = edgeId ? next.edges.find((edge) => edge.id === edgeId)?.destinationPort : undefined;
  for (const kind of kinds) {
    if (!edgeId || !destination || !destinationPort) {
      next = appendLibraryNode(next, kind);
      continue;
    }
    next = insertDraftProcessor(next, edgeId, kind);
    const processor = next.nodes.at(-1);
    edgeId = processor
      ? next.edges.find((edge) => edge.sourceNode === processor.id && edge.destinationNode === destination && edge.destinationPort === destinationPort)?.id
      : undefined;
  }
  return next;
}

/**
 * Builds a default channel matrix for connecting a source port to a
 * destination port. Fanning a narrower source out to a wider destination
 * (e.g. mono -> stereo) duplicates the source into every destination
 * channel. Fanning a wider source into a narrower destination (e.g.
 * stereo -> mono) downmixes by averaging every source channel instead of
 * silently keeping only the first one and dropping the rest.
 */
export function defaultChannelMatrix(sourceChannels: number, destinationChannels: number): number[] {
  const matrix = Array.from({ length: destinationChannels * sourceChannels }, () => 0);
  if (sourceChannels <= destinationChannels) {
    for (let destinationChannel = 0; destinationChannel < destinationChannels; destinationChannel += 1) {
      const sourceChannel = Math.min(destinationChannel, sourceChannels - 1);
      matrix[destinationChannel * sourceChannels + sourceChannel] = 1;
    }
  } else {
    const gain = 1 / sourceChannels;
    for (let destinationChannel = 0; destinationChannel < destinationChannels; destinationChannel += 1) {
      for (let sourceChannel = 0; sourceChannel < sourceChannels; sourceChannel += 1) {
        matrix[destinationChannel * sourceChannels + sourceChannel] = gain;
      }
    }
  }
  return matrix;
}

/** Adds a topology edge to a local draft; backend validation still gates commit. */
export function appendDraftConnection(
  session: Session,
  sourceNodeId: EntityId,
  sourcePortName: string,
  destinationNodeId: EntityId,
  destinationPortName: string,
): Session {
  if (sourceNodeId === destinationNodeId) throw new Error("A node cannot connect to itself");
  const sourceNode = session.nodes.find((node) => node.id === sourceNodeId);
  const destinationNode = session.nodes.find((node) => node.id === destinationNodeId);
  if (!sourceNode || !destinationNode) throw new Error("Both connection nodes are required");
  const sourcePort = sourceNode.ports.find((port) => port.name === sourcePortName);
  const destinationPort = destinationNode.ports.find((port) => port.name === destinationPortName);
  if (!sourcePort || sourcePort.direction !== "output") throw new Error("Choose an output source port");
  if (!destinationPort || destinationPort.direction !== "input") throw new Error("Choose an input destination port");
  if (session.edges.some((edge) => edge.sourceNode === sourceNodeId && edge.sourcePort === sourcePortName && edge.destinationNode === destinationNodeId && edge.destinationPort === destinationPortName)) {
    throw new Error("That connection is already in the draft");
  }
  if (destinationNode.kind !== "mixer" && session.edges.some((edge) => edge.destinationNode === destinationNodeId && edge.destinationPort === destinationPortName)) {
    throw new Error("That input already has a connection");
  }
  const matrix = defaultChannelMatrix(sourcePort.channels, destinationPort.channels);
  let suffix = 1;
  let id = `edge-${suffix}`;
  while (session.edges.some((edge) => edge.id === id)) {
    suffix += 1;
    id = `edge-${suffix}`;
  }
  return {
    ...session,
    edges: [...session.edges, {
      id,
      sourceNode: sourceNodeId,
      sourcePort: sourcePortName,
      destinationNode: destinationNodeId,
      destinationPort: destinationPortName,
      matrix,
      enabled: true,
    }],
  };
}

/** Inserts a deterministic stereo mixer into one draft edge without committing topology. */
export function insertDraftMixer(session: Session, edgeId: EntityId): Session {
  const edge = session.edges.find((candidate) => candidate.id === edgeId);
  if (!edge) throw new Error(`Unknown draft connection: ${edgeId}`);
  const sourceNode = session.nodes.find((node) => node.id === edge.sourceNode);
  const sourcePort = sourceNode?.ports.find((port) => port.name === edge.sourcePort);
  if (!sourcePort || sourcePort.direction !== "output") throw new Error("Inserted mixer requires an output source port");
  const withoutEdge = removeDraftConnection(session, edgeId);
  const withMixer = appendLibraryNode(withoutEdge, "mixer");
  const mixer = withMixer.nodes.at(-1);
  if (!mixer) throw new Error("Unable to create an inserted mixer");
  const widthMatched = { ...withMixer, nodes: withMixer.nodes.map((node) => node.id === mixer.id ? { ...node, ports: node.ports.map((port) => ({ ...port, channels: sourcePort.channels })) } : node) };
  const upstream = appendDraftConnection(widthMatched, edge.sourceNode, edge.sourcePort, mixer.id, "in");
  const split = appendDraftConnection(upstream, mixer.id, "out", edge.destinationNode, edge.destinationPort);
  return { ...split, edges: split.edges.map((candidate) => candidate.sourceNode === mixer.id && candidate.destinationNode === edge.destinationNode ? { ...candidate, matrix: [...edge.matrix] } : candidate) };
}

/** Turns a second direct source-to-output connection into a visible mixer. */
export function addSourceToOccupiedOutput(
  session: Session,
  occupiedEdgeId: EntityId,
  sourceNodeId: EntityId,
  sourcePortName: string,
): Session {
  const occupied = session.edges.find((edge) => edge.id === occupiedEdgeId);
  if (!occupied) throw new Error("The output connection changed; try again");
  const mixerSession = insertDraftMixer(session, occupiedEdgeId);
  const mixer = mixerSession.nodes.at(-1);
  if (!mixer || mixer.kind !== "mixer") throw new Error("Unable to add a mixer to the output");
  return appendDraftConnection(mixerSession, sourceNodeId, sourcePortName, mixer.id, "in");
}

/** Inserts a built-in processor directly into one draft connection. */
export function insertDraftProcessor(
  session: Session,
  edgeId: EntityId,
  kind: InsertableProcessorKind,
): Session {
  const edge = session.edges.find((candidate) => candidate.id === edgeId);
  if (!edge) throw new Error(`Unknown draft connection: ${edgeId}`);
  const sourceNode = session.nodes.find((node) => node.id === edge.sourceNode);
  const destinationNode = session.nodes.find((node) => node.id === edge.destinationNode);
  const sourcePort = sourceNode?.ports.find((port) => port.name === edge.sourcePort);
  const destinationPort = destinationNode?.ports.find((port) => port.name === edge.destinationPort);
  if (!sourcePort || sourcePort.direction !== "output" || !destinationPort || destinationPort.direction !== "input") {
    throw new Error("Inserted processor requires valid source and destination ports");
  }
  const withoutEdge = removeDraftConnection(session, edgeId);
  const withProcessor = appendLibraryNode(withoutEdge, kind);
  const processor = withProcessor.nodes.at(-1);
  if (!processor) throw new Error("Unable to create an inserted processor");
  const resized = {
    ...withProcessor,
    nodes: withProcessor.nodes.map((node) => node.id === processor.id
      ? { ...node, ports: node.ports.map((port) => ({ ...port, channels: sourcePort.channels })) }
      : node),
  };
  const upstream = appendDraftConnection(resized, edge.sourceNode, edge.sourcePort, processor.id, "in");
  const downstream = appendDraftConnection(upstream, processor.id, "out", edge.destinationNode, edge.destinationPort);
  return {
    ...downstream,
    edges: downstream.edges.map((candidate) => candidate.sourceNode === processor.id && candidate.destinationNode === edge.destinationNode
      ? { ...candidate, matrix: [...edge.matrix] }
      : candidate),
  };
}

/** Inserts a scanned VST2/VST3 plugin directly into one draft connection,
 * the same edge-splice shape as `insertDraftProcessor` uses for built-in
 * processors. The plugin is added disabled, matching `appendPluginPlaceholderNode`'s
 * fail-closed default; the dry signal passes through unaffected until it is
 * bound to an isolated worker and explicitly enabled. */
export function insertDraftPluginProcessor(
  session: Session,
  edgeId: EntityId,
  entry: PluginScanEntry,
): Session {
  const edge = session.edges.find((candidate) => candidate.id === edgeId);
  if (!edge) throw new Error(`Unknown draft connection: ${edgeId}`);
  const sourceNode = session.nodes.find((node) => node.id === edge.sourceNode);
  const destinationNode = session.nodes.find((node) => node.id === edge.destinationNode);
  const sourcePort = sourceNode?.ports.find((port) => port.name === edge.sourcePort);
  const destinationPort = destinationNode?.ports.find((port) => port.name === edge.destinationPort);
  if (!sourcePort || sourcePort.direction !== "output" || !destinationPort || destinationPort.direction !== "input") {
    throw new Error("Inserted plugin requires valid source and destination ports");
  }
  const withoutEdge = removeDraftConnection(session, edgeId);
  const withPlugin = appendPluginPlaceholderNode(withoutEdge, entry);
  const plugin = withPlugin.nodes.at(-1);
  if (!plugin) throw new Error("Unable to create an inserted plugin");
  const resized = {
    ...withPlugin,
    nodes: withPlugin.nodes.map((node) => node.id === plugin.id
      ? { ...node, ports: node.ports.map((port) => ({ ...port, channels: sourcePort.channels })) }
      : node),
  };
  const upstream = appendDraftConnection(resized, edge.sourceNode, edge.sourcePort, plugin.id, "in");
  const downstream = appendDraftConnection(upstream, plugin.id, "out", edge.destinationNode, edge.destinationPort);
  return {
    ...downstream,
    edges: downstream.edges.map((candidate) => candidate.sourceNode === plugin.id && candidate.destinationNode === edge.destinationNode
      ? { ...candidate, matrix: [...edge.matrix] }
      : candidate),
  };
}

/** Removes a mixer only when it has exactly one incoming and one outgoing path. */
export function removeSinglePathDraftMixer(session: Session, mixerId: EntityId): Session {
  const mixer = session.nodes.find((node) => node.id === mixerId);
  if (!mixer || mixer.kind !== "mixer") throw new Error("Choose a mixer node");
  const incoming = session.edges.filter((edge) => edge.destinationNode === mixerId);
  const outgoing = session.edges.filter((edge) => edge.sourceNode === mixerId);
  if (incoming.length !== 1 || outgoing.length !== 1) throw new Error("Mixer removal requires exactly one incoming and one outgoing connection");
  const sourceNode = session.nodes.find((node) => node.id === incoming[0].sourceNode);
  const mixerInput = mixer.ports.find((port) => port.name === incoming[0].destinationPort);
  const mixerOutput = mixer.ports.find((port) => port.name === outgoing[0].sourcePort);
  const destinationNode = session.nodes.find((node) => node.id === outgoing[0].destinationNode);
  const sourcePort = sourceNode?.ports.find((port) => port.name === incoming[0].sourcePort);
  const destinationPort = destinationNode?.ports.find((port) => port.name === outgoing[0].destinationPort);
  if (!sourcePort || !mixerInput || !mixerOutput || !destinationPort) throw new Error("Mixer connections reference unknown ports");
  if (incoming[0].matrix.length !== mixerInput.channels * sourcePort.channels || outgoing[0].matrix.length !== destinationPort.channels * mixerOutput.channels) {
    throw new Error("Mixer removal requires valid channel matrices");
  }
  const composedMatrix = Array.from({ length: destinationPort.channels * sourcePort.channels }, (_, index) => {
    const destinationChannel = Math.floor(index / sourcePort.channels);
    const sourceChannel = index % sourcePort.channels;
    let value = 0;
    for (let mixerChannel = 0; mixerChannel < mixerInput.channels; mixerChannel += 1) {
      value += (outgoing[0].matrix[destinationChannel * mixerOutput.channels + mixerChannel] ?? 0) * (incoming[0].matrix[mixerChannel * sourcePort.channels + sourceChannel] ?? 0);
    }
    return value;
  });
  const reduced = removeDraftNode(session, mixerId);
  const reconnected = appendDraftConnection(reduced, incoming[0].sourceNode, incoming[0].sourcePort, outgoing[0].destinationNode, outgoing[0].destinationPort);
  return { ...reconnected, edges: reconnected.edges.map((edge) => edge.sourceNode === incoming[0].sourceNode && edge.destinationNode === outgoing[0].destinationNode ? { ...edge, matrix: composedMatrix } : edge) };
}

/** Removes one draft edge while leaving the authoritative graph untouched. */
export function removeDraftConnection(session: Session, edgeId: EntityId): Session {
  if (!session.edges.some((edge) => edge.id === edgeId)) throw new Error(`Unknown draft connection: ${edgeId}`);
  return { ...session, edges: session.edges.filter((edge) => edge.id !== edgeId) };
}

/** Removes a draft node and its incident edges without changing the session revision. */
export function removeDraftNode(session: Session, nodeId: EntityId): Session {
  if (!session.nodes.some((node) => node.id === nodeId)) throw new Error(`Unknown draft node: ${nodeId}`);
  if (session.nodes.length <= 1) throw new Error("A draft must retain at least one node");
  return {
    ...session,
    nodes: session.nodes.filter((node) => node.id !== nodeId),
    edges: session.edges.filter((edge) => edge.sourceNode !== nodeId && edge.destinationNode !== nodeId),
  };
}

/** Duplicates one node as an unconnected draft node with a deterministic ID. */
export function duplicateDraftNode(session: Session, nodeId: EntityId): Session {
  const original = session.nodes.find((node) => node.id === nodeId);
  if (!original) throw new Error(`Unknown draft node: ${nodeId}`);
  let suffix = 1;
  let id = `${nodeId}-copy-${suffix}`;
  while (session.nodes.some((node) => node.id === id)) {
    suffix += 1;
    id = `${nodeId}-copy-${suffix}`;
  }
  return {
    ...session,
    nodes: [...session.nodes, {
      ...original,
      id,
      name: `${original.name} copy ${suffix}`,
      parameters: { ...original.parameters },
      ports: original.ports.map((port) => ({ ...port })),
    }],
  };
}

/** Changes only a draft edge's enabled state; topology and revision are preserved. */
export function setDraftConnectionEnabled(session: Session, edgeId: EntityId, enabled: boolean): Session {
  if (!session.edges.some((edge) => edge.id === edgeId)) throw new Error(`Unknown draft connection: ${edgeId}`);
  return { ...session, edges: session.edges.map((edge) => edge.id === edgeId ? { ...edge, enabled } : edge) };
}

/**
 * Creates a UI candidate without changing the authoritative session revision.
 * Validation and commit remain backend responsibilities.
 */
export function setNodeDraftFlag(
  session: Session,
  nodeId: EntityId,
  flag: "enabled" | "bypass",
  value: boolean,
): Session {
  const nodeIndex = session.nodes.findIndex((node) => node.id === nodeId);
  if (nodeIndex < 0) throw new Error(`Unknown node: ${nodeId}`);
  return {
    ...session,
    nodes: session.nodes.map((node, index) => index === nodeIndex ? { ...node, [flag]: value } : node),
  };
}

/** Renames a node in the local candidate while preserving its identity and topology. */
export function setNodeDraftName(session: Session, nodeId: EntityId, name: string): Session {
  const trimmed = name.trim();
  if (trimmed.length === 0) throw new Error("Node name cannot be empty");
  if (trimmed.length > 120) throw new Error("Node name cannot exceed 120 characters");
  if (!session.nodes.some((node) => node.id === nodeId)) throw new Error(`Unknown node: ${nodeId}`);
  return { ...session, nodes: session.nodes.map((node) => node.id === nodeId ? { ...node, name: trimmed } : node) };
}

/** Normalizes and validates a session name in the local candidate. */
export function setSessionDraftName(session: Session, name: string): Session {
  const trimmed = name.trim();
  if (trimmed.length === 0) throw new Error("Session name cannot be empty");
  if (trimmed.length > 120) throw new Error("Session name cannot exceed 120 characters");
  return { ...session, name: trimmed };
}

export function setNodeDraftParameter(
  session: Session,
  nodeId: EntityId,
  parameter: string,
  value: boolean | number | string,
): Session {
  const nodeIndex = session.nodes.findIndex((node) => node.id === nodeId);
  if (nodeIndex < 0) throw new Error(`Unknown node: ${nodeId}`);
  const node = session.nodes[nodeIndex];
  if (node.kind === "volume" && parameter === "percent" && (typeof value !== "number" || !Number.isFinite(value) || value < 0 || value > 200)) {
    throw new Error("Volume must be between 0 and 200 %");
  }
  if (node.kind === "mixer" && parameter.startsWith(MIXER_INPUT_VOLUME_PREFIX) && (typeof value !== "number" || !Number.isFinite(value) || value < 0 || value > 100)) {
    throw new Error("Mixer input volume must be between 0 and 100 %");
  }
  if (node.kind === "gain" && parameter === "gainDb" && (typeof value !== "number" || !Number.isFinite(value) || value < GAIN_MIN_DB || value > GAIN_MAX_DB)) {
    throw new Error(`Gain must be between ${GAIN_MIN_DB} and ${GAIN_MAX_DB} dB`);
  }
  if (node.kind === "parametricEq" && typeof value === "number" && !Number.isFinite(value)) throw new Error("Parametric EQ values must be finite");
  return {
    ...session,
    nodes: session.nodes.map((node, index) => index === nodeIndex
      ? { ...node, parameters: { ...node.parameters, [parameter]: value } }
      : node),
  };
}

/** Restores supported built-in processor parameters to their documented defaults. */
export function resetNodeDraftParameters(session: Session, nodeId: EntityId): Session {
  const node = session.nodes.find((item) => item.id === nodeId);
  if (!node) throw new Error(`Unknown node: ${nodeId}`);
  const definition = libraryNodeDefinitions[node.kind as LibraryNodeKind];
  const parameters = definition ? { ...definition.parameters } : { ...node.parameters };
  return { ...session, nodes: session.nodes.map((item) => item.id === nodeId ? { ...item, parameters } : item) };
}

/** Produces deterministic plan inputs for changed node boolean flags. */
export function describeDraftChanges(base: Session, candidate: Session): DraftChange[] {
  return candidate.nodes.flatMap((node, index) => {
    const original = base.nodes.find((item) => item.id === node.id);
    if (!original) return [];
    const changes: DraftChange[] = [];
    if (original.enabled !== node.enabled) changes.push({ path: `/nodes/${index}/enabled`, value: node.enabled });
    if (original.bypass !== node.bypass) changes.push({ path: `/nodes/${index}/bypass`, value: node.bypass });
    const parameterNames = new Set([...Object.keys(original.parameters), ...Object.keys(node.parameters)].sort());
    for (const parameter of parameterNames) {
      if (original.parameters[parameter] !== node.parameters[parameter] && node.parameters[parameter] !== undefined) {
        changes.push({ path: `/nodes/${index}/parameters/${parameter}`, value: node.parameters[parameter] });
      }
    }
    return changes;
  });
}

/** Plans and commits a draft through the authoritative backend in two phases. */
export async function applyGraphDraft(
  backend: Pick<UiBackend, "planGraph" | "commitGraph">,
  candidate: Session,
  idempotencyKey: string,
  acknowledgments?: string[],
) {
  const plan = await backend.planGraph(candidate);
  if (plan.baseRevision !== candidate.revision) {
    throw new Error("Backend returned a plan for a different session revision");
  }
  if (plan.warnings.length > 0 && acknowledgments === undefined) {
    throw new Error(`Plan requires acknowledgment: ${plan.warnings.join(", ")}`);
  }
  return backend.commitGraph(plan.planId, plan.baseRevision, idempotencyKey, acknowledgments);
}

/** Windows' conventional VST3 and VST2 install folders, scanned by "Scan standard folders". */
export const STANDARD_PLUGIN_FOLDERS = [
  "C:\\Program Files\\Common Files\\VST3",
  "C:\\Program Files\\Common Files\\VST2",
  "C:\\Program Files\\VSTPlugins",
  "C:\\Program Files\\Steinberg\\VSTPlugins",
];

/** A plugin that can be added to the graph, from any remembered scan. */
export type PluginCatalogEntry = { entry: PluginScanEntry; name: string; format: "VST3" | "VST2"; folder: string };

/**
 * Supported x64 plugins from all remembered scans, one per binary (the same
 * plugin found through two folders is listed once), sorted by name.
 */
export function pluginCatalog(inventories: ReadonlyArray<{ directory: string; entries: PluginScanEntry[] }>): PluginCatalogEntry[] {
  const seen = new Set<string>();
  const catalog: PluginCatalogEntry[] = [];
  for (const inventory of inventories) {
    for (const entry of inventory.entries) {
      const identity = entry.identity;
      if (!identity || !["supportedVst2X64Gated", "supportedVst3X64"].includes(identity.compatibility)) continue;
      const key = `${identity.sha256}:${identity.binaryPath.toLowerCase()}`;
      if (seen.has(key)) continue;
      seen.add(key);
      const file = identity.path.split(/[\\/]/).pop() ?? identity.path;
      catalog.push({ entry, name: file.replace(/\.(vst3|dll)$/i, ""), format: identity.format === "vst3" ? "VST3" : "VST2", folder: inventory.directory });
    }
  }
  return catalog.sort((left, right) => left.name.localeCompare(right.name, undefined, { sensitivity: "base" }));
}
