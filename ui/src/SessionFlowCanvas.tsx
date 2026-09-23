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
  type NodeProps,
  type ReactFlowInstance,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useEffect, useRef, useState, type MouseEvent, type PointerEvent, type ReactNode } from "react";
import type { DiagnosticsSnapshot, Node, NodeKind, Session } from "@audiorouter/contracts";
import { clearLayout, readLayout, writeLayout, type LayoutPositions } from "./layout";
import { nodePortLabels, relatedNodeIds } from "./graphView";
import { libraryEntries, type LibraryEntry, type LibraryFlowGroup } from "./library";
import { GAIN_MAX_DB, GAIN_MIN_DB, type LibraryNodeKind } from "./draft";
import { PROCESSOR_ACTIONS } from "./DraftConnectionList";
import type { RecorderStatus } from "./backend";

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
  recorderStatuses?: RecorderStatus[];
  onSetNodeParameter?: (nodeId: string, name: string, value: boolean | number | string) => void;
  onOpenPluginPicker?: (edgeId?: string) => void;
  onOpenApplicationPicker?: () => void;
  onConnectionRejected?: (message: string) => void;
  canEdit?: boolean;
};

// Every node below renders its own complete set of Handle components (one
// per port per side, via EDGE_SIDES) inside data.label. Without an explicit
// custom node type, React Flow falls back to its built-in "default" node
// type, which *also* renders its own implicit target/source Handle at
// Top/Bottom in addition to whatever data.label contains — producing a
// visible extra handle on the top and bottom edges only (matching a
// user-reported "3 dots on top/bottom, 2 on left/right" screenshot). A
// bare pass-through component registered as the node type renders only
// data.label, with no implicit handles of its own. Defined at module scope
// so its identity is stable across renders; recreating this object on
// every render is a documented React Flow foot-gun that forces internal
// node type remounts.
function FlowNodeRenderer({ data }: NodeProps) {
  return <>{(data as { label?: ReactNode }).label}</>;
}
const NODE_TYPES = { flowNode: FlowNodeRenderer };

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

const NODE_FLOW_GROUPS: Record<NodeKind, LibraryFlowGroup> = {
  physicalInput: "input",
  testSignal: "input",
  applicationCapture: "input",
  endpointLoopback: "input",
  virtualRenderSource: "input",
  physicalOutput: "output",
  virtualCaptureSink: "output",
  mixer: "tool",
  gain: "tool",
  mute: "tool",
  meter: "tool",
  parametricEq: "tool",
  compressor: "tool",
  gate: "tool",
  limiter: "tool",
  delay: "tool",
  graphicEq: "tool",
  pitch: "tool",
  recorder: "tool",
  plugin: "tool",
};

/** Where a node sits in the source -> tool -> destination signal-flow model,
 * used for the node card's accent color and the drag shelf's grouping. */
function nodeFlowGroup(kind: NodeKind): LibraryFlowGroup {
  return NODE_FLOW_GROUPS[kind] ?? "tool";
}

const FLOW_GROUPS: LibraryFlowGroup[] = ["input", "tool", "output"];
const FLOW_GROUP_LABELS: Record<LibraryFlowGroup, string> = { input: "Inputs", tool: "Tools", output: "Outputs" };

type NodeKindFamily = "input" | "native" | "vst" | "output";
const NODE_KIND_FAMILY_LABELS: Record<NodeKindFamily, string> = { input: "Input", native: "Native", vst: "VST", output: "Output" };

/** Beyond input/tool/output signal-flow position, a "tool" is either a
 * built-in native processor or an isolated-worker VST plugin; those are
 * visually and operationally very different (a VST loads user-supplied code
 * from disk in its own process), so this is called out as its own family
 * rather than folded into the generic "tool" label. */
function nodeKindFamily(kind: NodeKind): NodeKindFamily {
  const group = nodeFlowGroup(kind);
  if (group === "input") return "input";
  if (group === "output") return "output";
  return kind === "plugin" ? "vst" : "native";
}

const NODE_KIND_LABELS: Partial<Record<NodeKind, string>> = {
  physicalInput: "Physical Input",
  physicalOutput: "Physical Output",
  applicationCapture: "App Capture",
  endpointLoopback: "Endpoint Loopback",
  virtualRenderSource: "Virtual Source",
  virtualCaptureSink: "Virtual Sink",
  testSignal: "Test Signal",
  parametricEq: "Parametric EQ",
  graphicEq: "Graphic EQ",
};

/** A readable node-kind label for the canvas card. A plugin's label names
 * its exact loaded format (VST2/VST3) instead of the generic "Plugin", since
 * that is the single most useful fact distinguishing it from a native tool. */
function humanizeNodeKind(node: Node): string {
  if (node.kind === "plugin") {
    const format = node.parameters.format;
    return format === "vst3" ? "VST3 Plugin" : format === "vst2" ? "VST2 Plugin" : "VST Plugin";
  }
  const known = NODE_KIND_LABELS[node.kind];
  if (known) return known;
  return node.kind.replace(/([a-z0-9])([A-Z])/g, "$1 $2").replace(/^./, (char) => char.toUpperCase());
}

function edgeHandleId(portName: string, side: EdgeSide) {
  return `${portName}__edge_${side}`;
}

/**
 * Slots a port among ALL of a node's ports (not just same-direction ones) so
 * an input and an output never land on the same offset along a side. Two
 * ports at the same offset would stack their four side handles exactly on
 * top of each other, and the later-rendered (output) handle always wins hit
 * testing, making the other port's handles ungrabbable on that side.
 */
function portOffset(node: Node, port: Node["ports"][number]) {
  const index = node.ports.indexOf(port);
  return `${((index + 1) / (node.ports.length + 1)) * 100}%`;
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
  const edgeData = data as { enabled?: boolean; active?: boolean; sourceSide?: EdgeSide; targetSide?: EdgeSide; onSetSide?: (edgeId: string, endpoint: "source" | "target", side: EdgeSide) => void; onToggle?: SessionFlowCanvasProps["onToggleConnection"]; onRemove?: SessionFlowCanvasProps["onRemoveConnection"]; onInsert?: SessionFlowCanvasProps["onInsertProcessor"]; onOpenPluginPicker?: SessionFlowCanvasProps["onOpenPluginPicker"]; canEdit?: boolean } | undefined;
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
        {menuOpen && <div className="edge-insert-menu" role="menu" aria-label={`Configure connection ${id}`}><div className="edge-side-picker"><strong>Source side</strong>{EDGE_SIDES.map((side) => <button key={`source-${side}`} type="button" className={sourceSide === side ? "is-selected" : ""} role="menuitem" onClick={() => edgeData?.onSetSide?.(id, "source", side)}>{side}</button>)}</div><div className="edge-side-picker"><strong>Target side</strong>{EDGE_SIDES.map((side) => <button key={`target-${side}`} type="button" className={targetSide === side ? "is-selected" : ""} role="menuitem" onClick={() => edgeData?.onSetSide?.(id, "target", side)}>{side}</button>)}</div><div className="edge-processor-picker"><strong>Add processor</strong>{PROCESSOR_ACTIONS.map((processor) => <button key={processor.kind} type="button" role="menuitem" onClick={() => { edgeData?.onInsert?.(id, processor.kind); setMenuOpen(false); }}>{processor.label}</button>)}<button type="button" role="menuitem" onClick={() => { edgeData?.onOpenPluginPicker?.(id); setMenuOpen(false); }}>VST plugin…</button></div></div>}
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

function trySetPointerCapture(element: Element, pointerId: number) {
  try { (element as Element & { setPointerCapture?: (id: number) => void }).setPointerCapture?.(pointerId); } catch { /* unsupported in this environment */ }
}

function tryReleasePointerCapture(element: Element, pointerId: number) {
  try {
    const withCapture = element as Element & { hasPointerCapture?: (id: number) => boolean; releasePointerCapture?: (id: number) => void };
    if (withCapture.hasPointerCapture?.(pointerId)) withCapture.releasePointerCapture?.(pointerId);
  } catch { /* unsupported in this environment */ }
}

/** Compact horizontal slider used inline on a canvas node for a single numeric
 * parameter (gain, delay time, pitch shift). Read-only when onChange is absent. */
function MiniFader({ label, value, min, max, step, formatValue, onChange, ariaLabel }: { label: string; value: number; min: number; max: number; step: number; formatValue: (value: number) => string; onChange?: (value: number) => void; ariaLabel: string }) {
  const percent = clamp(((value - min) / (max - min)) * 100, 0, 100);
  const setFromClientX = (clientX: number, bounds: DOMRect) => {
    if (!onChange) return;
    const ratio = clamp((clientX - bounds.left) / bounds.width, 0, 1);
    const raw = min + ratio * (max - min);
    onChange(clamp(Math.round(raw / step) * step, min, max));
  };
  return <div className="node-fader nodrag nopan">
    <div className="node-fader-track" role="slider" tabIndex={onChange ? 0 : -1} aria-label={ariaLabel} aria-valuemin={min} aria-valuemax={max} aria-valuenow={value} aria-valuetext={formatValue(value)}
      onPointerDown={(event) => { if (!onChange) return; event.preventDefault(); event.stopPropagation(); setFromClientX(event.clientX, event.currentTarget.getBoundingClientRect()); trySetPointerCapture(event.currentTarget, event.pointerId); }}
      onPointerMove={(event) => { if (!onChange || event.buttons !== 1) return; setFromClientX(event.clientX, event.currentTarget.getBoundingClientRect()); }}
      onPointerUp={(event) => tryReleasePointerCapture(event.currentTarget, event.pointerId)}
      onKeyDown={(event) => {
        if (!onChange) return;
        if (event.key === "ArrowRight" || event.key === "ArrowUp") { event.preventDefault(); onChange(clamp(value + step, min, max)); }
        else if (event.key === "ArrowLeft" || event.key === "ArrowDown") { event.preventDefault(); onChange(clamp(value - step, min, max)); }
        else if (event.key === "Home") { event.preventDefault(); onChange(min); }
        else if (event.key === "End") { event.preventDefault(); onChange(max); }
      }}
    >
      <div className="node-fader-fill" style={{ width: `${percent}%` }} />
    </div>
    <div className="node-fader-readout"><span>{label}</span><strong>{formatValue(value)}</strong></div>
  </div>;
}

/** Gain-reduction meter for dynamics processors (compressor/limiter), driven
 * by the backend's per-channel gainReductionDb telemetry when available. */
function GainReductionMeter({ telemetry, detail }: { telemetry: ReturnType<typeof nodeTelemetryFor>; detail: string }) {
  const values = telemetry?.processor?.gainReductionDb ?? null;
  const grDb = values && values.length > 0 ? Math.max(...values) : null;
  const percent = grDb === null ? 0 : clamp((grDb / 24) * 100, 0, 100);
  return <div className="node-gr-meter" aria-label={grDb === null ? "Gain reduction not available" : `Gain reduction ${grDb.toFixed(1)} dB`}>
    <div className="node-gr-track"><span className="node-gr-fill" style={{ height: `${percent}%` }} /></div>
    <div className="node-gr-readout"><span>GR</span><strong>{grDb === null ? "—" : `${grDb.toFixed(1)} dB`}</strong></div>
    <small className="node-gr-detail">{detail}</small>
  </div>;
}

/** Open/closed indicator for a gate node, driven by the backend's per-channel
 * gateOpen telemetry when available. */
function GateVisual({ telemetry, thresholdDb }: { telemetry: ReturnType<typeof nodeTelemetryFor>; thresholdDb: number }) {
  const states = telemetry?.processor?.gateOpen ?? null;
  const open = states && states.length > 0 ? states.some(Boolean) : null;
  return <div className={`node-gate ${open === true ? "is-open" : open === false ? "is-closed" : "is-unknown"}`} aria-label={open === null ? "Gate state not available" : open ? "Gate open" : "Gate closed"}>
    <span className="node-gate-dot" />
    <strong>{open === null ? "Gate" : open ? "Open" : "Closed"}</strong>
    <small>Threshold {thresholdDb} dB</small>
  </div>;
}

/** One-tap mute toggle so the most common live action on a Mute node does not
 * require opening the inspector. */
function MuteToggle({ node, onSetNodeParameter }: { node: Node; onSetNodeParameter?: SessionFlowCanvasProps["onSetNodeParameter"] }) {
  const muted = Boolean(node.parameters.muted);
  return <button type="button" className={`node-mute-toggle nodrag nopan ${muted ? "is-muted" : "is-live"}`} disabled={!onSetNodeParameter} aria-pressed={muted} aria-label={muted ? `${node.name}, muted, click to unmute` : `${node.name}, live, click to mute`} onClick={(event) => { event.stopPropagation(); onSetNodeParameter?.(node.id, "muted", !muted); }}>
    <span className="node-mute-dot" />{muted ? "Muted" : "Live"}
  </button>;
}

/** Merge glyph for a Mixer node plus a live count of connected inputs, since a
 * static "sums connected inputs" label never actually reflected the graph. */
function MixerVisual({ inputCount, telemetry }: { inputCount: number; telemetry: ReturnType<typeof nodeTelemetryFor> }) {
  return <div className="node-mixer-wrap">
    <div className="node-mixer" aria-label={inputCount === 0 ? "Mixer has no connected inputs yet" : `Mixer sums ${inputCount} connected input${inputCount === 1 ? "" : "s"}`}>
      <span className="node-mixer-icon" aria-hidden="true">Σ</span>
      <span>{inputCount === 0 ? "No inputs connected" : `Summing ${inputCount} input${inputCount === 1 ? "" : "s"}`}</span>
    </div>
    {telemetry?.meter && <MiniMeter telemetry={telemetry} />}
  </div>;
}

/** Test Signal shows its configured tone alongside the ordinary output meter
 * so the source's shape is visible without opening the inspector. */
function TestSignalVisual({ node, telemetry }: { node: Node; telemetry: ReturnType<typeof nodeTelemetryFor> }) {
  const frequencyHz = Number(node.parameters.frequencyHz ?? 440);
  const levelDb = Number(node.parameters.levelDb ?? -18);
  return <div className="node-test-signal">
    <div className="node-test-signal-config"><span>{frequencyHz} Hz</span><span>{levelDb.toFixed(1)} dB</span></div>
    <MiniMeter telemetry={telemetry} />
  </div>;
}

const RECORDER_STATE_LABEL: Record<RecorderStatus["state"], string> = {
  idle: "Idle",
  armed: "Armed",
  recording: "Recording",
  paused: "Paused",
  stopping: "Stopping",
  completed: "Completed",
  failed: "Failed",
};

/** A Recorder node's most important fact is whether it is actually recording
 * right now, which a generic pass-through meter never showed. */
function RecorderVisual({ status, telemetry }: { status: RecorderStatus | null | undefined; telemetry: ReturnType<typeof nodeTelemetryFor> }) {
  const state = status?.state ?? null;
  return <div className="node-recorder">
    <div className={`node-recorder-state ${state ? `is-${state}` : "is-unknown"}`} aria-label={state ? `Recorder ${RECORDER_STATE_LABEL[state]}` : "No recorder session for this node yet"}>
      <span className="node-recorder-dot" />
      <strong>{state ? RECORDER_STATE_LABEL[state] : "No session"}</strong>
    </div>
    <MiniMeter telemetry={telemetry} />
  </div>;
}

/** Basename of a filesystem path, tolerant of both `/` and `\` separators. */
function basename(path: string) {
  const normalized = path.replace(/\\/g, "/");
  const parts = normalized.split("/");
  return parts[parts.length - 1] || path;
}

/** A captured application's identity is the whole point of this node; a
 * generic Ready chip gave no way to tell captures apart at a glance. */
function ApplicationCaptureVisual({ node, telemetry }: { node: Node; telemetry: ReturnType<typeof nodeTelemetryFor> }) {
  const executable = String(node.parameters.executable ?? "Unknown application");
  const policy = node.parameters.processPolicy === "selectedInstance" ? "This running instance" : "Any matching instance";
  return <div className="node-app-capture-wrap">
    <div className="node-app-capture" title={executable}><strong>{executable}</strong><small>{policy}</small></div>
    {telemetry?.meter && <MiniMeter telemetry={telemetry} />}
  </div>;
}

/** Shows which exact render endpoint this loopback source is bound to. */
function EndpointLoopbackVisual({ node, telemetry }: { node: Node; telemetry: ReturnType<typeof nodeTelemetryFor> }) {
  const endpointId = String(node.parameters.endpointId ?? "");
  const shortId = endpointId.length > 22 ? `${endpointId.slice(0, 10)}…${endpointId.slice(-8)}` : endpointId || "No endpoint bound";
  return <div className="node-endpoint-loopback-wrap">
    <div className="node-endpoint-loopback" title={endpointId}><span className="node-endpoint-loopback-icon" aria-hidden="true">↺</span><span>{shortId}</span></div>
    {telemetry?.meter && <MiniMeter telemetry={telemetry} />}
  </div>;
}

/** Deferred virtual-bus source/sink still shows which existing bus identity
 * it targets, even while managed provisioning remains unavailable. */
function VirtualBusVisual({ node, telemetry }: { node: Node; telemetry: ReturnType<typeof nodeTelemetryFor> }) {
  const busId = String(node.parameters.busId ?? "Unbound");
  const isSource = node.kind === "virtualRenderSource";
  return <div className="node-virtual-bus-wrap">
    <div className="node-virtual-bus" title={busId}><span className="node-virtual-bus-icon" aria-hidden="true">{isSource ? "↦" : "↤"}</span><span>Bus {busId}</span></div>
    {telemetry?.meter && <MiniMeter telemetry={telemetry} />}
  </div>;
}

/** A plugin node's format and binary identity matter more than a generic
 * Ready chip once a session has more than one third-party plugin. */
function PluginVisual({ node, telemetry }: { node: Node; telemetry: ReturnType<typeof nodeTelemetryFor> }) {
  const format = node.parameters.format;
  const formatLabel = format === "vst3" ? "VST3" : format === "vst2" ? "VST2" : "Plugin";
  const fileName = basename(String(node.parameters.path ?? ""));
  const health = telemetry?.plugin ?? null;
  const healthAlert = health?.state === "failed" || health?.state === "quarantined";
  return <div className="node-plugin">
    <div className="node-plugin-header"><span className="node-plugin-format">{formatLabel}</span><span className={`node-state ${node.bypass ? "is-bypassed" : node.enabled ? "is-ready" : "is-disabled"}`}>{node.bypass ? "bypass" : node.enabled ? "active" : "stopped"}</span></div>
    <div className="node-plugin-file" title={fileName}>{fileName}</div>
    {healthAlert && <div className="node-plugin-health-alert" role="alert">{health.state === "quarantined" ? `Quarantined (${health.failureCount} failures)` : "Worker failed"}</div>}
    {telemetry?.meter && <MiniMeter telemetry={telemetry} />}
  </div>;
}

function NodeVisual({ node, telemetry, onSetNodeParameter, recorderStatus, mixerInputCount }: { node: Node; telemetry: ReturnType<typeof nodeTelemetryFor>; onSetNodeParameter?: SessionFlowCanvasProps["onSetNodeParameter"]; recorderStatus?: RecorderStatus | null; mixerInputCount?: number }) {
  if (node.kind === "parametricEq" || node.kind === "graphicEq") return <MiniEq node={node} onSetNodeParameter={onSetNodeParameter} />;
  if (node.kind === "gain") return <MiniFader label="Gain" value={Number(node.parameters.gainDb ?? 0)} min={GAIN_MIN_DB} max={GAIN_MAX_DB} step={1} formatValue={(value) => `${value.toFixed(1)} dB`} onChange={onSetNodeParameter ? (value) => onSetNodeParameter(node.id, "gainDb", Number(value.toFixed(1))) : undefined} ariaLabel={`${node.name} gain`} />;
  if (node.kind === "pitch") return <MiniFader label="Pitch" value={Number(node.parameters.semitones ?? 0)} min={-24} max={24} step={0.5} formatValue={(value) => `${value > 0 ? "+" : ""}${value.toFixed(1)} st`} onChange={onSetNodeParameter ? (value) => onSetNodeParameter(node.id, "semitones", Number(value.toFixed(1))) : undefined} ariaLabel={`${node.name} pitch shift`} />;
  if (node.kind === "delay") return <MiniFader label="Delay" value={Number(node.parameters.delayMs ?? 0)} min={0} max={2000} step={10} formatValue={(value) => `${Math.round(value)} ms`} onChange={onSetNodeParameter ? (value) => onSetNodeParameter(node.id, "delayMs", Math.round(value)) : undefined} ariaLabel={`${node.name} delay time`} />;
  if (node.kind === "mute") return <MuteToggle node={node} onSetNodeParameter={onSetNodeParameter} />;
  if (node.kind === "compressor") return <GainReductionMeter telemetry={telemetry} detail={`Threshold ${node.parameters.thresholdDb} dB · Ratio ${node.parameters.ratio}:1`} />;
  if (node.kind === "limiter") return <GainReductionMeter telemetry={telemetry} detail={`Ceiling ${node.parameters.ceilingDb} dB`} />;
  if (node.kind === "gate") return <GateVisual telemetry={telemetry} thresholdDb={Number(node.parameters.thresholdDb ?? 0)} />;
  if (node.kind === "mixer") return <MixerVisual inputCount={mixerInputCount ?? 0} telemetry={telemetry} />;
  if (node.kind === "testSignal") return <TestSignalVisual node={node} telemetry={telemetry} />;
  if (node.kind === "recorder") return <RecorderVisual status={recorderStatus} telemetry={telemetry} />;
  if (node.kind === "applicationCapture") return <ApplicationCaptureVisual node={node} telemetry={telemetry} />;
  if (node.kind === "endpointLoopback") return <EndpointLoopbackVisual node={node} telemetry={telemetry} />;
  if (node.kind === "virtualRenderSource" || node.kind === "virtualCaptureSink") return <VirtualBusVisual node={node} telemetry={telemetry} />;
  if (node.kind === "plugin") return <PluginVisual node={node} telemetry={telemetry} />;
  if (telemetry?.meter || ["physicalInput", "physicalOutput", "meter"].includes(node.kind)) return <MiniMeter telemetry={telemetry} />;
  return <div className="node-activity" aria-label={node.enabled && !node.bypass ? "Processor ready" : "Processor inactive"}><span className="activity-dot" />{node.bypass ? "Bypassed" : node.enabled ? "Ready" : "Disabled"}</div>;
}

export function SessionFlowCanvas({ session, selectedNodeId, selectedNodeIds = [selectedNodeId], onSelect, onSelectMany, onConnect, onRemoveConnection, onToggleConnection, onInsertProcessor, onRemoveNode, onAddLibraryNode, onAddVirtualBusNode, diagnostics, recorderStatuses = [], onSetNodeParameter, onOpenPluginPicker, onOpenApplicationPicker, onConnectionRejected, canEdit = true }: SessionFlowCanvasProps) {
  const layoutKey = `audiorouter.ui.layout.${session.id}`;
  const [positions, setPositions] = useState<LayoutPositions>(() => readLayout(typeof window === "undefined" ? null : window.localStorage, layoutKey));
  const edgeLayoutKey = `${layoutKey}.edges`;
  const [edgeSides, setEdgeSides] = useState<Record<string, { source: EdgeSide; target: EdgeSide }>>(() => {
    if (typeof window === "undefined") return {};
    try { return JSON.parse(window.localStorage.getItem(edgeLayoutKey) ?? "{}") as Record<string, { source: EdgeSide; target: EdgeSide }>; } catch { return {}; }
  });
  const positionsRef = useRef(positions);
  const sourceHandleRef = useRef<{ nodeId: string; handleId: string; side: EdgeSide } | null>(null);
  const targetHandleRef = useRef<{ nodeId: string; handleId: string; side: EdgeSide } | null>(null);
  const pendingConnectionRef = useRef<{ connection: Connection; sourceSide?: EdgeSide; targetSide?: EdgeSide; rawTargetHandle?: string | null } | null>(null);
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
  const libraryButton = (entry: LibraryEntry) => {
    const kind = entry.kind ?? entry.virtualKind!;
    const unavailable = Boolean(entry.unavailableReason);
    return <button type="button" key={`drag-${entry.id}`} className="canvas-library-item" draggable={canEdit && !unavailable} disabled={!canEdit || unavailable} title={entry.unavailableReason ?? entry.note} onClick={() => { if (unavailable) return; if (entry.kind) addLibraryNode(entry.kind, positionFor(session.nodes.length)); else if (onAddVirtualBusNode) addVirtualBusNode(entry.virtualKind === "virtualRenderSource" ? "renderSource" : "captureSink", positionFor(session.nodes.length)); else routeLibraryDrop(entry.virtualKind!, positionFor(session.nodes.length)); }} onDragStart={(event) => { if (!canEdit || unavailable) return; event.dataTransfer.effectAllowed = "copy"; event.dataTransfer.setData(LIBRARY_DROP_MIME, kind); event.dataTransfer.setData(LIBRARY_DROP_TEXT_MIME, kind); }}>
      <span className="canvas-library-item-icon" aria-hidden="true">{entry.label.charAt(0)}</span>
      <span>{entry.label}</span>
    </button>;
  };
  const captureConnectionHandle = (event: { target: EventTarget | null; clientX?: number; clientY?: number; button?: number }) => {
    const target = event.target as HTMLElement | null;
    let handle = target?.closest<HTMLElement>("[data-debug-handle-id]");
    if (!handle && typeof document !== "undefined" && event.clientX !== undefined && event.clientY !== undefined) {
      handle = document.elementsFromPoint(event.clientX, event.clientY).find((element): element is HTMLElement => element instanceof HTMLElement && Boolean(element.dataset.debugHandleId)) ?? null;
    }
    if (!handle) return;
    const handleId = handle.dataset.debugHandleId;
    const side = handle.dataset.debugSide as EdgeSide | undefined;
    const direction = handle.dataset.debugDirection as "source" | "target" | undefined;
    const nodeId = handle.dataset.nodeid;
    const node = session.nodes.find((candidate) => candidate.id === nodeId);
    if (!handleId || !side || !direction || !nodeId || !node) return;
    const captured = { nodeId, handleId, side };
    if (direction === "source") sourceHandleRef.current = captured;
    else targetHandleRef.current = captured;
    logConnectionDebug("handle-capture", {
      nodeId,
      nodeName: node.name,
      direction,
      side,
      handleId,
      clientX: event.clientX,
      clientY: event.clientY,
      button: event.button,
    });
  };
  const nodes: FlowNode[] = session.nodes.map((node, index) => {
    const telemetry = nodeTelemetryFor(node, diagnostics);
    const recorderStatus = node.kind === "recorder" ? recorderStatuses.find((status) => status.nodeId === node.id) ?? null : null;
    const mixerInputCount = node.kind === "mixer" ? session.edges.filter((edge) => edge.destinationNode === node.id).length : 0;
    return ({
    id: node.id,
    type: "flowNode",
    position: positions[node.id] ?? positionFor(index),
    data: {
      label: (
        <div className={`flow-node-content node-kind-${node.kind}`} aria-label={`${node.name}, ${node.kind}`} onMouseDownCapture={captureConnectionHandle} onPointerDownCapture={captureConnectionHandle}>
          {node.ports.filter((port) => port.direction === "input").map((port) => { const offset = portOffset(node, port); return EDGE_SIDES.map((side) => { const handleId = edgeHandleId(port.name, side); return <Handle key={`input-${port.name}-${side}`} type="target" id={handleId} position={edgeSidePosition(side)} style={side === "left" || side === "right" ? { top: offset } : { left: offset }} aria-label={side === "left" ? `${node.name} ${port.name} input` : `${node.name} ${port.name} input ${side} connector`} data-debug-side={side} data-debug-direction="target" data-debug-handle-id={handleId} onMouseDown={(event) => { targetHandleRef.current = { nodeId: node.id, handleId, side }; logConnectionDebug("handle-mousedown", { nodeId: node.id, nodeName: node.name, direction: "target", port: port.name, side, handleId, clientX: event.clientX, clientY: event.clientY, button: event.button }); }} onPointerDown={(event) => logConnectionDebug("handle-pointer-down", { nodeId: node.id, nodeName: node.name, direction: "target", port: port.name, side, handleId, clientX: event.clientX, clientY: event.clientY, button: event.button })} />; }); })}
          <div className="flow-node-kicker"><span className={`node-kind-family node-kind-family-${nodeKindFamily(node.kind)}`}>{NODE_KIND_FAMILY_LABELS[nodeKindFamily(node.kind)]}</span><span className="node-kind">{humanizeNodeKind(node)}</span><span className={`node-state ${node.bypass ? "is-bypassed" : node.enabled ? "is-ready" : "is-disabled"}`}>{node.bypass ? "bypass" : node.enabled ? "ready" : "off"}</span></div>
          <div className="flow-node-title"><strong>{node.name}</strong>{canEdit && <button type="button" className="flow-node-delete" aria-label={`Delete ${node.name}`} title={`Delete ${node.name}`} onClick={(event) => { event.stopPropagation(); if (onRemoveNode) onRemoveNode(node.id); else globalThis.dispatchEvent(new CustomEvent("audiorouter:remove-node", { detail: { nodeId: node.id } })); }}>×</button>}</div>
          <NodeVisual node={node} telemetry={telemetry} onSetNodeParameter={onSetNodeParameter} recorderStatus={recorderStatus} mixerInputCount={mixerInputCount} />
          <small className="node-port-count">{node.ports.length} port{node.ports.length === 1 ? "" : "s"} · {node.enabled ? "enabled" : "disabled"}</small>
          <span className="flow-port-list">{nodePortLabels(node).map((port) => <small key={port}>{port}</small>)}</span>
          {node.ports.filter((port) => port.direction === "output").map((port) => { const offset = portOffset(node, port); return EDGE_SIDES.map((side) => { const handleId = edgeHandleId(port.name, side); return <Handle key={`output-${port.name}-${side}`} type="source" id={handleId} position={edgeSidePosition(side)} style={side === "left" || side === "right" ? { top: offset } : { left: offset }} aria-label={side === "right" ? `${node.name} ${port.name} output` : `${node.name} ${port.name} output ${side} connector`} data-debug-side={side} data-debug-direction="source" data-debug-handle-id={handleId} onMouseDown={(event) => { sourceHandleRef.current = { nodeId: node.id, handleId, side }; logConnectionDebug("handle-mousedown", { nodeId: node.id, nodeName: node.name, direction: "source", port: port.name, side, handleId, clientX: event.clientX, clientY: event.clientY, button: event.button }); }} onPointerDown={(event) => logConnectionDebug("handle-pointer-down", { nodeId: node.id, nodeName: node.name, direction: "source", port: port.name, side, handleId, clientX: event.clientX, clientY: event.clientY, button: event.button })} />; }); })}
        </div>
      ),
    },
    className: `flow-node-${nodeFlowGroup(node.kind)}${node.kind === "plugin" ? " flow-node-vst" : ""}`,
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
    data: { enabled: edge.enabled, active, sourceSide: edgeSides[edge.id]?.source, targetSide: edgeSides[edge.id]?.target, onSetSide: setEdgeSide, onToggle: onToggleConnection, onRemove: onRemoveConnection, onInsert: onInsertProcessor, onOpenPluginPicker, canEdit },
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
        <div className="canvas-library" aria-label="Drag processors to canvas">
          <strong>Drag or select to add</strong>
          {FLOW_GROUPS.map((group) => {
            const entries = libraryEntries.filter((entry) => entry.flow === group && (entry.kind !== undefined || entry.virtualKind !== undefined)).sort((left, right) => left.label.localeCompare(right.label));
            if (entries.length === 0) return null;
            return <div key={group} className={`canvas-library-group canvas-library-group-${group}`}>
              <span className="canvas-library-group-label">{FLOW_GROUP_LABELS[group]}</span>
              {entries.map((entry) => libraryButton(entry))}
              {group === "input" && <button type="button" className="canvas-library-item" disabled={!canEdit} onClick={() => onOpenApplicationPicker?.()}>
                <span className="canvas-library-item-icon" aria-hidden="true">A</span>
                <span>Application</span>
              </button>}
              {group === "tool" && <button type="button" className="canvas-library-item" disabled={!canEdit} onClick={() => onOpenPluginPicker?.()}>
                <span className="canvas-library-item-icon" aria-hidden="true">P</span>
                <span>Plugin (VST2/VST3)</span>
              </button>}
            </div>;
          })}
        </div>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={NODE_TYPES}
        onInit={(instance) => { flowInstanceRef.current = instance; if (session.nodes.length > 0 && !initialFitDoneRef.current) { initialFitDoneRef.current = true; globalThis.requestAnimationFrame(() => instance.fitView({ padding: 0.2 })); } }}
        nodesConnectable={canEdit}
        nodesDraggable
        selectionOnDrag
        onSelectionChange={({ nodes: selectedNodes }) => onSelectMany?.(selectedNodes.map((node) => node.id))}
        onNodeClick={(_, node) => onSelect(node.id)}
        onMouseDownCapture={captureConnectionHandle}
        onPointerDownCapture={captureConnectionHandle}
        onConnectStart={(event, params) => { sourceHandleRef.current = null; targetHandleRef.current = null; pendingConnectionRef.current = null; captureConnectionHandle(event); logConnectionDebug("react-flow-connect-start", { handleType: params.handleType, nodeId: params.nodeId, handleId: params.handleId, capturedSource: sourceHandleRef.current, capturedTarget: targetHandleRef.current }); }}
        onConnectEnd={(event, connectionState) => { const clientX = "clientX" in event ? event.clientX : undefined; const clientY = "clientY" in event ? event.clientY : undefined; captureConnectionHandle(event); const pending = pendingConnectionRef.current; if (pending && connectionState.toNode?.id === pending.connection.target) { const targetNode = session.nodes.find((node) => node.id === pending.connection.target); let rawTargetHandle = pending.rawTargetHandle ?? (targetHandleRef.current?.nodeId === pending.connection.target ? targetHandleRef.current.handleId : null); if (!rawTargetHandle && targetNode && clientX !== undefined && clientY !== undefined && typeof document !== "undefined") { const nodeElement = Array.from(document.querySelectorAll<HTMLElement>(".react-flow__node")).find((element) => element.dataset.id === targetNode.id); const bounds = nodeElement?.getBoundingClientRect(); if (bounds) { const distances: Record<EdgeSide, number> = { left: Math.abs(clientX - bounds.left), right: Math.abs(clientX - bounds.right), top: Math.abs(clientY - bounds.top), bottom: Math.abs(clientY - bounds.bottom) }; const side = EDGE_SIDES.reduce((closest, candidate) => distances[candidate] < distances[closest] ? candidate : closest, "left" as EdgeSide); const port = targetNode.ports.find((candidate) => candidate.direction === "input")?.name; if (port) { rawTargetHandle = edgeHandleId(port, side); targetHandleRef.current = { nodeId: targetNode.id, handleId: rawTargetHandle, side }; } } } const target = logicalPortHandle(rawTargetHandle); const finalConnection = { ...pending.connection, targetHandle: target.port }; const edgeId = onConnect(finalConnection); logConnectionDebug("draft-connect-result", { edgeId, normalizedSourceHandle: finalConnection.sourceHandle, normalizedTargetHandle: finalConnection.targetHandle, sourceSide: pending.sourceSide, targetSide: target.side }); if (edgeId) { if (pending.sourceSide) setEdgeSide(edgeId, "source", pending.sourceSide); if (target.side) setEdgeSide(edgeId, "target", target.side); } } else if (!pending && connectionState.fromHandle && connectionState.toHandle && connectionState.fromHandle.type === connectionState.toHandle.type) { onConnectionRejected?.(connectionState.fromHandle.type === "target" ? "That drag started and ended on an input; drag from an output (orange) to an input (blue) instead." : "That drag started and ended on an output; drag from an output (orange) to an input (blue) instead."); } pendingConnectionRef.current = null; sourceHandleRef.current = null; targetHandleRef.current = null; logConnectionDebug("react-flow-connect-end", { inProgress: "inProgress" in connectionState ? connectionState.inProgress : false, fromNode: connectionState.fromNode?.id, fromHandle: connectionState.fromHandle?.id, toNode: connectionState.toNode?.id, toHandle: connectionState.toHandle?.id, toPosition: connectionState.toPosition }); }}
        onConnect={canEdit ? (connection) => { const rawSourceHandle = connection.sourceHandle ?? (sourceHandleRef.current?.nodeId === connection.source ? sourceHandleRef.current.handleId : null); const rawTargetHandle = connection.targetHandle ?? (targetHandleRef.current?.nodeId === connection.target ? targetHandleRef.current.handleId : null); logConnectionDebug("react-flow-connect", { source: connection.source, sourceHandle: connection.sourceHandle, recoveredSourceHandle: rawSourceHandle, target: connection.target, targetHandle: connection.targetHandle, recoveredTargetHandle: rawTargetHandle }); const source = logicalPortHandle(rawSourceHandle); const target = logicalPortHandle(rawTargetHandle); pendingConnectionRef.current = { connection: { ...connection, sourceHandle: source.port, targetHandle: target.port }, sourceSide: source.side, targetSide: target.side, rawTargetHandle }; } : undefined}
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
