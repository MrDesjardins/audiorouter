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
    { id: "gain", kind: "gain", typeVersion: 1, name: "Voice gain", enabled: true, bypass: false, parameters: { gainDb: 4.5 }, ports: [{ name: "in", direction: "input", channels: 1 }, { name: "out", direction: "output", channels: 1 }] },
    { id: "headphones", kind: "physicalOutput", typeVersion: 1, name: "Headphones", enabled: true, bypass: false, parameters: {}, ports: [{ name: "in", direction: "input", channels: 2 }] },
  ],
  edges: [
    { id: "mic-to-gain", sourceNode: "mic", sourcePort: "out", destinationNode: "gain", destinationPort: "in", matrix: [], enabled: true },
    { id: "gain-to-headphones", sourceNode: "gain", sourcePort: "out", destinationNode: "headphones", destinationPort: "in", matrix: [], enabled: true },
  ],
};

const recorderStatuses: RecorderStatus[] = [];

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
    { nodeId: "gain", kind: "gain", meter: null, processor: null, plugin: null },
    { nodeId: "headphones", kind: "physicalOutput", meter: { peakDb: -9, rmsDb: -14, clippedSamples: 0, channelPeakDb: [-9, -9], channelRmsDb: [-14, -14], channelClippedSamples: [0, 0] }, processor: null, plugin: null },
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
    <div className="flow-harness" style={{ height: "100vh", background: "#10131a" }}>
      <header className="flow-harness-caption"><strong>Signal flow preview</strong><span>Simulated telemetry · visual check only · no audio device is connected</span></header>
      <SessionFlowCanvas
        session={session}
        selectedNodeId={selectedNodeId}
        sessionRunning
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
