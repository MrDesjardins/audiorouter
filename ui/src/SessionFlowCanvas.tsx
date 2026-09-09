import {
  Background,
  Controls,
  Handle,
  MiniMap,
  Position,
  ReactFlow,
  type Connection,
  type Edge as FlowEdge,
  type Node as FlowNode,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useEffect, useState } from "react";
import type { Session } from "@audiorouter/contracts";
import { clearLayout, readLayout, writeLayout, type LayoutPositions } from "./layout";
import { nodePortLabels, relatedNodeIds } from "./graphView";

type SessionFlowCanvasProps = {
  session: Session;
  selectedNodeId: string;
  onSelect: (id: string) => void;
  onSelectMany?: (ids: string[]) => void;
  onConnect: (connection: Connection) => void;
};

function positionFor(index: number) {
  const columns = 3;
  return {
    x: (index % columns) * 260,
    y: Math.floor(index / columns) * 150,
  };
}

export function SessionFlowCanvas({ session, selectedNodeId, onSelect, onSelectMany, onConnect }: SessionFlowCanvasProps) {
  const layoutKey = `audiorouter.ui.layout.${session.id}`;
  const [positions, setPositions] = useState<LayoutPositions>(() => readLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey));
  useEffect(() => { setPositions(readLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey)); }, [layoutKey]);
  const highlightedNodeIds = relatedNodeIds(session, selectedNodeId);
  const nodes: FlowNode[] = session.nodes.map((node, index) => ({
    id: node.id,
    position: positions[node.id] ?? positionFor(index),
    data: {
      label: (
        <div className="flow-node-content" aria-label={`${node.name}, ${node.kind}`}>
          {node.ports.filter((port) => port.direction === "input").map((port, portIndex) => <Handle key={`input-${port.name}`} type="target" id={port.name} position={Position.Left} style={{ top: `${35 + portIndex * 18}px` }} aria-label={`${node.name} ${port.name} input`} />)}
          <span className="node-kind">{node.kind}</span>
          <strong>{node.name}</strong>
          <small>{node.ports.length} port{node.ports.length === 1 ? "" : "s"} - {node.enabled ? "enabled" : "disabled"}</small>
          <span className="flow-port-list">{nodePortLabels(node).map((port) => <small key={port}>{port}</small>)}</span>
          {node.ports.filter((port) => port.direction === "output").map((port, portIndex) => <Handle key={`output-${port.name}`} type="source" id={port.name} position={Position.Right} style={{ top: `${35 + portIndex * 18}px` }} aria-label={`${node.name} ${port.name} output`} />)}
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
      opacity: highlightedNodeIds.has(node.id) ? 1 : 0.55,
    },
  }));

  const edges: FlowEdge[] = session.edges.map((edge) => ({
    id: edge.id,
    source: edge.sourceNode,
    target: edge.destinationNode,
    label: `${edge.sourcePort} → ${edge.destinationPort}`,
    animated: false,
    style: { stroke: edge.enabled && highlightedNodeIds.has(edge.sourceNode) && highlightedNodeIds.has(edge.destinationNode) ? "#65d1b5" : "#667085", strokeWidth: edge.enabled && highlightedNodeIds.has(edge.sourceNode) && highlightedNodeIds.has(edge.destinationNode) ? 3 : 1, opacity: !highlightedNodeIds.has(edge.sourceNode) || !highlightedNodeIds.has(edge.destinationNode) ? 0.45 : 1 },
  }));

  return (
    <div className="session-flow-canvas" aria-label="Signal-flow graph">
      <div className="session-flow-toolbar"><span className="muted">Positions are presentation-only.</span><button type="button" className="secondary" onClick={() => { clearLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey); setPositions({}); }}>Reset layout</button></div>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        fitView
        nodesConnectable
        nodesDraggable
        selectionOnDrag
        onSelectionChange={({ nodes: selectedNodes }) => onSelectMany?.(selectedNodes.map((node) => node.id))}
        onNodeClick={(_, node) => onSelect(node.id)}
        onConnect={onConnect}
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
