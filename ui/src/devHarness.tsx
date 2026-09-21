// Temporary local-only harness for manually exercising SessionFlowCanvas in a
// real browser without a connected backend. Not part of the shipped app;
// served only via harness.html during interactive UI verification.
import { createRoot } from "react-dom/client";
import { useState } from "react";
import type { Connection } from "@xyflow/react";
import type { DiagnosticsSnapshot, Session } from "@audiorouter/contracts";
import "./styles.css";
import { SessionFlowCanvas } from "./SessionFlowCanvas";
import type { RecorderStatus } from "./backend";
import { appendLibraryNode, appendVirtualBusNode, type LibraryNodeKind } from "./draft";

let nextEdgeId = 1;

const nodeVisualsSession: Session = {
  id: "node-visuals-demo",
  name: "Node visuals demo",
  schemaVersion: 1,
  revision: 1,
  nodes: [
    { id: "mic", kind: "physicalInput", typeVersion: 1, name: "Microphone", enabled: true, bypass: false, parameters: {}, ports: [{ name: "out", direction: "output", channels: 1 }] },
    { id: "test", kind: "testSignal", typeVersion: 1, name: "Test tone", enabled: true, bypass: false, parameters: { frequencyHz: 440, levelDb: -18, durationMs: 1000 }, ports: [{ name: "out", direction: "output", channels: 2 }] },
    { id: "gain", kind: "gain", typeVersion: 1, name: "Voice gain", enabled: true, bypass: false, parameters: { gainDb: 4.5 }, ports: [{ name: "in", direction: "input", channels: 1 }, { name: "out", direction: "output", channels: 1 }] },
    { id: "mute", kind: "mute", typeVersion: 1, name: "Kill switch", enabled: true, bypass: false, parameters: { muted: false }, ports: [{ name: "in", direction: "input", channels: 1 }, { name: "out", direction: "output", channels: 1 }] },
    { id: "comp", kind: "compressor", typeVersion: 1, name: "Voice comp", enabled: true, bypass: false, parameters: { thresholdDb: -18, ratio: 3, attackMs: 10, releaseMs: 150, kneeDb: 6, makeupDb: 0 }, ports: [{ name: "in", direction: "input", channels: 1 }, { name: "out", direction: "output", channels: 1 }] },
    { id: "gate", kind: "gate", typeVersion: 1, name: "Noise gate", enabled: true, bypass: false, parameters: { thresholdDb: -45, rangeDb: 60, hysteresisDb: 3, ratio: 4, attackMs: 5, holdMs: 50, releaseMs: 150 }, ports: [{ name: "in", direction: "input", channels: 1 }, { name: "out", direction: "output", channels: 1 }] },
    { id: "limiter", kind: "limiter", typeVersion: 1, name: "Safety limiter", enabled: true, bypass: false, parameters: { ceilingDb: -1, lookaheadMs: 5, releaseMs: 100 }, ports: [{ name: "in", direction: "input", channels: 1 }, { name: "out", direction: "output", channels: 1 }] },
    { id: "delay", kind: "delay", typeVersion: 1, name: "Slap delay", enabled: true, bypass: false, parameters: { delayMs: 250 }, ports: [{ name: "in", direction: "input", channels: 1 }, { name: "out", direction: "output", channels: 1 }] },
    { id: "pitch", kind: "pitch", typeVersion: 1, name: "Pitch nudge", enabled: true, bypass: false, parameters: { semitones: -2, cents: 0 }, ports: [{ name: "in", direction: "input", channels: 1 }, { name: "out", direction: "output", channels: 1 }] },
    { id: "mixer", kind: "mixer", typeVersion: 1, name: "Sub mix", enabled: true, bypass: false, parameters: {}, ports: [{ name: "in", direction: "input", channels: 2 }, { name: "out", direction: "output", channels: 2 }] },
    { id: "recorder", kind: "recorder", typeVersion: 1, name: "Session capture", enabled: true, bypass: false, parameters: {}, ports: [{ name: "in", direction: "input", channels: 1 }, { name: "out", direction: "output", channels: 1 }] },
    { id: "appcap", kind: "applicationCapture", typeVersion: 1, name: "Zoom capture 1", enabled: false, bypass: false, parameters: { executable: "Zoom.exe", processPolicy: "selectedInstance", processId: 21768 }, ports: [{ name: "out", direction: "output", channels: 2 }] },
    { id: "loopback", kind: "endpointLoopback", typeVersion: 1, name: "Endpoint loopback 1", enabled: false, bypass: false, parameters: { endpointId: "{0.0.0.00000000}.{9b1e2c4a-8f3d-4e21-bd7c-1a2b3c4d5e6f}" }, ports: [{ name: "out", direction: "output", channels: 2 }] },
    { id: "vbus", kind: "virtualCaptureSink", typeVersion: 1, name: "Virtual capture sink 1", enabled: false, bypass: false, parameters: { busId: "voice-bus" }, ports: [{ name: "in", direction: "input", channels: 2 }] },
    { id: "plugin", kind: "plugin", typeVersion: 1, name: "Waves Vendor 1", enabled: true, bypass: false, parameters: { path: "C:\\Program Files\\Common Files\\VST3\\ReaComp.vst3", format: "vst3", fingerprint: "abc123", classId: "default" }, ports: [{ name: "in", direction: "input", channels: 1 }, { name: "out", direction: "output", channels: 1 }] },
    { id: "headphones", kind: "physicalOutput", typeVersion: 1, name: "Headphones", enabled: true, bypass: false, parameters: {}, ports: [{ name: "in", direction: "input", channels: 2 }] },
  ],
  edges: [
    { id: "seed-1", sourceNode: "test", sourcePort: "out", destinationNode: "mixer", destinationPort: "in", matrix: [], enabled: true },
    { id: "seed-2", sourceNode: "mic", sourcePort: "out", destinationNode: "mixer", destinationPort: "in", matrix: [], enabled: true },
  ],
};

const recorderStatuses: RecorderStatus[] = [
  { sessionId: nodeVisualsSession.id, nodeId: "recorder", state: "recording", lastFrame: 48_000 },
];

const diagnostics: DiagnosticsSnapshot = {
  build: "dev-harness",
  backend: "control-plane",
  storage: "memory",
  audio: { state: "available", reason: "" },
  nativeAdapter: "running",
  nativeAdapterKind: null,
  nativeSessionId: null,
  schedulerTelemetry: null,
  nodeTelemetry: [
    { nodeId: "mic", kind: "physicalInput", meter: { peakDb: -12, rmsDb: -18, clippedSamples: 0, channelPeakDb: [-12], channelRmsDb: [-18], channelClippedSamples: [0] }, processor: null, plugin: null },
    { nodeId: "test", kind: "testSignal", meter: { peakDb: -18, rmsDb: -20, clippedSamples: 0, channelPeakDb: [-18, -18], channelRmsDb: [-20, -20], channelClippedSamples: [0, 0] }, processor: null, plugin: null },
    { nodeId: "comp", kind: "compressor", meter: null, processor: { gainReductionDb: [6.5], gateOpen: [] }, plugin: null },
    { nodeId: "gate", kind: "gate", meter: null, processor: { gainReductionDb: [], gateOpen: [true] }, plugin: null },
    { nodeId: "limiter", kind: "limiter", meter: null, processor: { gainReductionDb: [1.2], gateOpen: [] }, plugin: null },
    { nodeId: "mixer", kind: "mixer", meter: { peakDb: -9, rmsDb: -14, clippedSamples: 0, channelPeakDb: [-9, -9], channelRmsDb: [-14, -14], channelClippedSamples: [0, 0] }, processor: null, plugin: null },
    { nodeId: "recorder", kind: "recorder", meter: { peakDb: -9, rmsDb: -14, clippedSamples: 0, channelPeakDb: [-9], channelRmsDb: [-14], channelClippedSamples: [0] }, processor: null, plugin: null },
    { nodeId: "plugin", kind: "plugin", meter: null, processor: null, plugin: { state: "quarantined", failureCount: 3 } },
  ],
  privacyMute: { muted: false, persistence: "memory" },
  recovery: { safeMode: false, recentCrashes: 0, persistence: "memory" },
  eventLog: { latestSequence: 0, retained: 0 },
  redacted: true,
};

function Harness() {
  const [session, setSession] = useState<Session>(nodeVisualsSession);
  const [selectedNodeId, setSelectedNodeId] = useState(nodeVisualsSession.nodes[0].id);

  const onConnect = (connection: Connection) => {
    console.info("[Harness] onConnect", connection);
    const id = `edge-${nextEdgeId++}`;
    setSession((current) => ({
      ...current,
      edges: [
        ...current.edges,
        {
          id,
          sourceNode: connection.source,
          sourcePort: connection.sourceHandle ?? "out",
          destinationNode: connection.target,
          destinationPort: connection.targetHandle ?? "in",
          matrix: [],
          enabled: true,
        },
      ],
    }));
    return id;
  };

  const onRemoveConnection = (edgeId: string) => {
    setSession((current) => ({ ...current, edges: current.edges.filter((edge) => edge.id !== edgeId) }));
  };

  const onSetNodeParameter = (nodeId: string, name: string, value: boolean | number | string) => {
    console.info("[Harness] onSetNodeParameter", nodeId, name, value);
    setSession((current) => ({
      ...current,
      nodes: current.nodes.map((node) => (node.id === nodeId ? { ...node, parameters: { ...node.parameters, [name]: value } } : node)),
    }));
  };

  const onAddLibraryNode = (kind: LibraryNodeKind) => {
    const next = appendLibraryNode(session, kind);
    const inserted = next.nodes.at(-1);
    console.info("[Harness] onAddLibraryNode", kind, inserted?.id);
    setSession(next);
    return inserted?.id;
  };

  const onAddVirtualBusNode = (direction: "renderSource" | "captureSink") => {
    const next = appendVirtualBusNode(session, "voice-bus", direction);
    const inserted = next.nodes.at(-1);
    console.info("[Harness] onAddVirtualBusNode", direction, inserted?.id);
    setSession(next);
    return inserted?.id;
  };

  return (
    <div style={{ height: "100vh", background: "#10131a" }}>
      <SessionFlowCanvas
        session={session}
        selectedNodeId={selectedNodeId}
        onSelect={setSelectedNodeId}
        canEdit
        diagnostics={diagnostics}
        recorderStatuses={recorderStatuses}
        onConnect={onConnect}
        onRemoveConnection={onRemoveConnection}
        onSetNodeParameter={onSetNodeParameter}
        onAddLibraryNode={onAddLibraryNode}
        onAddVirtualBusNode={onAddVirtualBusNode}
        onOpenPluginPicker={() => console.info("[Harness] onOpenPluginPicker")}
        onToggleConnection={(edgeId, enabled) => setSession((current) => ({ ...current, edges: current.edges.map((edge) => (edge.id === edgeId ? { ...edge, enabled } : edge)) }))}
      />
    </div>
  );
}

createRoot(document.getElementById("root")!).render(<Harness />);
