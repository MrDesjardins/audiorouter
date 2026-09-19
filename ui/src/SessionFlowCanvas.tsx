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
import { useEffect, useRef, useState } from "react";
import type { Session } from "@audiorouter/contracts";
import { clearLayout, readLayout, writeLayout, type LayoutPositions } from "./layout";
import { nodePortLabels, relatedNodeIds } from "./graphView";
import { libraryEntries } from "./library";
import type { LibraryNodeKind } from "./draft";

export const LIBRARY_DROP_SOURCE = "__audiorouter_library_drop__";
export const LIBRARY_DROP_MIME = "application/x-audiorouter-library-kind";
const LIBRARY_DROP_TEXT_MIME = "text/plain";

type SessionFlowCanvasProps = {
  session: Session;
  selectedNodeId: string;
  selectedNodeIds?: string[];
  onSelect: (id: string) => void;
  onSelectMany?: (ids: string[]) => void;
  onConnect: (connection: Connection, dropPosition?: { x: number; y: number }) => string | void;
  onRemoveConnection?: (edgeId: string) => void;
  onRemoveNode?: (nodeId: string) => void;
  onAddLibraryNode?: (kind: LibraryNodeKind, position: { x: number; y: number }) => string | void;
  onAddVirtualBusNode?: (direction: "renderSource" | "captureSink", position: { x: number; y: number }) => string | void;
  canEdit?: boolean;
};

/** Returns only real graph edges from a canvas deletion event. */
export function deletedConnectionIds(edges: Pick<FlowEdge, "id">[]): string[] {
  return edges.map((edge) => edge.id).filter((id) => id.length > 0);
}

/** Returns only non-empty node identities from a canvas deletion event. */
export function deletedNodeIds(nodes: Pick<FlowNode, "id">[]): string[] {
  return nodes.map((node) => node.id).filter((id) => id.length > 0);
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

function isLibraryNodeKind(value: string): value is LibraryNodeKind {
  return libraryEntries.some((entry) => entry.kind === value);
}

function isVirtualBusKind(value: string): value is "virtualRenderSource" | "virtualCaptureSink" {
  return value === "virtualRenderSource" || value === "virtualCaptureSink";
}

function readLibraryDropKind(dataTransfer: DataTransfer): string {
  return dataTransfer.getData(LIBRARY_DROP_MIME) || dataTransfer.getData(LIBRARY_DROP_TEXT_MIME);
}

export function SessionFlowCanvas({ session, selectedNodeId, selectedNodeIds = [selectedNodeId], onSelect, onSelectMany, onConnect, onRemoveConnection, onRemoveNode, onAddLibraryNode, onAddVirtualBusNode, canEdit = true }: SessionFlowCanvasProps) {
  const layoutKey = `audiorouter.ui.layout.${session.id}`;
  const [positions, setPositions] = useState<LayoutPositions>(() => readLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey));
  const positionsRef = useRef(positions);
  useEffect(() => {
    const next = readLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey);
    positionsRef.current = next;
    setPositions(next);
  }, [layoutKey]);
  const highlightedNodeIds = relatedNodeIds(session, selectedNodeId);
  const tidyLayout = () => {
    const next = Object.fromEntries(session.nodes.map((node, index) => [node.id, positionFor(index)]));
    positionsRef.current = next;
    setPositions(next);
    writeLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey, next);
  };
  const addLibraryNode = (kind: LibraryNodeKind, position: { x: number; y: number }) => {
    const nodeId = onAddLibraryNode?.(kind, position);
    if (nodeId) {
      const next = { ...positionsRef.current, [nodeId]: position };
      positionsRef.current = next;
      setPositions(next);
      writeLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey, next);
    }
  };
  const addVirtualBusNode = (direction: "renderSource" | "captureSink", position: { x: number; y: number }) => {
    const nodeId = onAddVirtualBusNode?.(direction, position);
    if (nodeId) {
      const next = { ...positionsRef.current, [nodeId]: position };
      positionsRef.current = next;
      setPositions(next);
      writeLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey, next);
    }
  };
  const routeLibraryDrop = (kind: string, position: { x: number; y: number }) => {
    const nodeId = onConnect({ source: LIBRARY_DROP_SOURCE, sourceHandle: kind, target: "__drop__", targetHandle: null }, position);
    if (nodeId) {
      const next = { ...positionsRef.current, [nodeId]: position };
      positionsRef.current = next;
      setPositions(next);
      writeLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey, next);
    }
  };
  const nodes: FlowNode[] = session.nodes.map((node, index) => ({
    id: node.id,
    position: positions[node.id] ?? positionFor(index),
    data: {
      label: (
        <div className="flow-node-content" aria-label={`${node.name}, ${node.kind}`}>
          {node.ports.filter((port) => port.direction === "input").map((port, portIndex) => <Handle key={`input-${port.name}`} type="target" id={port.name} position={Position.Left} style={{ top: `${35 + portIndex * 18}px` }} aria-label={`${node.name} ${port.name} input`} />)}
          <span className="node-kind">{node.kind}</span>
          <div className="flow-node-title"><strong>{node.name}</strong>{canEdit && <button type="button" className="flow-node-delete" aria-label={`Delete ${node.name}`} title={`Delete ${node.name}`} onClick={(event) => { event.stopPropagation(); if (onRemoveNode) onRemoveNode(node.id); else globalThis.dispatchEvent(new CustomEvent("audiorouter:remove-node", { detail: { nodeId: node.id } })); }}>×</button>}</div>
          <small>{node.ports.length} port{node.ports.length === 1 ? "" : "s"} - {node.enabled ? "enabled" : "disabled"}</small>
          <span className="flow-port-list">{nodePortLabels(node).map((port) => <small key={port}>{port}</small>)}</span>
          {node.ports.filter((port) => port.direction === "output").map((port, portIndex) => <Handle key={`output-${port.name}`} type="source" id={port.name} position={Position.Right} style={{ top: `${35 + portIndex * 18}px` }} aria-label={`${node.name} ${port.name} output`} />)}
        </div>
      ),
    },
    draggable: true,
    selectable: true,
    deletable: canEdit,
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
    <div className="session-flow-canvas" aria-label="Signal-flow graph" onDragOver={(event) => { if (!canEdit) return; event.preventDefault(); event.dataTransfer.dropEffect = "copy"; }} onDrop={(event) => { const kind = readLibraryDropKind(event.dataTransfer); event.preventDefault(); const bounds = event.currentTarget.getBoundingClientRect(); const position = libraryDropPosition(event.clientX, event.clientY, bounds); if (isLibraryNodeKind(kind)) { if (onAddLibraryNode) addLibraryNode(kind, position); else routeLibraryDrop(kind, position); } else if (isVirtualBusKind(kind)) { if (onAddVirtualBusNode) addVirtualBusNode(kind === "virtualRenderSource" ? "renderSource" : "captureSink", position); else routeLibraryDrop(kind, position); } }}>
      <div className="canvas-library" aria-label="Drag processors to canvas"><strong>Drag or select to add</strong>{libraryEntries.filter((entry) => entry.kind !== undefined || entry.virtualKind !== undefined).map((entry) => { const kind = entry.kind ?? entry.virtualKind!; const unavailable = Boolean(entry.unavailableReason); return <button type="button" key={`drag-${entry.id}`} draggable={canEdit && !unavailable} disabled={!canEdit || unavailable} title={entry.unavailableReason} onClick={() => { if (unavailable) return; if (entry.kind) addLibraryNode(entry.kind, positionFor(session.nodes.length)); else if (onAddVirtualBusNode) addVirtualBusNode(entry.virtualKind === "virtualRenderSource" ? "renderSource" : "captureSink", positionFor(session.nodes.length)); else routeLibraryDrop(entry.virtualKind!, positionFor(session.nodes.length)); }} onDragStart={(event) => { if (!canEdit || unavailable) return; event.dataTransfer.effectAllowed = "copy"; event.dataTransfer.setData(LIBRARY_DROP_MIME, kind); event.dataTransfer.setData(LIBRARY_DROP_TEXT_MIME, kind); }}>{entry.label}</button>; })}</div>
      <div className="session-flow-toolbar"><span className="muted">Positions are presentation-only.</span><span className="muted" role="status" aria-live="polite">{selectedNodeIds.length} node{selectedNodeIds.length === 1 ? "" : "s"} selected</span><button type="button" className="secondary" onClick={tidyLayout}>Tidy layout</button><button type="button" className="secondary" onClick={() => { clearLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey); positionsRef.current = {}; setPositions({}); }}>Reset layout</button></div>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        fitView
        onInit={(instance) => { globalThis.requestAnimationFrame(() => instance.fitView({ padding: 0.2 })); }}
        nodesConnectable={canEdit}
        nodesDraggable
        selectionOnDrag
        onSelectionChange={({ nodes: selectedNodes }) => onSelectMany?.(selectedNodes.map((node) => node.id))}
        onNodeClick={(_, node) => onSelect(node.id)}
        onConnect={canEdit ? onConnect : undefined}
        onEdgesDelete={canEdit && onRemoveConnection ? (deleted) => { for (const edgeId of deletedConnectionIds(deleted)) onRemoveConnection(edgeId); } : undefined}
        onNodesDelete={canEdit ? (deleted) => { for (const nodeId of deletedNodeIds(deleted)) { if (onRemoveNode) onRemoveNode(nodeId); else globalThis.dispatchEvent(new CustomEvent("audiorouter:remove-node", { detail: { nodeId } })); } } : undefined}
        onNodeDragStop={(_, node) => { const next = { ...positionsRef.current, [node.id]: node.position }; positionsRef.current = next; setPositions(next); writeLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey, next); }}
        proOptions={{ hideAttribution: true }}
      >
        <Background gap={24} size={1} color="#2e4057" />
        <Controls showInteractive={false} />
        <MiniMap pannable zoomable nodeColor="#65d1b5" />
      </ReactFlow>
    </div>
  );
}
