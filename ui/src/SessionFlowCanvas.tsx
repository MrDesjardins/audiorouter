import {
  Background,
  BaseEdge,
  Controls,
  EdgeLabelRenderer,
  getBezierPath,
  Handle,
  Position,
  ReactFlow,
  useInternalNode,
  type Connection,
  type Edge as FlowEdge,
  type EdgeProps,
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
import { PROCESSOR_ACTIONS } from "./DraftConnectionList";

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
  onToggleConnection?: (edgeId: string, enabled: boolean) => void;
  onInsertProcessor?: (edgeId: string, kind: import("./draft").InsertableProcessorKind) => void;
  onRemoveNode?: (nodeId: string) => void;
  onAddLibraryNode?: (kind: LibraryNodeKind, position: { x: number; y: number }) => string | void;
  onAddVirtualBusNode?: (direction: "renderSource" | "captureSink", position: { x: number; y: number }) => string | void;
  diagnostics?: DiagnosticsSnapshot | null;
  onSetNodeParameter?: (nodeId: string, name: string, value: boolean | number | string) => void;
  canEdit?: boolean;
};

type EdgeSide = "left" | "right" | "top" | "bottom";
const EDGE_SIDES: EdgeSide[] = ["left", "right", "top", "bottom"];
const edgeSidePosition = (side: EdgeSide) => side === "left" ? Position.Left : side === "right" ? Position.Right : side === "top" ? Position.Top : Position.Bottom;

function edgePoint(node: ReturnType<typeof useInternalNode>, side: EdgeSide, fallbackX: number, fallbackY: number) {
  if (!node) return { x: fallbackX, y: fallbackY };
  const width = node.measured.width ?? 218;
  const height = node.measured.height ?? 150;
  const origin = node.internals.positionAbsolute;
  if (side === "left") return { x: origin.x, y: origin.y + height / 2 };
  if (side === "right") return { x: origin.x + width, y: origin.y + height / 2 };
  if (side === "top") return { x: origin.x + width / 2, y: origin.y };
  return { x: origin.x + width / 2, y: origin.y + height };
}

function edgeHandleId(portName: string, side: EdgeSide) {
  return `${portName}__edge_${side}`;
}

function logicalPortHandle(handle: string | null | undefined) {
  if (!handle) return { port: handle ?? null, side: undefined as EdgeSide | undefined };
  const separator = handle.lastIndexOf("__edge_");
  const side = handle.slice(separator + 7) as EdgeSide;
  return separator > 0 && EDGE_SIDES.includes(side) ? { port: handle.slice(0, separator), side } : { port: handle, side: undefined };
}

function logConnectionDebug(event: string, details: Record<string, unknown>) {
  console.info(`[AudioRouter connection] ${event}`, { timestamp: new Date().toISOString(), ...details });
}

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

function AudioEdgeActions({ id, source, target, sourceX, sourceY, targetX, targetY, data, style }: EdgeProps) {
  const [menuOpen, setMenuOpen] = useState(false);
  const sourceNode = useInternalNode(source);
  const targetNode = useInternalNode(target);
  const edgeData = data as { enabled?: boolean; active?: boolean; sourceSide?: EdgeSide; targetSide?: EdgeSide; onSetSide?: (edgeId: string, endpoint: "source" | "target", side: EdgeSide) => void; onToggle?: SessionFlowCanvasProps["onToggleConnection"]; onRemove?: SessionFlowCanvasProps["onRemoveConnection"]; onInsert?: SessionFlowCanvasProps["onInsertProcessor"]; canEdit?: boolean } | undefined;
  const sourceSide = edgeData?.sourceSide ?? "right";
  const targetSide = edgeData?.targetSide ?? "left";
  const sourcePoint = edgePoint(sourceNode, sourceSide, sourceX, sourceY);
  const targetPoint = edgePoint(targetNode, targetSide, targetX, targetY);
  const [path, labelX, labelY] = getBezierPath({ sourceX: sourcePoint.x, sourceY: sourcePoint.y, sourcePosition: edgeSidePosition(sourceSide), targetX: targetPoint.x, targetY: targetPoint.y, targetPosition: edgeSidePosition(targetSide) });
  return <>
    <BaseEdge path={path} style={style} className={edgeData?.active ? "flow-edge-active" : undefined} />
    <EdgeLabelRenderer>
      <div className="edge-action-bar nodrag nopan" style={{ transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)` }} onPointerDown={(event) => event.stopPropagation()}>
        <button type="button" className="edge-action-icon" disabled={!edgeData?.canEdit} aria-label={edgeData?.enabled ? `Disable connection ${id}` : `Enable connection ${id}`} title={edgeData?.enabled ? "Disable connection" : "Enable connection"} onClick={() => edgeData?.onToggle?.(id, !edgeData.enabled)}>{edgeData?.enabled ? "Ⅱ" : "▶"}</button>
        <button type="button" className="edge-action-icon edge-action-remove" disabled={!edgeData?.canEdit} aria-label={`Remove connection ${id}`} title="Remove connection" onClick={() => edgeData?.onRemove?.(id)}>×</button>
        <button type="button" className="edge-action-icon" disabled={!edgeData?.canEdit} aria-label={`Add processor to connection ${id}`} title="Add processor" onClick={() => setMenuOpen((open) => !open)}>＋</button>
        {menuOpen && <div className="edge-insert-menu" role="menu" aria-label={`Configure connection ${id}`}><div className="edge-side-picker"><strong>Source side</strong>{EDGE_SIDES.map((side) => <button key={`source-${side}`} type="button" className={sourceSide === side ? "is-selected" : ""} role="menuitem" onClick={() => edgeData?.onSetSide?.(id, "source", side)}>{side}</button>)}</div><div className="edge-side-picker"><strong>Target side</strong>{EDGE_SIDES.map((side) => <button key={`target-${side}`} type="button" className={targetSide === side ? "is-selected" : ""} role="menuitem" onClick={() => edgeData?.onSetSide?.(id, "target", side)}>{side}</button>)}</div><div className="edge-processor-picker"><strong>Add processor</strong>{PROCESSOR_ACTIONS.map((processor) => <button key={processor.kind} type="button" role="menuitem" onClick={() => { edgeData?.onInsert?.(id, processor.kind); setMenuOpen(false); }}>{processor.label}</button>)}</div></div>}
      </div>
    </EdgeLabelRenderer>
  </>;
}

const edgeTypes = { audioActions: AudioEdgeActions };

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
  if (node.kind === "graphicEq") {
    const frequencies = [31.5, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16_000];
    return frequencies.map((frequencyHz, index) => ({
      index,
      enabled: true,
      frequencyHz,
      gainDb: Number(node.parameters[`band${index}Db`] ?? 0),
      q: 1,
      type: "graphic",
    }));
  }
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
  }).filter((band) => band.enabled);
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
  const [dragBandIndex, setDragBandIndex] = useState<number | null>(null);
  const updateBand = (band: EqBand, event: PointerEvent<SVGSVGElement>) => {
    if (!onSetNodeParameter) return;
    const bounds = event.currentTarget.getBoundingClientRect();
    const x = clamp(event.clientX - bounds.left - 8, 0, 184);
    const y = clamp(event.clientY - bounds.top, 0, 88);
    const gainDb = clamp((44 - y) / 2.45, -12, 12);
    if (node.kind === "graphicEq") {
      onSetNodeParameter(node.id, `band${band.index}Db`, Number(gainDb.toFixed(1)));
      return;
    }
    const frequencyHz = 20 * (10 ** (x / 184 * 3));
    onSetNodeParameter(node.id, `band${band.index}FrequencyHz`, Math.round(frequencyHz));
    onSetNodeParameter(node.id, `band${band.index}GainDb`, Number(gainDb.toFixed(1)));
  };
  return <div className={`node-eq nodrag nopan ${node.kind === "graphicEq" ? "node-eq-graphic" : "node-eq-parametric"}`} aria-label={`${node.kind === "graphicEq" ? "Graphic" : "Parametric"} equalizer response preview`}>
    <svg className="nodrag nopan" viewBox="0 0 200 88" role="img" aria-label="Parametric EQ curve" onPointerMove={(event) => { if (dragBandIndex !== null && bands[dragBandIndex]) updateBand(bands[dragBandIndex], event); }} onPointerUp={(event) => { setDragBandIndex(null); if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId); }} onPointerCancel={(event) => { setDragBandIndex(null); if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId); }}>
      <path className="eq-grid" d="M8 14H192M8 44H192M8 74H192M8 8V80M69 8V80M130 8V80M192 8V80" />
      <path className="eq-curve" d={bands.length ? `M8 44 ${bands.map((band) => `L ${eqX(band.frequencyHz).toFixed(1)} ${eqY(band.gainDb).toFixed(1)}`).join(" ")} L192 44` : "M8 44H192"} />
      {bands.map((band) => <circle key={band.index} className="eq-band nodrag nopan" cx={eqX(band.frequencyHz)} cy={eqY(band.gainDb)} r="4" tabIndex={0} role="slider" aria-valuemin={-12} aria-valuemax={12} aria-valuenow={band.gainDb} aria-label={`Band ${band.index + 1}, ${band.frequencyHz} Hz, ${band.gainDb} dB`} onKeyDown={(event) => { if (!onSetNodeParameter) return; const gainStep = event.key === "ArrowUp" ? 0.5 : event.key === "ArrowDown" ? -0.5 : 0; const frequencyStep = event.key === "ArrowRight" ? 1.08 : event.key === "ArrowLeft" ? 1 / 1.08 : 1; if (gainStep !== 0 || frequencyStep !== 1) { event.preventDefault(); if (gainStep !== 0) onSetNodeParameter(node.id, `band${band.index}GainDb`, Number(clamp(band.gainDb + gainStep, -12, 12).toFixed(1))); if (frequencyStep !== 1) onSetNodeParameter(node.id, `band${band.index}FrequencyHz`, Math.round(clamp(band.frequencyHz * frequencyStep, 20, 20_000))); } }} onPointerDown={(event) => { event.preventDefault(); event.stopPropagation(); setDragBandIndex(bands.findIndex((candidate) => candidate.index === band.index)); event.currentTarget.ownerSVGElement?.setPointerCapture(event.pointerId); }} />)}
    </svg>
    <div className="node-eq-scale"><span>20 Hz</span><span>1 kHz</span><span>20 kHz</span></div>
  </div>;
}

function NodeVisual({ node, telemetry, onSetNodeParameter }: { node: Node; telemetry: ReturnType<typeof nodeTelemetryFor>; onSetNodeParameter?: SessionFlowCanvasProps["onSetNodeParameter"] }) {
  if (node.kind === "parametricEq" || node.kind === "graphicEq") return <MiniEq node={node} onSetNodeParameter={onSetNodeParameter} />;
  if (telemetry?.meter || ["physicalInput", "physicalOutput", "meter", "testSignal", "recorder"].includes(node.kind)) return <MiniMeter telemetry={telemetry} />;
  return <div className="node-activity" aria-label={node.enabled && !node.bypass ? "Processor ready" : "Processor inactive"}><span className="activity-dot" />{node.bypass ? "Bypassed" : node.enabled ? "Ready" : "Disabled"}</div>;
}

export function SessionFlowCanvas({ session, selectedNodeId, selectedNodeIds = [selectedNodeId], onSelect, onSelectMany, onConnect, onRemoveConnection, onToggleConnection, onInsertProcessor, onRemoveNode, onAddLibraryNode, onAddVirtualBusNode, diagnostics, onSetNodeParameter, canEdit = true }: SessionFlowCanvasProps) {
  const layoutKey = `audiorouter.ui.layout.${session.id}`;
  const [positions, setPositions] = useState<LayoutPositions>(() => readLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey));
  const edgeLayoutKey = `${layoutKey}.edges`;
  const [edgeSides, setEdgeSides] = useState<Record<string, { source: EdgeSide; target: EdgeSide }>>(() => {
    if (typeof window === "undefined") return {};
    try { return JSON.parse(window.localStorage.getItem(edgeLayoutKey) ?? "{}") as Record<string, { source: EdgeSide; target: EdgeSide }>; } catch { return {}; }
  });
  const positionsRef = useRef(positions);
  const flowInstanceRef = useRef<ReactFlowInstance | null>(null);
  const initialFitDoneRef = useRef(false);
  useEffect(() => {
    initialFitDoneRef.current = false;
    const next = readLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey);
    positionsRef.current = next;
    setPositions(next);
    if (typeof window !== "undefined") {
      try { setEdgeSides(JSON.parse(window.localStorage.getItem(edgeLayoutKey) ?? "{}") as Record<string, { source: EdgeSide; target: EdgeSide }>); } catch { setEdgeSides({}); }
    }
  }, [layoutKey, edgeLayoutKey]);
  const setEdgeSide = (edgeId: string, endpoint: "source" | "target", side: EdgeSide) => {
    setEdgeSides((currentMap) => {
      const current = currentMap[edgeId] ?? { source: "right" as EdgeSide, target: "left" as EdgeSide };
      const next = { ...currentMap, [edgeId]: { ...current, [endpoint]: side } };
      if (typeof window !== "undefined") window.localStorage.setItem(edgeLayoutKey, JSON.stringify(next));
      return next;
    });
  };
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
          {node.ports.filter((port) => port.direction === "input").map((port, portIndex, ports) => { const offset = `${((portIndex + 1) / (ports.length + 1)) * 100}%`; return EDGE_SIDES.map((side) => <Handle key={`input-${port.name}-${side}`} type="target" id={edgeHandleId(port.name, side)} position={edgeSidePosition(side)} style={side === "left" || side === "right" ? { top: offset } : { left: offset }} aria-label={side === "left" ? `${node.name} ${port.name} input` : `${node.name} ${port.name} input ${side} connector`} data-debug-side={side} onPointerDown={(event) => logConnectionDebug("handle-pointer-down", { nodeId: node.id, nodeName: node.name, direction: "target", port: port.name, side, handleId: edgeHandleId(port.name, side), clientX: event.clientX, clientY: event.clientY, button: event.button })} />); })}
          <div className="flow-node-kicker"><span className="node-kind">{node.kind}</span><span className={`node-state ${node.bypass ? "is-bypassed" : node.enabled ? "is-ready" : "is-disabled"}`}>{node.bypass ? "bypass" : node.enabled ? "ready" : "off"}</span></div>
          <div className="flow-node-title"><strong>{node.name}</strong>{canEdit && <button type="button" className="flow-node-delete" aria-label={`Delete ${node.name}`} title={`Delete ${node.name}`} onClick={(event) => { event.stopPropagation(); if (onRemoveNode) onRemoveNode(node.id); else globalThis.dispatchEvent(new CustomEvent("audiorouter:remove-node", { detail: { nodeId: node.id } })); }}>×</button>}</div>
          <NodeVisual node={node} telemetry={telemetry} onSetNodeParameter={onSetNodeParameter} />
          <small className="node-port-count">{node.ports.length} port{node.ports.length === 1 ? "" : "s"} · {node.enabled ? "enabled" : "disabled"}</small>
          <span className="flow-port-list">{nodePortLabels(node).map((port) => <small key={port}>{port}</small>)}</span>
          {node.ports.filter((port) => port.direction === "output").map((port, portIndex, ports) => { const offset = `${((portIndex + 1) / (ports.length + 1)) * 100}%`; return EDGE_SIDES.map((side) => <Handle key={`output-${port.name}-${side}`} type="source" id={edgeHandleId(port.name, side)} position={edgeSidePosition(side)} style={side === "left" || side === "right" ? { top: offset } : { left: offset }} aria-label={side === "right" ? `${node.name} ${port.name} output` : `${node.name} ${port.name} output ${side} connector`} data-debug-side={side} onPointerDown={(event) => logConnectionDebug("handle-pointer-down", { nodeId: node.id, nodeName: node.name, direction: "source", port: port.name, side, handleId: edgeHandleId(port.name, side), clientX: event.clientX, clientY: event.clientY, button: event.button })} />); })}
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
    type: "audioActions",
    data: { enabled: edge.enabled, active, sourceSide: edgeSides[edge.id]?.source, targetSide: edgeSides[edge.id]?.target, onSetSide: setEdgeSide, onToggle: onToggleConnection, onRemove: onRemoveConnection, onInsert: onInsertProcessor, canEdit },
    deletable: canEdit && onRemoveConnection !== undefined,
    selectable: true,
    className: active ? "flow-edge-active" : undefined,
    animated: active,
    style: { stroke: edge.enabled && highlightedNodeIds.has(edge.sourceNode) && highlightedNodeIds.has(edge.destinationNode) ? "#f5a524" : "#65748a", strokeWidth: edge.enabled && highlightedNodeIds.has(edge.sourceNode) && highlightedNodeIds.has(edge.destinationNode) ? 3 : 1, opacity: !highlightedNodeIds.has(edge.sourceNode) || !highlightedNodeIds.has(edge.destinationNode) ? 0.45 : 1 },
    });
  });

  return (
    <>
    <div className="canvas-layout-actions" aria-label="Canvas layout actions"><span className="muted" role="status" aria-live="polite">{selectedNodeIds.length} node{selectedNodeIds.length === 1 ? "" : "s"} selected</span><button type="button" className="secondary" onClick={tidyLayout}>Tidy layout</button><button type="button" className="secondary" onClick={() => { clearLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey); if (typeof window !== "undefined") window.localStorage.removeItem(edgeLayoutKey); positionsRef.current = {}; setEdgeSides({}); setPositions({}); }}>Reset layout</button></div>
    <div className="session-flow-canvas" aria-label="Signal-flow graph" onDragOver={(event) => { if (!canEdit) return; event.preventDefault(); event.dataTransfer.dropEffect = "copy"; }} onDrop={(event) => { const kind = readLibraryDropKind(event.dataTransfer); event.preventDefault(); const bounds = event.currentTarget.getBoundingClientRect(); const position = libraryDropPosition(event.clientX, event.clientY, bounds); if (isLibraryNodeKind(kind)) { if (onAddLibraryNode) addLibraryNode(kind, position); else routeLibraryDrop(kind, position); } else if (isVirtualBusKind(kind)) { if (onAddVirtualBusNode) addVirtualBusNode(kind === "virtualRenderSource" ? "renderSource" : "captureSink", position); else routeLibraryDrop(kind, position); } }}>
        <div className="canvas-library" aria-label="Drag processors to canvas"><strong>Drag or select to add</strong>{libraryEntries.filter((entry) => entry.kind !== undefined || entry.virtualKind !== undefined).sort((left, right) => left.label.localeCompare(right.label)).map((entry) => { const kind = entry.kind ?? entry.virtualKind!; const unavailable = Boolean(entry.unavailableReason); return <button type="button" key={`drag-${entry.id}`} draggable={canEdit && !unavailable} disabled={!canEdit || unavailable} title={entry.unavailableReason} onClick={() => { if (unavailable) return; if (entry.kind) addLibraryNode(entry.kind, positionFor(session.nodes.length)); else if (onAddVirtualBusNode) addVirtualBusNode(entry.virtualKind === "virtualRenderSource" ? "renderSource" : "captureSink", positionFor(session.nodes.length)); else routeLibraryDrop(entry.virtualKind!, positionFor(session.nodes.length)); }} onDragStart={(event) => { if (!canEdit || unavailable) return; event.dataTransfer.effectAllowed = "copy"; event.dataTransfer.setData(LIBRARY_DROP_MIME, kind); event.dataTransfer.setData(LIBRARY_DROP_TEXT_MIME, kind); }}>{entry.label}</button>; })}</div>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onInit={(instance) => { flowInstanceRef.current = instance; if (session.nodes.length > 0 && !initialFitDoneRef.current) { initialFitDoneRef.current = true; globalThis.requestAnimationFrame(() => instance.fitView({ padding: 0.2 })); } }}
        nodesConnectable={canEdit}
        nodesDraggable
        selectionOnDrag
        onSelectionChange={({ nodes: selectedNodes }) => onSelectMany?.(selectedNodes.map((node) => node.id))}
        onNodeClick={(_, node) => onSelect(node.id)}
        onConnectStart={(_, params) => logConnectionDebug("react-flow-connect-start", { handleType: params.handleType, nodeId: params.nodeId, handleId: params.handleId })}
        onConnectEnd={(_, connectionState) => logConnectionDebug("react-flow-connect-end", { inProgress: "inProgress" in connectionState ? connectionState.inProgress : false, fromNode: connectionState.fromNode?.id, fromHandle: connectionState.fromHandle?.id, toNode: connectionState.toNode?.id, toHandle: connectionState.toHandle?.id, toPosition: connectionState.toPosition })}
        onConnect={canEdit ? (connection) => { logConnectionDebug("react-flow-connect", { source: connection.source, sourceHandle: connection.sourceHandle, target: connection.target, targetHandle: connection.targetHandle }); const source = logicalPortHandle(connection.sourceHandle); const target = logicalPortHandle(connection.targetHandle); const normalized = { ...connection, sourceHandle: source.port, targetHandle: target.port }; const edgeId = onConnect(normalized); logConnectionDebug("draft-connect-result", { edgeId, normalizedSourceHandle: normalized.sourceHandle, normalizedTargetHandle: normalized.targetHandle, sourceSide: source.side, targetSide: target.side }); if (edgeId) { if (source.side) setEdgeSide(edgeId, "source", source.side); if (target.side) setEdgeSide(edgeId, "target", target.side); } } : undefined}
        edgeTypes={edgeTypes}
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
