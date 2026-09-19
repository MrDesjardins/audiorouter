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
  type ReactFlowInstance,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useEffect, useRef, useState, type PointerEvent } from "react";
import type { DiagnosticsSnapshot, Node, Session } from "@audiorouter/contracts";
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
  diagnostics?: DiagnosticsSnapshot | null;
  onSetNodeParameter?: (nodeId: string, name: string, value: boolean | number | string) => void;
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

function clamp(value: number, minimum: number, maximum: number) {
  return Math.min(maximum, Math.max(minimum, value));
}

function dbPercent(value: number) {
  return clamp(((value + 60) / 60) * 100, 0, 100);
}

function formatDb(value: number) {
  return value <= -59.9 ? "−∞" : `${value.toFixed(1)} dB`;
}

function nodeTelemetryFor(node: Node | undefined, diagnostics: DiagnosticsSnapshot | null | undefined) {
  return node ? diagnostics?.nodeTelemetry.find((item) => item.nodeId === node.id) ?? null : null;
}

function MiniMeter({ telemetry }: { telemetry: ReturnType<typeof nodeTelemetryFor> }) {
  const peak = telemetry?.meter?.peakDb ?? -60;
  const rms = telemetry?.meter?.rmsDb ?? -60;
  const active = peak > -60;
  return <div className={`node-meter ${active ? "is-active" : "is-silent"}`} aria-label={`Peak ${formatDb(peak)}, RMS ${formatDb(rms)}`}>
    <div className="node-meter-track"><span className="node-meter-rms" style={{ height: `${dbPercent(rms)}%` }} /><span className="node-meter-peak" style={{ height: `${dbPercent(peak)}%` }} /></div>
    <div className="node-meter-readout"><span>RMS {formatDb(rms)}</span><span>PK {formatDb(peak)}</span></div>
  </div>;
}

type EqBand = { index: number; enabled: boolean; frequencyHz: number; gainDb: number; q: number; type: string };

function eqBandsFor(node: Node): EqBand[] {
  return Array.from({ length: 8 }, (_, index) => {
    const prefix = `band${index}`;
    return {
      index,
      enabled: node.parameters[`${prefix}Enabled`] !== false,
      frequencyHz: Number(node.parameters[`${prefix}FrequencyHz`] ?? 1000),
      gainDb: Number(node.parameters[`${prefix}GainDb`] ?? 0),
      q: Number(node.parameters[`${prefix}Q`] ?? 1),
      type: String(node.parameters[`${prefix}Type`] ?? "peaking"),
    };
  }).filter((band) => band.enabled || band.frequencyHz > 0);
}

function eqX(frequencyHz: number) {
  return 8 + (Math.log10(clamp(frequencyHz, 20, 20_000) / 20) / 3) * 184;
}

function eqY(gainDb: number) {
  return 44 - clamp(gainDb, -12, 12) * 2.45;
}

export function eqBandCoordinates(frequencyHz: number, gainDb: number) {
  return { x: eqX(frequencyHz), y: eqY(gainDb) };
}

export function telemetrySignalActive(telemetry: ReturnType<typeof nodeTelemetryFor>) {
  return (telemetry?.meter?.peakDb ?? -60) > -60;
}

function MiniEq({ node, onSetNodeParameter }: { node: Node; onSetNodeParameter?: SessionFlowCanvasProps["onSetNodeParameter"] }) {
  const bands = eqBandsFor(node);
  const updateBand = (band: EqBand, event: PointerEvent<SVGSVGElement>) => {
    if (!onSetNodeParameter) return;
    const bounds = event.currentTarget.getBoundingClientRect();
    const x = clamp(event.clientX - bounds.left - 8, 0, 184);
    const y = clamp(event.clientY - bounds.top, 0, 88);
    const frequencyHz = 20 * (10 ** (x / 184 * 3));
    const gainDb = clamp((44 - y) / 2.45, -12, 12);
    onSetNodeParameter(node.id, `band${band.index}FrequencyHz`, Math.round(frequencyHz));
    onSetNodeParameter(node.id, `band${band.index}GainDb`, Number(gainDb.toFixed(1)));
  };
  return <div className="node-eq" aria-label="Equalizer response preview">
    <svg viewBox="0 0 200 88" role="img" aria-label="Parametric EQ curve" onPointerMove={(event) => { const rawIndex = event.currentTarget.dataset.dragBand; if (!rawIndex) return; const index = Number(rawIndex); if (Number.isInteger(index) && index >= 0 && bands[index]) updateBand(bands[index], event); }} onPointerUp={(event) => { event.currentTarget.dataset.dragBand = ""; }}>
      <path className="eq-grid" d="M8 14H192M8 44H192M8 74H192M8 8V80M69 8V80M130 8V80M192 8V80" />
      <path className="eq-curve" d={bands.length ? `M8 44 ${bands.map((band) => `L ${eqX(band.frequencyHz).toFixed(1)} ${eqY(band.gainDb).toFixed(1)}`).join(" ")} L192 44` : "M8 44H192"} />
      {bands.map((band) => <circle key={band.index} className="eq-band" cx={eqX(band.frequencyHz)} cy={eqY(band.gainDb)} r="4" tabIndex={0} role="slider" aria-valuemin={-12} aria-valuemax={12} aria-valuenow={band.gainDb} aria-label={`Band ${band.index + 1}, ${band.frequencyHz} Hz, ${band.gainDb} dB`} onKeyDown={(event) => { if (!onSetNodeParameter) return; const gainStep = event.key === "ArrowUp" ? 0.5 : event.key === "ArrowDown" ? -0.5 : 0; const frequencyStep = event.key === "ArrowRight" ? 1.08 : event.key === "ArrowLeft" ? 1 / 1.08 : 1; if (gainStep !== 0 || frequencyStep !== 1) { event.preventDefault(); if (gainStep !== 0) onSetNodeParameter(node.id, `band${band.index}GainDb`, Number(clamp(band.gainDb + gainStep, -12, 12).toFixed(1))); if (frequencyStep !== 1) onSetNodeParameter(node.id, `band${band.index}FrequencyHz`, Math.round(clamp(band.frequencyHz * frequencyStep, 20, 20_000))); } }} onPointerDown={(event) => { event.stopPropagation(); event.currentTarget.ownerSVGElement?.setPointerCapture(event.pointerId); event.currentTarget.ownerSVGElement?.dataset && (event.currentTarget.ownerSVGElement.dataset.dragBand = String(band.index)); }} />)}
    </svg>
    <div className="node-eq-scale"><span>20 Hz</span><span>1 kHz</span><span>20 kHz</span></div>
  </div>;
}

function NodeVisual({ node, telemetry, onSetNodeParameter }: { node: Node; telemetry: ReturnType<typeof nodeTelemetryFor>; onSetNodeParameter?: SessionFlowCanvasProps["onSetNodeParameter"] }) {
  if (node.kind === "parametricEq" || node.kind === "graphicEq") return <MiniEq node={node} onSetNodeParameter={onSetNodeParameter} />;
  if (telemetry?.meter || ["physicalInput", "physicalOutput", "meter", "testSignal", "recorder"].includes(node.kind)) return <MiniMeter telemetry={telemetry} />;
  return <div className="node-activity" aria-label={node.enabled && !node.bypass ? "Processor ready" : "Processor inactive"}><span className="activity-dot" />{node.bypass ? "Bypassed" : node.enabled ? "Ready" : "Disabled"}</div>;
}

export function SessionFlowCanvas({ session, selectedNodeId, selectedNodeIds = [selectedNodeId], onSelect, onSelectMany, onConnect, onRemoveConnection, onRemoveNode, onAddLibraryNode, onAddVirtualBusNode, diagnostics, onSetNodeParameter, canEdit = true }: SessionFlowCanvasProps) {
  const layoutKey = `audiorouter.ui.layout.${session.id}`;
  const [positions, setPositions] = useState<LayoutPositions>(() => readLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey));
  const positionsRef = useRef(positions);
  const flowInstanceRef = useRef<ReactFlowInstance | null>(null);
  const initialFitDoneRef = useRef(false);
  useEffect(() => {
    initialFitDoneRef.current = false;
    const next = readLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey);
    positionsRef.current = next;
    setPositions(next);
  }, [layoutKey]);
  useEffect(() => {
    if (initialFitDoneRef.current || !flowInstanceRef.current || session.nodes.length === 0) return;
    const frame = globalThis.requestAnimationFrame(() => flowInstanceRef.current?.fitView({ padding: 0.2 }));
    initialFitDoneRef.current = true;
    return () => globalThis.cancelAnimationFrame(frame);
  }, [session.nodes.length]);
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
  const nodes: FlowNode[] = session.nodes.map((node, index) => {
    const telemetry = nodeTelemetryFor(node, diagnostics);
    return ({
    id: node.id,
    position: positions[node.id] ?? positionFor(index),
    data: {
      label: (
        <div className={`flow-node-content node-kind-${node.kind}`} aria-label={`${node.name}, ${node.kind}`}>
          {node.ports.filter((port) => port.direction === "input").map((port, portIndex) => <Handle key={`input-${port.name}`} type="target" id={port.name} position={Position.Left} style={{ top: `${35 + portIndex * 18}px` }} aria-label={`${node.name} ${port.name} input`} />)}
          <div className="flow-node-kicker"><span className="node-kind">{node.kind}</span><span className={`node-state ${node.bypass ? "is-bypassed" : node.enabled ? "is-ready" : "is-disabled"}`}>{node.bypass ? "bypass" : node.enabled ? "ready" : "off"}</span></div>
          <div className="flow-node-title"><strong>{node.name}</strong>{canEdit && <button type="button" className="flow-node-delete" aria-label={`Delete ${node.name}`} title={`Delete ${node.name}`} onClick={(event) => { event.stopPropagation(); if (onRemoveNode) onRemoveNode(node.id); else globalThis.dispatchEvent(new CustomEvent("audiorouter:remove-node", { detail: { nodeId: node.id } })); }}>×</button>}</div>
          <NodeVisual node={node} telemetry={telemetry} onSetNodeParameter={onSetNodeParameter} />
          <small className="node-port-count">{node.ports.length} port{node.ports.length === 1 ? "" : "s"} · {node.enabled ? "enabled" : "disabled"}</small>
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
    });
  });

  const edges: FlowEdge[] = session.edges.map((edge) => {
    const sourceActive = telemetrySignalActive(nodeTelemetryFor(session.nodes.find((node) => node.id === edge.sourceNode), diagnostics));
    const destinationActive = telemetrySignalActive(nodeTelemetryFor(session.nodes.find((node) => node.id === edge.destinationNode), diagnostics));
    const active = edge.enabled && diagnostics?.audio.state === "available" && sourceActive && destinationActive;
    return ({
    id: edge.id,
    source: edge.sourceNode,
    target: edge.destinationNode,
    label: `${edge.sourcePort} → ${edge.destinationPort}`,
    deletable: canEdit && onRemoveConnection !== undefined,
    selectable: true,
    className: active ? "flow-edge-active" : undefined,
    animated: active,
    style: { stroke: edge.enabled && highlightedNodeIds.has(edge.sourceNode) && highlightedNodeIds.has(edge.destinationNode) ? "#f5a524" : "#65748a", strokeWidth: edge.enabled && highlightedNodeIds.has(edge.sourceNode) && highlightedNodeIds.has(edge.destinationNode) ? 3 : 1, opacity: !highlightedNodeIds.has(edge.sourceNode) || !highlightedNodeIds.has(edge.destinationNode) ? 0.45 : 1 },
    });
  });

  return (
    <>
    <div className="canvas-layout-actions" aria-label="Canvas layout actions"><span className="muted" role="status" aria-live="polite">{selectedNodeIds.length} node{selectedNodeIds.length === 1 ? "" : "s"} selected</span><button type="button" className="secondary" onClick={tidyLayout}>Tidy layout</button><button type="button" className="secondary" onClick={() => { clearLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey); positionsRef.current = {}; setPositions({}); }}>Reset layout</button></div>
    <div className="session-flow-canvas" aria-label="Signal-flow graph" onDragOver={(event) => { if (!canEdit) return; event.preventDefault(); event.dataTransfer.dropEffect = "copy"; }} onDrop={(event) => { const kind = readLibraryDropKind(event.dataTransfer); event.preventDefault(); const bounds = event.currentTarget.getBoundingClientRect(); const position = libraryDropPosition(event.clientX, event.clientY, bounds); if (isLibraryNodeKind(kind)) { if (onAddLibraryNode) addLibraryNode(kind, position); else routeLibraryDrop(kind, position); } else if (isVirtualBusKind(kind)) { if (onAddVirtualBusNode) addVirtualBusNode(kind === "virtualRenderSource" ? "renderSource" : "captureSink", position); else routeLibraryDrop(kind, position); } }}>
      <div className="canvas-library" aria-label="Drag processors to canvas"><strong>Drag or select to add</strong>{libraryEntries.filter((entry) => entry.kind !== undefined || entry.virtualKind !== undefined).map((entry) => { const kind = entry.kind ?? entry.virtualKind!; const unavailable = Boolean(entry.unavailableReason); return <button type="button" key={`drag-${entry.id}`} draggable={canEdit && !unavailable} disabled={!canEdit || unavailable} title={entry.unavailableReason} onClick={() => { if (unavailable) return; if (entry.kind) addLibraryNode(entry.kind, positionFor(session.nodes.length)); else if (onAddVirtualBusNode) addVirtualBusNode(entry.virtualKind === "virtualRenderSource" ? "renderSource" : "captureSink", positionFor(session.nodes.length)); else routeLibraryDrop(entry.virtualKind!, positionFor(session.nodes.length)); }} onDragStart={(event) => { if (!canEdit || unavailable) return; event.dataTransfer.effectAllowed = "copy"; event.dataTransfer.setData(LIBRARY_DROP_MIME, kind); event.dataTransfer.setData(LIBRARY_DROP_TEXT_MIME, kind); }}>{entry.label}</button>; })}</div>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        fitView
        onInit={(instance) => { flowInstanceRef.current = instance; if (session.nodes.length > 0 && !initialFitDoneRef.current) { initialFitDoneRef.current = true; globalThis.requestAnimationFrame(() => instance.fitView({ padding: 0.2 })); } }}
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
      </ReactFlow>
    </div>
    </>
  );
}
