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
import { libraryEntries } from "./library";
import type { LibraryNodeKind } from "./draft";

export const LIBRARY_DROP_SOURCE = "__audiorouter_library_drop__";

type SessionFlowCanvasProps = {
  session: Session;
  selectedNodeId: string;
  selectedNodeIds?: string[];
  onSelect: (id: string) => void;
  onSelectMany?: (ids: string[]) => void;
  onConnect: (connection: Connection) => void;
  onRemoveConnection?: (edgeId: string) => void;
  onAddLibraryNode?: (kind: LibraryNodeKind, position: { x: number; y: number }) => string | void;
  canEdit?: boolean;
};

/** Returns only real graph edges from a canvas deletion event. */
export function deletedConnectionIds(edges: Pick<FlowEdge, "id">[]): string[] {
  return edges.map((edge) => edge.id).filter((id) => id.length > 0);
}

function positionFor(index: number) {
  const columns = 3;
  return {
    x: (index % columns) * 260,
    y: Math.floor(index / columns) * 150,
  };
}

export function libraryDropPosition(clientX: number, clientY: number, bounds: Pick<DOMRect, "left" | "top">): { x: number; y: number } {
  const safeX = Number.isFinite(clientX) ? clientX : bounds.left;
  const safeY = Number.isFinite(clientY) ? clientY : bounds.top;
  return { x: Math.max(0, safeX - bounds.left - 20), y: Math.max(0, safeY - bounds.top - 20) };
}

export function SessionFlowCanvas({ session, selectedNodeId, selectedNodeIds = [selectedNodeId], onSelect, onSelectMany, onConnect, onRemoveConnection, onAddLibraryNode, canEdit = true }: SessionFlowCanvasProps) {
  const layoutKey = `audiorouter.ui.layout.${session.id}`;
  const [positions, setPositions] = useState<LayoutPositions>(() => readLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey));
  useEffect(() => { setPositions(readLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey)); }, [layoutKey]);
  const highlightedNodeIds = relatedNodeIds(session, selectedNodeId);
  const tidyLayout = () => {
    const next = Object.fromEntries(session.nodes.map((node, index) => [node.id, positionFor(index)]));
    setPositions(next);
    writeLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey, next);
  };
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
      border: selectedNodeIds.includes(node.id) ? "2px solid var(--accent, #65d1b5)" : "1px solid var(--line, #40536b)",
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
    deletable: canEdit && onRemoveConnection !== undefined,
    selectable: true,
    style: { stroke: edge.enabled && highlightedNodeIds.has(edge.sourceNode) && highlightedNodeIds.has(edge.destinationNode) ? "#65d1b5" : "#667085", strokeWidth: edge.enabled && highlightedNodeIds.has(edge.sourceNode) && highlightedNodeIds.has(edge.destinationNode) ? 3 : 1, opacity: !highlightedNodeIds.has(edge.sourceNode) || !highlightedNodeIds.has(edge.destinationNode) ? 0.45 : 1 },
  }));

  return (
    <div className="session-flow-canvas" aria-label="Signal-flow graph" onDragOver={(event) => { if (event.dataTransfer.types.includes("application/x-audiorouter-library-kind")) event.preventDefault(); }} onDrop={(event) => { const kind = event.dataTransfer.getData("application/x-audiorouter-library-kind") as LibraryNodeKind; if (!kind) return; event.preventDefault(); const bounds = event.currentTarget.getBoundingClientRect(); const position = libraryDropPosition(event.clientX, event.clientY, bounds); const nodeId = onAddLibraryNode?.(kind, position); if (nodeId) { const next = { ...positions, [nodeId]: position }; setPositions(next); writeLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey, next); } else if (!onAddLibraryNode) onConnect({ source: LIBRARY_DROP_SOURCE, sourceHandle: kind, target: "__drop__", targetHandle: null }); }}>
      <div className="canvas-library" aria-label="Drag processors to canvas"><strong>Drag to canvas</strong>{libraryEntries.filter((entry): entry is typeof entry & { kind: LibraryNodeKind } => entry.kind !== null).map((entry) => <button type="button" key={`drag-${entry.id}`} draggable={canEdit} disabled={!canEdit} onDragStart={(event) => { if (!canEdit) return; event.dataTransfer.effectAllowed = "copy"; event.dataTransfer.setData("application/x-audiorouter-library-kind", entry.kind); }}>{entry.label}</button>)}</div>
      <div className="session-flow-toolbar"><span className="muted">Positions are presentation-only.</span><span className="muted" role="status" aria-live="polite">{selectedNodeIds.length} node{selectedNodeIds.length === 1 ? "" : "s"} selected</span><button type="button" className="secondary" onClick={tidyLayout}>Tidy layout</button><button type="button" className="secondary" onClick={() => { clearLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey); setPositions({}); }}>Reset layout</button></div>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        fitView
        nodesConnectable={canEdit}
        nodesDraggable
        selectionOnDrag
        onSelectionChange={({ nodes: selectedNodes }) => onSelectMany?.(selectedNodes.map((node) => node.id))}
        onNodeClick={(_, node) => onSelect(node.id)}
        onConnect={canEdit ? onConnect : undefined}
        onEdgesDelete={canEdit && onRemoveConnection ? (deleted) => { for (const edgeId of deletedConnectionIds(deleted)) onRemoveConnection(edgeId); } : undefined}
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
