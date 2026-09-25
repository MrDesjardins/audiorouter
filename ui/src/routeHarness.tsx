// Local E2E app harness: exercises real workspace and graph editing against
// a deterministic in-memory preview backend. It never opens audio devices.
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { createDisconnectedBackend, type UiBackend } from "./backend";
import { demoSession } from "./fixtures";
import type { Session } from "@audiorouter/contracts";
import "./styles.css";

let planned: Session | null = null;
let committed = structuredClone(demoSession);
let previewCandidate: Session | null = null;
let running = false;
let prepared = false;
const sourceStates = new Map<string, "playing" | "paused" | "stopped">();
let sequence = 0;
const fixtureBackend = createDisconnectedBackend(demoSession);
const initial = await fixtureBackend.snapshot();
const diagnostics = () => ({
  ...initial.diagnostics,
  audio: { state: running ? "available" as const : "unavailable" as const, reason: "Browser fixture: simulated audio; no devices are opened." },
  nodeTelemetry: running && (!(previewCandidate ?? committed).nodes.some((node) => node.kind === "testSignal") || (previewCandidate ?? committed).nodes.some((node) => node.kind === "testSignal" && sourceStates.get(node.id) === "playing")) ? (previewCandidate ?? committed).nodes.filter((node) => node.kind === "physicalOutput").map((node) => {
    const rmsDb = -28 + Math.sin(Date.now() / 300) * 10;
    return { nodeId: node.id, kind: node.kind, meter: { peakDb: rmsDb + 3, rmsDb, clippedSamples: 0, channelPeakDb: [rmsDb + 3], channelRmsDb: [rmsDb], channelClippedSamples: [0] }, processor: null, plugin: null };
  }) : [],
  nativeSessionId: prepared ? committed.id : null,
  nativeAdapter: prepared ? running ? "running" as const : "configured-stopped" as const : "implemented-not-activated" as const,
  nativeAdapterKind: prepared ? "endpoint" as const : null,
});

function graphValidationError(session: Session): string | null {
  const sources = session.nodes.filter((node) => ["physicalInput", "testSignal", "audioFile", "applicationCapture", "endpointLoopback"].includes(node.kind)).map((node) => node.id);
  const outputs = new Set(session.nodes.filter((node) => ["physicalOutput", "recorder"].includes(node.kind)).map((node) => node.id));
  const nodes = new Map(session.nodes.map((node) => [node.id, node]));
  const outgoing = new Map<string, string[]>();
  const seenEdges = new Set<string>();
  const destinationCounts = new Map<string, number>();
  for (const edge of session.edges) {
    const source = nodes.get(edge.sourceNode);
    const destination = nodes.get(edge.destinationNode);
    const sourcePort = source?.ports.find((port) => port.name === edge.sourcePort);
    const destinationPort = destination?.ports.find((port) => port.name === edge.destinationPort);
    if (!source || !destination || !sourcePort || sourcePort.direction !== "output" || !destinationPort || destinationPort.direction !== "input") return "Preview planner: every connection needs an existing output and input port.";
    if (edge.matrix.length !== sourcePort.channels * destinationPort.channels || edge.matrix.some((value) => !Number.isFinite(value) || value < -2 || value > 2)) return "Preview planner: a connection has an invalid channel matrix.";
    const edgeKey = `${edge.sourceNode}\u0000${edge.sourcePort}\u0000${edge.destinationNode}\u0000${edge.destinationPort}`;
    if (seenEdges.has(edgeKey)) return "Preview planner: duplicate connections are not allowed.";
    seenEdges.add(edgeKey);
    const destinationKey = `${edge.destinationNode}\u0000${edge.destinationPort}`;
    const destinationCount = (destinationCounts.get(destinationKey) ?? 0) + 1;
    destinationCounts.set(destinationKey, destinationCount);
    if (destinationCount > 1 && destination.kind !== "mixer") return "Preview planner: only mixer inputs can have multiple incoming connections.";
    outgoing.set(edge.sourceNode, [...(outgoing.get(edge.sourceNode) ?? []), edge.destinationNode]);
  }
  for (const source of sources) {
    const visited = new Set<string>([source]);
    const queue = [source];
    while (queue.length > 0) {
      const current = queue.shift()!;
      for (const destination of outgoing.get(current) ?? []) {
        if (outputs.has(destination)) return null;
        if (!visited.has(destination)) { visited.add(destination); queue.push(destination); }
      }
    }
  }
  return "Preview planner: connect an input to an output before planning.";
}

const previewBackend: UiBackend = {
  ...fixtureBackend,
  connected: true,
  async snapshot() {
    return structuredClone({ ...initial, session: committed, diagnostics: diagnostics(), status: { ...initial.status, activeSessionIds: running ? [committed.id] : [], reason: "Browser fixture: simulated audio; no devices are opened." } });
  },
  async listSessions() { return [structuredClone(committed)]; },
  async processorResponse({ bands, frequenciesHz }) {
    // Browser-only shape fixture for visual interaction; the desktop backend
    // uses the actual DSP coefficients for this response.
    return { frequenciesHz, magnitudeDb: frequenciesHz.map((frequency) => bands.reduce((sum, band) => {
      if (band.enabled === false) return sum;
      const octaves = Math.log2(frequency / band.frequencyHz);
      const bell = Math.exp(-0.5 * (octaves * band.q) ** 2);
      if (band.type === "notch") return sum - 18 * bell;
      if (band.type === "lowPass") return sum - Math.max(0, octaves) * 12;
      if (band.type === "highPass") return sum + Math.min(0, octaves) * 12;
      if (band.type === "lowShelf") return sum + band.gainDb / (1 + Math.exp(octaves * 4));
      if (band.type === "highShelf") return sum + band.gainDb / (1 + Math.exp(-octaves * 4));
      return sum + band.gainDb * bell;
    }, 0)) };
  },
  async refreshDiagnostics() { return structuredClone(diagnostics()); },
  async subscribe(afterSequence = 0) { return { backendEpoch: 1, events: [], nextSequence: sequence, resyncRequired: afterSequence < sequence }; },
  async prepareNativeEndpoint(sessionId, captureEndpointId, renderEndpointId) {
    if (sessionId !== committed.id || captureEndpointId !== "capture-preview" || renderEndpointId !== "render-preview") throw new Error("Fixture endpoint mismatch");
    prepared = true; sequence += 1;
    return { sessionId, captureEndpointId, renderEndpointId, state: "configured-stopped" };
  },
  async startSession(sessionId, _idempotencyKey, candidate) {
    if (!prepared || sessionId !== committed.id) throw new Error("Fixture audio is not prepared");
    if (candidate) {
      const validationError = graphValidationError(candidate);
      if (validationError) throw new Error(validationError);
      previewCandidate = structuredClone(candidate);
    } else previewCandidate = null;
    running = true; sequence += 1;
    // Simulates the native lifecycle response for UI regressions only.
    return { sessionId, state: "running", generation: 1, runtime: "native" };
  },
  async stopSession(sessionId) {
    running = false; previewCandidate = null; sourceStates.clear(); sequence += 1;
    return { sessionId, state: "stopped", runtime: "native", recorders: [] };
  },
  async transportAudioSource(sessionId, nodeId, action) {
    if (!running || sessionId !== committed.id) throw new Error("Fixture route is stopped");
    const node = (previewCandidate ?? committed).nodes.find((candidate) => candidate.id === nodeId);
    if (!node || !["audioFile", "testSignal"].includes(node.kind)) throw new Error("Fixture source is unavailable");
    if (action === "play") sourceStates.set(nodeId, "playing");
    if (action === "pause") sourceStates.set(nodeId, "paused");
    if (action === "stop") sourceStates.set(nodeId, "stopped");
    sequence += 1;
    return { sessionId, nodeId, state: sourceStates.get(nodeId) ?? "stopped", loop: false };
  },
  async listApplications() {
    return [{ processId: 4242, executable: "Music.exe", executablePath: "C:\\Apps\\Music.exe", audioDisplayNames: ["Music"], audioActivity: "active", captureCapability: "observed", audioSessionCount: 1, activeAudioSessionCount: 1, captureSessionCount: 1, renderSessionCount: 1, creationTime100ns: "123456789" }];
  },
  async listDevices() {
    const format = { sampleRateHz: 48_000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 };
    const periods = { default100ns: 100_000, minimum100ns: 30_000 };
    return [
      { id: "capture-preview", name: "Preview Microphone", direction: "capture", state: "active", defaultRoles: [], format, periods },
      { id: "render-preview", name: "Preview Output", direction: "render", state: "active", defaultRoles: [], format, periods },
    ];
  },
  async beginAudioUpload(fileName, sizeBytes) {
    if (!fileName.match(/\.(wav|mp3)$/i) || sizeBytes > 64 * 1024 * 1024) throw new Error("Preview upload rejected.");
    return { uploadId: "e2e-audio-upload", chunkBytes: 64 * 1024 };
  },
  async uploadAudioChunk(_uploadId, _chunkIndex, _dataBase64) {
    return { receivedBytes: 4, nextChunkIndex: 1 };
  },
  async finishAudioUpload() {
    return { mediaId: "e2e-audio-media", fileName: "calibration.wav", format: "wav", durationMs: 1000, channels: 1, sampleRateHz: 48_000 };
  },
  async planGraph(candidate) {
    if (candidate.id !== committed.id || candidate.revision !== committed.revision) throw new Error(`Preview planner: stale session revision (${candidate.id}@${candidate.revision}; saved ${committed.id}@${committed.revision}).`);
    const validationError = graphValidationError(candidate);
    if (validationError) throw new Error(validationError);
    planned = structuredClone(candidate);
    return { planId: "e2e-graph-plan", baseRevision: candidate.revision, expiresInMs: 60_000, diff: [], affectedDestinations: [], warnings: [], requiredScopes: [] };
  },
  async commitGraph(_planId, baseRevision) {
    if (!planned || planned.revision !== baseRevision || committed.revision !== baseRevision) throw new Error("Preview planner: no matching graph plan.");
    committed = { ...structuredClone(planned), revision: baseRevision + 1 };
    planned = null; sequence += 1;
    return { sessionId: committed.id, revision: committed.revision };
  },
};

createRoot(document.getElementById("root")!).render(<App backend={previewBackend} />);
