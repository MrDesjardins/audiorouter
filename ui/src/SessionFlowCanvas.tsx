import {
  Background,
  Controls,
  MiniMap,
  ReactFlow,
  type Edge as FlowEdge,
  type Node as FlowNode,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useEffect, useState } from "react";
import type { Session } from "@audiorouter/contracts";
import { readLayout, writeLayout, type LayoutPositions } from "./layout";

type SessionFlowCanvasProps = {
  session: Session;
  selectedNodeId: string;
  onSelect: (id: string) => void;
};

function positionFor(index: number) {
  const columns = 3;
  return {
    x: (index % columns) * 260,
    y: Math.floor(index / columns) * 150,
  };
}

export function SessionFlowCanvas({ session, selectedNodeId, onSelect }: SessionFlowCanvasProps) {
  const layoutKey = `audiorouter.ui.layout.${session.id}`;
  const [positions, setPositions] = useState<LayoutPositions>(() => readLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey));
  useEffect(() => { setPositions(readLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey)); }, [layoutKey]);
  const nodes: FlowNode[] = session.nodes.map((node, index) => ({
    id: node.id,
    position: positions[node.id] ?? positionFor(index),
    data: {
      label: (
        <div className="flow-node-content" aria-label={`${node.name}, ${node.kind}`}>
          <span className="node-kind">{node.kind}</span>
          <strong>{node.name}</strong>
          <small>{node.ports.length} port{node.ports.length === 1 ? "" : "s"} - {node.enabled ? "enabled" : "disabled"}</small>
        </div>
      ),
    },
    draggable: true,
    selectable: true,
    style: {
      border: node.id === selectedNodeId ? "2px solid var(--accent, #65d1b5)" : "1px solid var(--line, #40536b)",
      borderRadius: 10,
      background: "var(--panel, #162132)",
      color: "var(--text, #edf4ff)",
      minWidth: 190,
    },
  }));

  const edges: FlowEdge[] = session.edges.map((edge) => ({
    id: edge.id,
    source: edge.sourceNode,
    target: edge.destinationNode,
    label: `${edge.sourcePort} → ${edge.destinationPort}`,
    animated: false,
    style: { stroke: edge.enabled ? "#65d1b5" : "#667085", strokeWidth: edge.enabled ? 2 : 1 },
  }));

  return (
    <div className="session-flow-canvas" aria-label="Signal-flow graph">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        fitView
        nodesConnectable={false}
        nodesDraggable
        onNodeClick={(_, node) => onSelect(node.id)}
        onNodeDragStop={(_, node) => { const next = { ...positions, [node.id]: node.position }; setPositions(next); writeLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey, next); }}
        proOptions={{ hideAttribution: true }}
      >
        <Background gap={24} size={1} color="#2e4057" />
        <Controls showInteractive={false} />
        <MiniMap pannable zoomable nodeColor="#65d1b5" />
      </ReactFlow>
    </div>
  );
}
