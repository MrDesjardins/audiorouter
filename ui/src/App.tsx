import { createContext, useContext, useEffect, useRef, useState, type ChangeEvent } from "react";
import type { NativeDuplexPumpResult, NativeEndpointPumpResult, Node, PluginParametersResult, RecordingRecoveryItem, RouteInspection, StateEventCategory } from "@audiorouter/contracts";
import { LIBRARY_DROP_SOURCE, SessionFlowCanvas } from "./SessionFlowCanvas";
import { createDisconnectedBackend, formatUiError, isRevisionConflict, SnapshotCache, type ApplicationRow, type RecorderStatus, type UiBackend } from "./backend";
import type { DeviceListItem } from "@audiorouter/contracts";
import { appendApplicationCaptureNode, appendDraftConnection, appendEqPresetNode, appendLibraryNode, appendPluginPlaceholderNode, appendVoiceChainPreset, applyGraphDraft, duplicateDraftNode, insertDraftMixer, insertDraftProcessor, removeDraftConnection, removeDraftNode, removeSinglePathDraftMixer, resetNodeDraftParameters, setDraftConnectionEnabled, setNodeDraftFlag, setNodeDraftName, setNodeDraftParameter, setSessionDraftName, type EqPresetId, type InsertableProcessorKind, type LibraryNodeKind, type VoiceChainPresetId } from "./draft";
import { demoSession, demoSessions } from "./fixtures";
import { recordDraft, redoDraft as redoDraftHistory, undoDraft as undoDraftHistory, type DraftHistory } from "./history";
import { templateSession, type TemplateId } from "./templates";
import { filterLibraryEntries, libraryEntries, libraryEntryAccessibleLabel } from "./library";
import { selectNativePump } from "./nativePump";
import { nodePortLabels, nodeStateLabel, routeLatencyText, routeNodeLabels } from "./graphView";
import { readCompactStatus, readShortcuts, readTheme, writeCompactStatus, writeShortcuts, writeTheme, type ThemeMode } from "./preferences";
import { defaultShortcutBinding, isEditableShortcutTarget, shortcutConflicts, shortcutFromKeyboardEvent, type ShortcutAction, type ShortcutBinding } from "./shortcuts";
import { ApplicationIdentityPanel } from "./ApplicationIdentityPanel";
import { setupChecklist } from "./setup";
import { uiIdempotencyKey } from "./idempotency";
import { processorAvailabilityText, processorLatencyText, processorParameterError, processorParametersText, type ProcessorDescriptor } from "./processorCatalog";
import { mergeSessionInventory } from "./sessionInventory";
import type { Connection } from "@xyflow/react";
import { DraftConnectionList, decodeTopologyAction } from "./DraftConnectionList";
import { BackendConnectionContext } from "./backendConnectionContext";
import { GraphList as NodeList } from "./GraphList";

const defaultBackend = createDisconnectedBackend();
const PluginParameterContext = createContext<{ parameters: PluginParametersResult | null; error: string | null }>({ parameters: null, error: null });

/** State categories that can invalidate the workspace snapshot. Meter events
 * are intentionally excluded; diagnostics use the bounded snapshot path. */
export const WORKSPACE_EVENT_CATEGORIES = [
  "graph.committed",
  "runtime.crashed",
  "runtime.started",
  "runtime.activated",
  "runtime.stopped",
  "devices.changed",
  "recovery.safeModeCleared",
  "virtualDevice.changed",
  "virtualBridge.failed",
  "virtualBridge.expired",
  "recorder.changed",
  "recording.metadataChanged",
  "recording.renamed",
  "recording.entryRemoved",
  "recording.recycled",
] as const satisfies readonly StateEventCategory[];

/** Default diagnostics/meter refresh: 20 Hz, below the API's 30 Hz ceiling. */
export const DIAGNOSTICS_REFRESH_INTERVAL_MS = 50;

type NativePumpStats = NativeEndpointPumpResult | NativeDuplexPumpResult;

export function formatNativePumpSummary(stats: NativePumpStats | null, running: boolean): string | null {
  if (!stats || !running) return null;
  const input = "input" in stats ? stats.input : stats;
  const output = "output" in stats ? stats.output : stats;
  const warnings = [
    output.droppedRenderFrames > 0 ? `${output.droppedRenderFrames} dropped` : null,
    output.renderBackpressureEvents > 0 ? `${output.renderBackpressureEvents} backpressure` : null,
  ].filter((value): value is string => value !== null);
  const recorderChunks = "recorderChunksDrained" in stats ? stats.recorderChunksDrained : 0;
  const processedQuanta = "input" in stats
    ? input.processedQuanta + output.processedQuanta
    : stats.processedQuanta;
  return `native ${input.capturedFrames} in / ${output.renderedFrames} out / ${processedQuanta} quanta${recorderChunks > 0 ? ` / ${recorderChunks} recorder chunks` : ""}${warnings.length > 0 ? ` / ${warnings.join(" / ")}` : ""}`;
}

export function formatRecordingDuration(frames: number, sampleRate: number): string {
  if (!Number.isFinite(frames) || frames < 0 || !Number.isFinite(sampleRate) || sampleRate <= 0) return "unknown";
  const totalMilliseconds = Math.round((frames / sampleRate) * 1000);
  const hours = Math.floor(totalMilliseconds / 3_600_000);
  const minutes = Math.floor((totalMilliseconds % 3_600_000) / 60_000);
  const seconds = Math.floor((totalMilliseconds % 60_000) / 1000);
  const milliseconds = totalMilliseconds % 1000;
  return `${hours.toString().padStart(2, "0")}:${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}.${milliseconds.toString().padStart(3, "0")}`;
}

function RecoveryCheckpointPanel({ backend }: { backend: UiBackend }) {
  const [items, setItems] = useState<RecordingRecoveryItem[]>([]);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    void backend.listRecordingRecovery().then((result) => {
      if (active) {
        setItems(result.items);
        setError(null);
      }
    }).catch((reason) => {
      if (active) {
        setItems([]);
        setError(formatUiError(reason, "Recording recovery unavailable."));
      }
    });
    return () => { active = false; };
  }, [backend]);
  return <section className="panel recovery-checkpoints" aria-labelledby="recovery-checkpoints-heading">
    <div className="section-heading"><div><p className="eyebrow">Recorder files</p><h2 id="recovery-checkpoints-heading">Recovery checkpoints</h2></div><span className="badge">{error ? "unavailable" : items.length}</span></div>
    {error ? <p className="muted">{error}</p> : items.length === 0 ? <p className="muted">No persisted recorder checkpoints were found.</p> : <ul aria-label="Persisted recorder checkpoints">{items.map((item) => <li key={item.recordingId}><code>{item.recordingId}</code> — {item.status}{item.checkpoint ? ` (${item.checkpoint.state})` : ""}</li>)}</ul>}
    <p className="muted">This list is read-only. Recovery inspection does not open, repair, play, or delete audio files.</p>
  </section>;
}

function StartupPanel({ backend }: { backend: UiBackend }) {
  const [status, setStatus] = useState<import("@audiorouter/contracts").StartupStatus | null>(null);
  const [enabled, setEnabled] = useState(false);
  const [plan, setPlan] = useState<import("@audiorouter/contracts").StartupPlanResult | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [nativeRegistration, setNativeRegistration] = useState<"registered" | "unregistered" | "unavailable">("unavailable");
  const refreshGeneration = useRef(0);
  const [busy, setBusy] = useState(false);
  const refresh = (clearMessage = true) => {
    const generation = ++refreshGeneration.current;
    void backend.getStartup().then((result) => {
      if (generation !== refreshGeneration.current) return;
      setStatus(result); setEnabled(result.enabled); if (clearMessage) setMessage(null);
    }).catch((error) => {
      if (generation === refreshGeneration.current) setMessage(formatUiError(error, "Startup status unavailable."));
    });
    if (backend.startupRegistrationStatus) void backend.startupRegistrationStatus().then((result) => {
      if (generation === refreshGeneration.current) setNativeRegistration(result);
    }).catch(() => {
      if (generation === refreshGeneration.current) setNativeRegistration("unavailable");
    });
  };
  useEffect(() => { setPlan(null); refresh(); }, [backend, backend.connected]);
  const createPlan = async () => {
    if (busy || !backend.connected) return;
    setBusy(true);
    setMessage("Planning sign-in startup policy...");
    try { const result = await backend.planStartup(enabled); setPlan(result); setMessage(result.reason); }
    catch (error) { setMessage(formatUiError(error, "Startup planning unavailable.")); }
    finally { setBusy(false); }
  };
  const applyPlan = async () => {
    if (!plan || busy || !backend.connected) return;
    setBusy(true);
    const plannedEnabled = plan.enabled;
    setMessage("Applying startup policy...");
    try {
      const result = await backend.applyStartup(plan.planId, uiIdempotencyKey("startup-apply"));
      if (backend.registerStartup) {
        try {
          const commandLine = await backend.registerStartup(plannedEnabled);
          setNativeRegistration(plannedEnabled ? "registered" : "unregistered");
          setMessage(`${plannedEnabled ? "Startup registration enabled" : "Startup registration disabled"}: ${commandLine}`);
        } catch (error) {
          setMessage(`Backend policy saved, but native startup registration failed: ${formatUiError(error, "native registration failed")}`);
        }
      } else {
        setMessage(result.reason);
      }
      setPlan(null); refresh(false);
    }
    catch (error) { setMessage(formatUiError(error, "Startup apply unavailable.")); }
    finally { setBusy(false); }
  };
  return <section className="panel startup-panel" aria-labelledby="startup-heading"><div className="section-heading"><div><p className="eyebrow">Background lifecycle</p><h2 id="startup-heading">Start at sign-in</h2></div><button type="button" className="secondary" onClick={() => refresh()} disabled={busy}>Refresh</button></div><p className="muted">{status?.reason ?? "Loading startup capability..."}</p><p className="muted" role="status">Native registration: {nativeRegistration}</p><label>Desired policy<select aria-label="Desired sign-in startup policy" value={enabled ? "enabled" : "disabled"} onChange={(event) => { setEnabled(event.target.value === "enabled"); setPlan(null); }} disabled={!backend.connected || busy}><option value="disabled">Disabled</option><option value="enabled">Enabled</option></select></label><div className="actions"><button type="button" className="secondary" onClick={() => void createPlan()} disabled={!backend.connected || busy}>Plan startup policy</button>{plan && <button type="button" className="secondary" onClick={() => void applyPlan()} disabled={!backend.connected || busy}>Apply planned policy</button>}</div>{message && <p className="muted" role="status">{message}</p>}<p className="muted">{backend.registerStartup ? "The native shell can register this user's startup preference after an authorized plan is applied." : "Native startup registration is unavailable in this host; planning remains a backend-only operation."}</p></section>;
}

function OsTransitionPanel({ backend, onRefresh }: { backend: UiBackend; onRefresh: () => void }) {
  const [result, setResult] = useState<import("@audiorouter/contracts").OsTransitionResult | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const revalidate = async () => {
    if (!backend.connected || !backend.osTransition || busy) return;
    setBusy(true);
    setMessage("Refreshing endpoints and checking stopped routes...");
    try {
      const next = await backend.osTransition("resume", uiIdempotencyKey("os-resume"));
      setResult(next);
      onRefresh();
      setMessage(next.action === "revalidateBeforeRestart"
        ? `Revalidation required for ${next.sessionIds.length + next.nativeSessionIds.length} stopped route${next.sessionIds.length + next.nativeSessionIds.length === 1 ? "" : "s"}. No route was restarted.`
        : "No stopped routes are waiting for revalidation.");
    } catch (error) {
      setMessage(formatUiError(error, "OS-transition revalidation failed."));
    } finally {
      setBusy(false);
    }
  };
  const restartPortable = async () => {
    if (!result || result.sessionIds.length === 0 || busy) return;
    setBusy(true);
    let started = 0;
    const failures: string[] = [];
    for (const sessionId of result.sessionIds) {
      try {
        await backend.startSession(sessionId, uiIdempotencyKey(`os-restart-${sessionId}`));
        started += 1;
      } catch (error) {
        failures.push(`${sessionId}: ${formatUiError(error, "start failed")}`);
      }
    }
    setMessage(failures.length === 0
      ? `Restarted ${started} validated portable route${started === 1 ? "" : "s"}. Native routes remain stopped.`
      : `Restarted ${started} portable route${started === 1 ? "" : "s"}; ${failures.length} route${failures.length === 1 ? "" : "s"} failed. ${failures.join(" ")}`);
    onRefresh();
    setBusy(false);
  };
  return <section className="panel os-transition-panel" aria-labelledby="os-transition-heading"><div className="section-heading"><div><p className="eyebrow">Background lifecycle</p><h2 id="os-transition-heading">Resume validation</h2></div><span className="badge">{result?.endpointInventory ?? "not run"}</span></div><p className="muted">After sleep, sign-out, or an endpoint change, refresh the exact endpoint inventory before restarting any stopped route.</p><div className="actions"><button type="button" className="secondary" onClick={() => void revalidate()} disabled={!backend.connected || !backend.osTransition || busy}>Revalidate after resume</button>{result?.sessionIds.length ? <button type="button" className="primary" onClick={() => void restartPortable()} disabled={!backend.connected || busy}>Restart validated portable routes</button> : null}</div>{result && <p className="muted" role="status">Last result: {result.action}; portable routes {result.sessionIds.length}; native routes {result.nativeSessionIds.length}.</p>}{message && <p className="muted" role="status" aria-live="polite">{message}</p>}<p className="muted">Native routes remain stopped until deliberate endpoint/driver validation. This action never changes Windows defaults, volume, mute, or driver state.</p></section>;
}

export function App({ backend }: { backend?: UiBackend } = {}) {
  const resolvedBackend = backend ?? defaultBackend;
  return <BackendConnectionContext.Provider value={resolvedBackend.connected}><AppContent backend={resolvedBackend} /></BackendConnectionContext.Provider>;
}

function ProcessorCatalog({ processors, error, node, backend }: { processors: ProcessorDescriptor[] | null; error: string | null; node: Node; backend: UiBackend }) {
  return <section className="panel processor-catalog" aria-labelledby="processor-catalog-heading"><div className="section-heading"><div><p className="eyebrow">DSP catalog</p><h2 id="processor-catalog-heading">Built-in processors</h2></div><span className="badge">{processors?.length ?? 0}</span></div>{error ? <p className="muted" role="status">Processor catalog unavailable: {error}</p> : processors === null ? <p className="muted">Connect to the backend to load the authoritative processor catalog.</p> : processors.length === 0 ? <p className="muted">No built-in processors are advertised.</p> : <ul aria-label="Built-in processor catalog">{processors.map((processor) => <li key={`${processor.id}@${processor.version}`}><strong>{processor.id}</strong> <small>{processor.category} · {processorAvailabilityText(processor)} · {processorLatencyText(processor)}</small><br /><small>Parameters: {processorParametersText(processor)}</small></li>)}</ul>}<p className="muted">This catalog is read-only. Unavailable processors cannot be added or activated.</p><EqResponsePreview node={node} backend={backend} /></section>;
}

function ProcessorParameterEditor({ node, processors, pluginParameters = null, pluginParameterError = null, connected, onChange }: { node: Node; processors: ProcessorDescriptor[] | null; pluginParameters?: PluginParametersResult | null; pluginParameterError?: string | null; connected: boolean; onChange: (name: string, value: boolean | number | string) => void }) {
  const pluginContext = useContext(PluginParameterContext);
  pluginParameters ??= pluginContext.parameters;
  pluginParameterError ??= pluginContext.error;
  if (node.kind === "plugin") {
    if (pluginParameterError) return <p className="muted" role="status">Plugin parameters unavailable: {pluginParameterError}</p>;
    if (!pluginParameters) return <p className="muted" role="status">Loading bounded parameters from the exact scanned plugin...</p>;
    if (pluginParameters.parameters.length === 0) return <p className="muted" role="status">This plugin exposes no automatable parameters.</p>;
    return <>{pluginParameters.parameters.map((parameter) => { const name = `pluginParameter:${parameter.parameterId}`; const value = typeof node.parameters[name] === "number" && Number.isFinite(node.parameters[name] as number) ? node.parameters[name] as number : parameter.defaultValue; return <label key={name}><span>{parameter.title}</span><input type="range" aria-label={`${parameter.title} slider`} value={value} min={parameter.minimum} max={parameter.maximum} step={0.001} disabled={!connected} onChange={(event) => onChange(name, Number(event.target.value))} /><input type="number" aria-label={`${parameter.title} precise value`} value={value} min={parameter.minimum} max={parameter.maximum} step={0.001} disabled={!connected} onChange={(event) => onChange(name, Number(event.target.value))} /><small>normalized parameter {parameter.parameterId}</small></label>; })}</>;
  }
  const descriptor = processors?.find((processor) => processor.id === node.kind);
  if (!descriptor || descriptor.parameters.length === 0) return null;
  return <>{descriptor.parameters.map((parameter) => {
    const value = node.parameters[parameter.name];
    if (parameter.type === "boolean") {
      return <label key={parameter.name}>{parameter.name}<input type="checkbox" checked={value === true} disabled={!connected} onChange={(event) => onChange(parameter.name, event.target.checked)} /></label>;
    }
    if (parameter.type === "string" && parameter.enum) {
      const fallback = typeof parameter.default === "string" && parameter.enum.includes(parameter.default) ? parameter.default : parameter.enum[0];
      const stringValue = typeof value === "string" && parameter.enum.includes(value) ? value : fallback;
      return <label key={parameter.name}>{parameter.name}<select value={stringValue} disabled={!connected} onChange={(event) => onChange(parameter.name, event.target.value)}>{parameter.enum.map((option) => <option key={option} value={option}>{option}</option>)}</select></label>;
    }
    if (parameter.type !== "number") return null;
    const fallback = typeof parameter.default === "number" ? parameter.default : 0;
    const numericValue = typeof value === "number" && Number.isFinite(value) ? value : fallback;
    const step = parameter.unit === "Hz" ? 1 : 0.1;
    const hasRange = Number.isFinite(parameter.minimum) && Number.isFinite(parameter.maximum) && parameter.minimum! < parameter.maximum!;
    const sliderValue = hasRange ? Math.min(parameter.maximum!, Math.max(parameter.minimum!, numericValue)) : numericValue;
    return <label key={parameter.name}><span>{parameter.name}{parameter.unit ? ` (${parameter.unit})` : ""}</span>{hasRange && <input type="range" aria-label={`${parameter.name} slider`} value={sliderValue} min={parameter.minimum} max={parameter.maximum} step={step} disabled={!connected} onChange={(event) => onChange(parameter.name, Number(event.target.value))} />}<input type="number" aria-label={`${parameter.name} precise value`} value={numericValue} min={parameter.minimum} max={parameter.maximum} step={step} disabled={!connected} onChange={(event) => onChange(parameter.name, Number(event.target.value))} /></label>;
  })}</>;
}

function InspectorChangeSummary({ draftNode, authoritativeNode }: { draftNode: Node; authoritativeNode?: Node }) {
  if (!authoritativeNode) {
    return <p className="muted" role="status">Draft effect: adds this node and its configured processing when the graph is committed.</p>;
  }
  const changes: string[] = [];
  if (draftNode.name !== authoritativeNode.name) changes.push('rename to "' + draftNode.name + '"');
  if (draftNode.enabled !== authoritativeNode.enabled) changes.push(draftNode.enabled ? "enable node" : "disable node");
  if (draftNode.bypass !== authoritativeNode.bypass) changes.push(draftNode.bypass ? "bypass processing" : "resume processing");
  const parameterNames = [...new Set([...Object.keys(authoritativeNode.parameters), ...Object.keys(draftNode.parameters)])].sort();
  for (const name of parameterNames) {
    if (!Object.is(authoritativeNode.parameters[name], draftNode.parameters[name])) {
      changes.push(name + ": " + String(authoritativeNode.parameters[name] ?? "unset") + " → " + String(draftNode.parameters[name] ?? "unset"));
    }
    if (changes.length === 4) break;
  }
  const more = changes.length === 4 && parameterNames.length > 4 ? "; more changes available in the inspector" : "";
  return <p className="muted" role="status">{changes.length === 0 ? "Draft effect: no changes to this node." : "Draft effect: " + changes.join("; ") + more + ". Plan changes to validate and commit."}</p>;
}

function NodeTelemetryPanel({ node, snapshot }: { node: Node; snapshot: import("@audiorouter/contracts").DiagnosticsSnapshot | null }) {
  const observation = snapshot?.nodeTelemetry.find((item) => item.nodeId === node.id);
  return <section className="node-telemetry" aria-labelledby="node-telemetry-heading"><div className="section-heading"><div><p className="eyebrow">Observed runtime</p><h3 id="node-telemetry-heading">Node telemetry</h3></div><span className="badge">{observation ? "available" : "not available"}</span></div>{!snapshot ? <p className="muted">Waiting for a backend diagnostics snapshot.</p> : !observation ? <p className="muted">No prepared meter or dynamics observation is available for this node. Draft and stopped nodes remain configuration-only.</p> : <div className="telemetry-values">{observation.meter && <p><strong>Meter</strong> · peak {observation.meter.peakDb.toFixed(1)} dB · RMS {observation.meter.rmsDb.toFixed(1)} dB · clipped {observation.meter.clippedSamples}</p>}{observation.processor && <p><strong>Processor</strong> · gain reduction {observation.processor.gainReductionDb.slice(0, 2).map((value) => `${value.toFixed(1)} dB`).join(" / ")} · gate {observation.processor.gateOpen.slice(0, 2).map((open) => open ? "open" : "closed").join(" / ")}</p>}</div>}<p className="muted">Values are bounded backend observations and may be unavailable while the realtime stage is busy.</p></section>;
}

const EQ_RESPONSE_FREQUENCIES = Array.from({ length: 48 }, (_, index) => 20 * Math.pow(1000, index / 47));

function EqResponsePreview({ node, backend }: { node: Node; backend: UiBackend }) {
  const [response, setResponse] = useState<import("./backend").ProcessorResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (node.kind !== "parametricEq" || !backend.connected) { setResponse(null); setError(null); return; }
    let active = true;
    const bands = Array.from({ length: 8 }, (_, index) => {
      const prefix = `band${index}`;
      const legacy = index === 0;
      return {
        enabled: typeof node.parameters[`${prefix}Enabled`] === "boolean" ? node.parameters[`${prefix}Enabled`] as boolean : legacy && node.parameters.frequencyHz !== undefined,
        type: (node.parameters[`${prefix}Type`] ?? "peaking") as "peaking" | "lowShelf" | "highShelf" | "lowPass" | "highPass" | "notch",
        frequencyHz: Number(node.parameters[`${prefix}FrequencyHz`] ?? (legacy ? node.parameters.frequencyHz : 1000)),
        q: Number(node.parameters[`${prefix}Q`] ?? (legacy ? node.parameters.q : 1)),
        gainDb: Number(node.parameters[`${prefix}GainDb`] ?? (legacy ? node.parameters.gainDb : 0)),
      };
    });
    void backend.processorResponse({ sampleRateHz: 48000, bands, frequenciesHz: EQ_RESPONSE_FREQUENCIES }).then((value) => { if (active) { setResponse(value); setError(null); } }).catch((reason) => { if (active) { setResponse(null); setError(formatUiError(reason, "EQ response unavailable.")); } });
    return () => { active = false; };
  }, [backend, node.kind, node.parameters]);
  if (node.kind !== "parametricEq") return null;
  const points = response?.frequenciesHz.map((frequency, index) => { const magnitude = response.magnitudeDb[index] ?? 0; const x = 8 + (Math.log10(frequency / 20) / 3) * 284; const bounded = Math.max(-24, Math.min(24, magnitude)); const y = 56 - ((bounded + 24) / 48) * 48; return `${x.toFixed(1)},${y.toFixed(1)}`; }).join(" ");
  return <section className="eq-response" aria-labelledby="eq-response-heading"><h3 id="eq-response-heading">EQ response</h3>{error ? <p className="muted" role="status">{error}</p> : !response ? <p className="muted">Loading the authoritative response...</p> : <svg viewBox="0 0 300 64" role="img" aria-label="Parametric EQ magnitude response"><line x1="8" y1="32" x2="292" y2="32" stroke="currentColor" opacity="0.35" /><polyline points={points} fill="none" stroke="currentColor" strokeWidth="1.5" /></svg>}</section>;
}

function PresetCatalog({ presets, error }: { presets: import("@audiorouter/contracts").DiscoveryDocument["presets"] | null; error: string | null }) {
  const request = (kind: "eq" | "voiceChain", presetId: string) => {
    globalThis.dispatchEvent(new CustomEvent(kind === "eq" ? "audiorouter:append-eq-preset" : "audiorouter:append-voice-preset", { detail: { presetId } }));
  };
  return <section className="panel preset-catalog" aria-labelledby="preset-catalog-heading">
    <div className="section-heading"><div><p className="eyebrow">Saved starting points</p><h2 id="preset-catalog-heading">Presets</h2></div><span className="badge">{presets ? presets.voiceChains.length + presets.eq.length : 0}</span></div>
    {error ? <p className="muted" role="status">Preset catalog unavailable: {error}</p> : presets === null ? <p className="muted">Connect to the backend to load the authoritative preset catalog.</p> : <ul aria-label="Available presets">
      {presets.voiceChains.map((preset) => <li key={"voice-" + preset.id}><strong>{preset.name}</strong> <small>Voice chain · {preset.description}</small><button type="button" className="secondary" disabled={!presets} onClick={() => request("voiceChain", preset.id)}>Add voice chain to draft</button></li>)}
      {presets.eq.map((preset) => <li key={"eq-" + preset.id}><strong>{preset.name}</strong> <small>EQ · {preset.description}</small><button type="button" className="secondary" disabled={!presets} onClick={() => request("eq", preset.id)}>Add EQ to draft</button></li>)}
    </ul>}
    <p className="muted">Presets expand into ordinary draft nodes; all actions remain subject to Plan changes.</p>
  </section>;
}

function SessionTransferPanel({ backend, session, onImported }: { backend: UiBackend; session: import("@audiorouter/contracts").Session; onImported: (session: import("@audiorouter/contracts").Session) => void }) {
  const [message, setMessage] = useState<string | null>(null);
  const [plan, setPlan] = useState<import("@audiorouter/contracts").SessionImportPlanResult | null>(null);
  const [busy, setBusy] = useState(false);
  const importRequest = useRef(0);
  const readFileText = (file: File) => typeof file.text === "function" ? file.text() : new Promise<string>((resolve, reject) => { const reader = new FileReader(); reader.onload = () => resolve(String(reader.result ?? "")); reader.onerror = () => reject(reader.error ?? new Error("Unable to read import file.")); reader.readAsText(file); });
  const exportSession = async () => {
    setMessage("Exporting the selected stopped-session configuration...");
    try {
      const exported = await backend.exportSession(session.id);
      const blob = new Blob([JSON.stringify(exported, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = `${exported.name.replace(/[^a-z0-9-_]+/gi, "-").replace(/^-+|-+$/g, "") || "audiorouter-session"}.audiorouter.json`;
      anchor.click();
      URL.revokeObjectURL(url);
      setMessage(`Exported ${exported.name}. Credentials, grants, recordings, and plugin binaries are not included.`);
    } catch (error) { setMessage(formatUiError(error, "Unable to export session.")); }
  };
  const inspectImport = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    const request = ++importRequest.current;
    setPlan(null);
    setBusy(true);
    setMessage("Validating the selected session import...");
    try {
      const candidate = JSON.parse(await readFileText(file)) as import("@audiorouter/contracts").Session;
      const next = await backend.planSessionImport(candidate);
      if (request !== importRequest.current) return;
      setPlan(next);
      setMessage(`Import validated for ${next.session.name}; it will remain stopped until you commit it.`);
    } catch (error) { if (request === importRequest.current) { setPlan(null); setMessage(formatUiError(error, "Unable to validate session import.")); } }
    finally { if (request === importRequest.current) setBusy(false); }
  };
  const commitImport = async () => {
    if (!plan || busy || !backend.connected) return;
    setBusy(true);
    setMessage("Committing the validated stopped-session import...");
    try {
      const result = await backend.commitSessionImport(plan.planId, uiIdempotencyKey("session-import"));
      setPlan(null);
      onImported(result.session);
      setMessage(`Imported stopped session ${result.session.name}. Review bindings before starting it.`);
    } catch (error) { setMessage(formatUiError(error, "Unable to commit session import.")); }
    finally { setBusy(false); }
  };
  return <section className="panel session-transfer-panel" aria-labelledby="session-transfer-heading"><div className="section-heading"><div><p className="eyebrow">Portable configuration</p><h2 id="session-transfer-heading">Session transfer</h2></div><span className="badge">stopped only</span></div><p className="muted">Export the selected configuration or validate an import. Imports never start audio, arm recorders, enable startup, or include credentials, recordings, plugin binaries, or machine-specific authorization. The UI transfer is JSON; the versioned `.audiorouter` ZIP bundle is available through the headless bundle commands.</p><div className="actions"><button type="button" className="secondary" onClick={() => void exportSession()} disabled={!backend.connected || busy}>Export session</button><label className="file-picker">Import session<input aria-label="Import session configuration" type="file" accept=".json,.audiorouter.json,application/json" onChange={(event) => void inspectImport(event)} disabled={!backend.connected || busy} /></label>{plan && <button type="button" className="primary" onClick={() => void commitImport()} disabled={!backend.connected || busy}>Commit stopped import</button>}</div>{plan && <p className="muted" role="status">Validated import: {plan.session.name} · expires in {Math.ceil(plan.expiresInMs / 1000)} seconds. Explicit commit is required.</p>}{message && <p className="muted" role="status" aria-live="polite">{message}</p>}</section>;
}

function PluginScanPanel({ backend, onAddPlaceholder }: { backend: UiBackend; onAddPlaceholder?: (entry: import("@audiorouter/contracts").PluginScanEntry) => void }) {
  const [directory, setDirectory] = useState("");
  const [result, setResult] = useState<import("@audiorouter/contracts").PluginScanResult | null>(null);
  const [inspectionPath, setInspectionPath] = useState("");
  const [inspection, setInspection] = useState<import("@audiorouter/contracts").PluginScanEntry | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const requestGeneration = useRef(0);
  const directoryRef = useRef("");
  const inspectionPathRef = useRef("");
  useEffect(() => { directoryRef.current = directory; inspectionPathRef.current = inspectionPath; }, [directory, inspectionPath]);
  const scan = async () => {
    if (!directory.trim()) { setMessage("Enter an absolute plugin directory."); return; }
    if (busy || !backend.connected) return;
    const requestedDirectory = directory.trim();
    const request = ++requestGeneration.current;
    setBusy(true);
    setMessage("Scanning selected directory...");
    try { const next = await backend.scanPlugins(requestedDirectory); if (request === requestGeneration.current && directoryRef.current.trim() === requestedDirectory) { setResult(next); setInspection(null); setInspectionPath(""); setMessage("Plugin scan completed without loading plugin code."); } }
    catch (error) { if (request === requestGeneration.current) { setResult(null); setInspection(null); setInspectionPath(""); setMessage(formatUiError(error, "Plugin scan unavailable.")); } }
    finally { if (request === requestGeneration.current) setBusy(false); }
  };
  const list = async () => {
    if (!directory.trim()) { setMessage("Enter an absolute plugin directory."); return; }
    if (busy || !backend.connected) return;
    const requestedDirectory = directory.trim();
    const request = ++requestGeneration.current;
    setBusy(true);
    setMessage("Loading the last explicit plugin scan...");
    try { const next = await backend.listPlugins(requestedDirectory); if (request === requestGeneration.current && directoryRef.current.trim() === requestedDirectory) { setResult(next); setInspection(null); setInspectionPath(""); setMessage("Loaded the last backend scan without rescanning."); } }
    catch (error) { if (request === requestGeneration.current) { setResult(null); setInspection(null); setInspectionPath(""); setMessage(formatUiError(error, "Plugin inventory unavailable.")); } }
    finally { if (request === requestGeneration.current) setBusy(false); }
  };
  const inspect = async () => {
    if (!inspectionPath.trim()) { setMessage("Enter an absolute plugin path."); return; }
    if (busy || !backend.connected) return;
    const requestedPath = inspectionPath.trim();
    const request = ++requestGeneration.current;
    setBusy(true);
    setMessage("Inspecting selected plugin path...");
    try { const next = await backend.inspectPlugin(requestedPath); if (request === requestGeneration.current && inspectionPathRef.current.trim() === requestedPath) { setInspection(next); setMessage("Plugin inspection completed without loading plugin code."); } }
    catch (error) { if (request === requestGeneration.current) { setInspection(null); setMessage(formatUiError(error, "Plugin inspection unavailable.")); } }
    finally { if (request === requestGeneration.current) setBusy(false); }
  };
  const retry = async () => {
    if (!directory.trim()) { setMessage("Enter an absolute plugin directory."); return; }
    if (busy || !backend.connected) return;
    const requestedDirectory = directory.trim();
    const request = ++requestGeneration.current;
    setBusy(true);
    setMessage("Retrying selected directory scan...");
    try { const next = await backend.retryPlugins(requestedDirectory, uiIdempotencyKey("plugins-retry")); if (request === requestGeneration.current && directoryRef.current.trim() === requestedDirectory) { setResult(next); setInspection(null); setInspectionPath(""); setMessage("Plugin scan retry completed without loading plugin code."); } }
    catch (error) { if (request === requestGeneration.current) { setResult(null); setInspection(null); setInspectionPath(""); setMessage(formatUiError(error, "Plugin scan retry unavailable.")); } }
    finally { if (request === requestGeneration.current) setBusy(false); }
  };
  const selectInspectionPath = (path: string) => {
    setInspectionPath(path);
    setInspection(null);
    setMessage("Selected the discovered path; inspect it explicitly when ready.");
  };
  const addToDraft = (entry: import("@audiorouter/contracts").PluginScanEntry) => {
    if (entry.identity && ["supportedVst2X64Gated", "supportedVst3X64"].includes(entry.identity.compatibility) && onAddPlaceholder) {
      onAddPlaceholder(entry);
    }
  };
  return <section className="panel plugin-scan-panel" aria-labelledby="plugin-scan-heading"><div className="section-heading"><div><p className="eyebrow">VST3 and VST2 discovery</p><h2 id="plugin-scan-heading">Plugin scan</h2></div><span className="badge">{result?.entries.length ?? 0}</span></div><p className="muted">Choose a directory explicitly. Discovery inspects bounded metadata only; it does not load or execute plugins.</p><label>Absolute plugin directory<input aria-label="Absolute plugin directory" value={directory} onChange={(event) => setDirectory(event.target.value)} disabled={!backend.connected} placeholder="C:\\Plugins" /></label><button type="button" className="secondary" onClick={() => void scan()} disabled={!backend.connected}>Scan directory</button><button type="button" className="secondary" onClick={() => void list()} disabled={!backend.connected}>Load last scan</button><button type="button" className="secondary" onClick={() => void retry()} disabled={!backend.connected}>Retry scan</button>{result && <ul aria-label="Plugin scan results">{result.entries.length === 0 ? <li className="muted">No VST3, VST2, or other DLL candidates found.</li> : result.entries.map((entry) => { const supported = entry.identity?.compatibility === "supportedVst2X64Gated" || entry.identity?.compatibility === "supportedVst3X64"; return <li key={entry.path}><strong>{entry.path}</strong> <small>{entry.identity ? `${entry.identity.format} · ${entry.identity.architecture} · ${entry.identity.compatibility}` : `${entry.errorCode ?? "unknown"}: ${entry.error ?? "inspection failed"}`}</small><button type="button" className="secondary" onClick={() => selectInspectionPath(entry.path)} disabled={!backend.connected}>Select for inspection</button>{supported && <button type="button" className="secondary" aria-label={`Add to draft: ${entry.path}`} onClick={() => addToDraft(entry)} disabled={!backend.connected || !onAddPlaceholder}>Add to draft</button>}</li>; })}</ul>}<label>Absolute plugin path<input aria-label="Absolute plugin path" value={inspectionPath} onChange={(event) => { setInspectionPath(event.target.value); setInspection(null); }} disabled={!backend.connected} placeholder="C:\\Plugins\\effect.vst3 or effect.dll" /></label><button type="button" className="secondary" onClick={() => void inspect()} disabled={!backend.connected}>Inspect path</button>{inspection && <p className="muted" role="status">{inspection.identity ? `${inspection.identity.format} ${inspection.identity.architecture} · ${inspection.identity.compatibility}` : `${inspection.errorCode ?? "unknown"}: ${inspection.error ?? "inspection failed"}`}</p>}{message && <p className="muted" role="status" aria-live="polite">{message}</p>}<p className="muted">The explicit <code>pluginScan</code> permission is required by the backend; selecting a result only copies its path, and inspection remains explicit. Adding a result to the graph is a separate explicit action. Loading the last scan never triggers a new filesystem scan.</p></section>;
}

function RecorderActions({ backend, sessionId, connected, recorderStatuses, recorderStatusAvailable }: { backend: UiBackend; sessionId: string; connected: boolean; recorderStatuses: RecorderStatus[]; recorderStatusAvailable: boolean }) {
  const [frameText, setFrameText] = useState("0");
  const [state, setState] = useState("idle");
  const [lastFrame, setLastFrame] = useState<number | null>(null);
  const [nodeId, setNodeId] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [recorderId, setRecorderId] = useState("recorder-1");
  const [format, setFormat] = useState<import("@audiorouter/contracts").RecorderFileFormat>("wavPcm24");
  const [channels, setChannels] = useState<1 | 2>(2);
  const [sampleRate, setSampleRate] = useState<44100 | 48000>(48000);
  const [dither, setDither] = useState(true);
  const [busy, setBusy] = useState(false);
  const frame = Number.parseInt(frameText, 10);
  const validFrame = Number.isSafeInteger(frame) && frame >= 0;
  useEffect(() => {
    if (!recorderStatusAvailable) { setState("unavailable"); setLastFrame(null); setNodeId(null); return; }
    const status = recorderStatuses.find((item) => item.sessionId === sessionId);
    if (status) { setState(status.state); setLastFrame(status.lastFrame); setNodeId(status.nodeId ?? null); }
    else { setState("idle"); setLastFrame(null); setNodeId(null); }
  }, [recorderStatuses, recorderStatusAvailable, sessionId]);
  const create = async () => {
    if (!recorderId.trim()) { setMessage("Provide a recorder ID."); return; }
    if (busy || !connected) return;
    setBusy(true);
    setMessage("Creating an unarmed recorder...");
    try {
      const result = await backend.createRecorder({ sessionId, recorderId: recorderId.trim(), format, sequence: 1, channels, sampleRate, dither, queueCapacity: 8, maximumChunksPerPass: 1, idempotencyKey: uiIdempotencyKey("recorder-create") });
      setState(result.state);
      setMessage(`Recorder ${result.recorderId} created unarmed at ${result.path}.`);
    } catch (error) {
      setMessage(formatUiError(error, "Unable to create recorder."));
    } finally { setBusy(false); }
  };
  const run = async (action: string, operation: () => Promise<{ state: string; lastFrame?: number | null }>) => {
    if (busy || !connected) return;
    setBusy(true);
    setMessage(`${action}...`);
    try {
      const result = await operation();
      setState(result.state);
      if (result.lastFrame !== undefined) setLastFrame(result.lastFrame ?? null);
      setMessage(`Recorder ${result.state} at frame ${frame}.`);
    } catch (error) {
      setState("failed");
      setMessage(formatUiError(error, `Unable to ${action.toLowerCase()} recorder.`));
    } finally { setBusy(false); }
  };
  return <section className="panel recorder-actions" aria-labelledby="recorder-actions-heading">
    <div className="section-heading"><div><p className="eyebrow">Frame boundary control</p><h2 id="recorder-actions-heading">Recorder</h2></div><span className="badge">{state}</span></div>
    <fieldset disabled={!connected}><legend>Create unarmed recorder</legend><label>Recorder ID<input aria-label="Recorder ID" value={recorderId} onChange={(event) => setRecorderId(event.target.value)} /></label><label>Format<select aria-label="Recorder format" value={format} onChange={(event) => { const next = event.target.value as typeof format; setFormat(next); if (next === "wavFloat32") setDither(false); }}><option value="wavPcm24">WAV PCM24</option><option value="wavPcm16">WAV PCM16</option><option value="wavFloat32">WAV Float32</option><option value="flac16">FLAC 16</option><option value="flac24">FLAC 24</option></select></label><label>Channels<select aria-label="Recorder channels" value={channels} onChange={(event) => setChannels(Number(event.target.value) as 1 | 2)}><option value={1}>Mono</option><option value={2}>Stereo</option></select></label><label>Sample rate<select aria-label="Recorder sample rate" value={sampleRate} onChange={(event) => setSampleRate(Number(event.target.value) as 44100 | 48000)}><option value={48000}>48 kHz</option><option value={44100}>44.1 kHz</option></select></label><label>TPDF dither<input aria-label="TPDF dither" type="checkbox" checked={dither} disabled={format === "wavFloat32"} onChange={(event) => setDither(event.target.checked)} /></label>{format === "wavFloat32" && <small>Float32 output is not dithered.</small>}<button type="button" className="secondary" onClick={() => void create()}>Create recorder</button></fieldset>
    <label>Engine frame<input aria-label="Recorder engine frame" inputMode="numeric" value={frameText} onChange={(event) => setFrameText(event.target.value)} disabled={!connected} /></label>
    <div className="actions">
      <button type="button" className="secondary" onClick={() => void run("Arming", () => backend.armRecorder(sessionId, uiIdempotencyKey("recorder-arm")))} disabled={!connected}>Arm</button>
      <button type="button" className="secondary" onClick={() => void run("Starting", () => backend.startRecorder(sessionId, frame, uiIdempotencyKey("recorder-start")))} disabled={!connected || !validFrame}>Start</button>
      <button type="button" className="secondary" onClick={() => void run("Pausing", () => backend.pauseRecorder(sessionId, frame, uiIdempotencyKey("recorder-pause")))} disabled={!connected || !validFrame}>Pause</button>
      <button type="button" className="secondary" onClick={() => void run("Resuming", () => backend.resumeRecorder(sessionId, frame, uiIdempotencyKey("recorder-resume")))} disabled={!connected || !validFrame}>Resume</button>
      <button type="button" className="secondary" onClick={() => void run("Splitting", () => backend.splitRecorder(sessionId, frame, uiIdempotencyKey("recorder-split")))} disabled={!connected || !validFrame}>Split</button>
      <button type="button" className="secondary" onClick={() => void run("Stopping", () => backend.stopRecorder(sessionId, frame, uiIdempotencyKey("recorder-stop")))} disabled={!connected || !validFrame}>Stop</button>
    </div>
    {message && <p className="muted" role="status">{message}</p>}
    {nodeId && <p className="muted" role="status">Attached recorder node: {nodeId}</p>}
    {lastFrame !== null && <p className="muted" role="status">Backend last frame: {lastFrame}</p>}
    <p className="muted">Actions are sent only to a connected backend and use explicit engine frame boundaries. The preview backend never arms or starts recording.</p>
  </section>;
}

function RecordingActions({ recordings, connected, busy, onRename, onReveal, onRecycle }: { recordings: import("@audiorouter/contracts").RecordingRow[]; connected: boolean; busy: boolean; onRename: (recordingId: string, newPath: string) => Promise<void>; onReveal: (recordingId: string) => Promise<void>; onRecycle: (recordingId: string, confirm: boolean) => Promise<void> }) {
  const [selectedId, setSelectedId] = useState("");
  const [newPath, setNewPath] = useState("");
  const selected = recordings.find((recording) => recording.id === selectedId) ?? recordings[0];
  useEffect(() => { if (selected) { setSelectedId(selected.id); setNewPath(selected.path); } else { setSelectedId(""); setNewPath(""); } }, [selected?.id, selected?.path]);
  if (!selected) return null;
  return <section className="panel recording-actions" aria-labelledby="recording-actions-heading"><div className="section-heading"><div><p className="eyebrow">File actions</p><h2 id="recording-actions-heading">Recording operations</h2></div><span className="badge">explicit</span></div><label>Recording<select aria-label="Recording file action target" value={selected.id} onChange={(event) => { const next = recordings.find((recording) => recording.id === event.target.value); setSelectedId(event.target.value); setNewPath(next?.path ?? ""); }} disabled={busy}>{recordings.map((recording) => <option key={recording.id} value={recording.id}>{recording.title || recording.id} · {recording.state}</option>)}</select></label><label>New path<input type="text" value={newPath} onChange={(event) => setNewPath(event.target.value)} disabled={!connected || busy} /></label><div className="actions"><button type="button" className="secondary" onClick={() => void onRename(selected.id, newPath)} disabled={!connected || busy || !newPath.trim() || newPath === selected.path}>Rename in approved directory</button><button type="button" className="secondary" onClick={() => void onReveal(selected.id)} disabled={!connected || busy}>Reveal</button><button type="button" className="secondary" onClick={() => void onRecycle(selected.id, false)} disabled={!connected || busy}>Preview recycle</button><button type="button" className="secondary" onClick={() => { if (window.confirm("Recycle this recording through the operating system?")) void onRecycle(selected.id, true); }} disabled={!connected || busy}>Recycle recording</button></div><p className="muted">Rename is restricted by the backend to the approved directory. Reveal and recycle never run while disconnected; recycle requires an explicit confirmation.</p></section>;
}

function VirtualDeviceLifecyclePanel({ backend }: { backend: UiBackend }) {
  const [devices, setDevices] = useState<import("@audiorouter/contracts").VirtualDeviceInfo[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [action, setAction] = useState<"create" | "rename" | "setEnabled" | "delete">("create");
  const [busId, setBusId] = useState("virtual-bus");
  const [busName, setBusName] = useState("AudioRouter Bus");
  const [plan, setPlan] = useState<import("@audiorouter/contracts").VirtualDevicePlanResult | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const refreshGeneration = useRef(0);
  const refresh = () => { const generation = ++refreshGeneration.current; setPlan(null); void backend.listVirtualDevices().then((items) => { if (generation !== refreshGeneration.current) return; setDevices(items); if (!items.some((item) => item.id === selectedId)) { const next = items[0]; setSelectedId(next?.id ?? ""); setBusId(next?.id ?? "virtual-bus"); setBusName(next?.name ?? "AudioRouter Bus"); } }).catch((error) => { if (generation === refreshGeneration.current) setMessage(formatUiError(error, "Virtual-device inventory unavailable.")); }); };
  useEffect(() => { if (backend.connected) refresh(); else { setDevices([]); setPlan(null); } }, [backend, backend.connected]);
  const selected = devices.find((device) => device.id === selectedId);
  const chooseAction = (next: "create" | "rename" | "setEnabled" | "delete") => { setAction(next); setPlan(null); if (next === "create") { setBusId("virtual-bus"); setBusName("AudioRouter Bus"); } else if (selected) { setBusId(selected.id); setBusName(selected.name); } };
  const createPlan = async () => { if (busy || !backend.connected) return; const operation: import("@audiorouter/contracts").VirtualDeviceOperation = action === "create" ? { action, id: busId.trim(), name: busName.trim() } : action === "rename" ? { action, id: selectedId, name: busName.trim() } : action === "setEnabled" ? { action, id: selectedId, enabled: !(selected?.enabled ?? false) } : { action, id: selectedId }; if (!operation.id || ("name" in operation && !operation.name)) { setMessage("Provide a valid bus ID and name."); return; } setBusy(true); setMessage("Planning managed virtual-bus change..."); try { const result = await backend.planVirtualDevice(operation); setPlan(result); setMessage(result.availability.reason); } catch (error) { setMessage(formatUiError(error, "Unable to plan managed virtual-bus change.")); } finally { setBusy(false); } };
  const applyPlan = async () => { if (!plan || busy || !backend.connected) return; setBusy(true); setMessage("Applying desired virtual-bus state..."); try { const result = await backend.applyVirtualDevice(plan.planId, uiIdempotencyKey("virtual-device-apply")); setPlan(null); setMessage(`${result.availability.reason}; no native endpoint is active in this build.`); refresh(); } catch (error) { setMessage(formatUiError(error, "Unable to apply virtual-bus plan.")); } finally { setBusy(false); } };
  return <section className="panel virtual-device-lifecycle" aria-labelledby="virtual-device-lifecycle-heading"><div className="section-heading"><div><p className="eyebrow">Managed buses</p><h2 id="virtual-device-lifecycle-heading">Virtual-device lifecycle</h2></div><button type="button" className="secondary" onClick={refresh} disabled={!backend.connected || busy}>Refresh</button></div>{devices.length === 0 ? <p className="muted">No managed virtual buses are currently listed.</p> : <ul aria-label="Managed virtual devices">{devices.map((device) => <li key={device.id}><strong>{device.name}</strong> <small>{device.enabled ? "enabled" : "disabled"} · {device.availability.reason} · lease {device.leaseOwner ?? "none"}</small></li>)}</ul>}<fieldset disabled={!backend.connected || busy}><legend>Desired-state operation</legend><label>Action<select aria-label="Virtual-device action" value={action} onChange={(event) => chooseAction(event.target.value as typeof action)}><option value="create">Create</option><option value="rename" disabled={!selected}>Rename</option><option value="setEnabled" disabled={!selected}>Enable/disable</option><option value="delete" disabled={!selected}>Delete</option></select></label>{action !== "create" && <label>Existing bus<select aria-label="Existing virtual-device target" value={selectedId} onChange={(event) => { const next = devices.find((device) => device.id === event.target.value); setSelectedId(event.target.value); setBusName(next?.name ?? ""); }}>{devices.map((device) => <option key={device.id} value={device.id}>{device.name}</option>)}</select></label>}<label>Bus ID<input value={busId} maxLength={64} disabled={action !== "create"} onChange={(event) => setBusId(event.target.value)} /></label>{action !== "delete" && <label>Bus name<input value={busName} maxLength={120} disabled={action === "setEnabled"} onChange={(event) => setBusName(event.target.value)} /></label>}<button type="button" className="secondary" onClick={() => void createPlan()} disabled={!backend.connected || busy}>Plan {action}</button>{plan && <button type="button" className="secondary" onClick={() => void applyPlan()} disabled={!backend.connected || busy}>Apply planned state</button>}</fieldset>{message && <p className="muted" role="status">{message}</p>}<p className="muted">Every change requires explicit plan/apply and <code>deviceAdministration</code>. The unavailable managed driver means no endpoint is synthesized or activated.</p></section>;
}

function VirtualRoutePanel({ backend }: { backend: UiBackend }) {
  const [state, setState] = useState<import("@audiorouter/contracts").VirtualRouteListResult | null>(null);
  const [routeText, setRouteText] = useState("[]");
  const [revisionText, setRevisionText] = useState("0");
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const refreshGeneration = useRef(0);
  const refresh = () => {
    const generation = ++refreshGeneration.current;
    void backend.listVirtualRoutes().then((result) => {
      if (generation !== refreshGeneration.current) return;
      setState(result);
      setRouteText(JSON.stringify(result.routes, null, 2));
      setRevisionText(String(result.revision));
      setMessage(null);
    }).catch((error) => { if (generation === refreshGeneration.current) setMessage(formatUiError(error, "Virtual-route inventory unavailable.")); });
  };
  useEffect(() => {
    if (backend.connected) refresh();
    else {
      setState(null);
      setRouteText("[]");
      setRevisionText("0");
      setMessage(null);
    }
  }, [backend, backend.connected]);
  const replace = async () => {
    if (busy || !backend.connected) return;
    const baseRevision = Number.parseInt(revisionText, 10);
    if (!Number.isSafeInteger(baseRevision) || baseRevision < 0) {
      setMessage("Base revision must be a non-negative integer.");
      return;
    }
    let routes: import("@audiorouter/contracts").VirtualBusRoute[];
    try {
      const parsed: unknown = JSON.parse(routeText);
      if (!Array.isArray(parsed)) throw new Error("route JSON must be an array");
      routes = parsed as import("@audiorouter/contracts").VirtualBusRoute[];
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Route JSON is invalid.");
      return;
    }
    setBusy(true); setMessage("Replacing explicit virtual-bus routes...");
    try {
      const result = await backend.replaceVirtualRoutes(baseRevision, routes, uiIdempotencyKey("virtual-routes-replace"));
      setState({ revision: result.revision, routes: result.routes });
      setRouteText(JSON.stringify(result.routes, null, 2));
      setRevisionText(String(result.revision));
      setMessage(`Virtual routes ${result.state} at revision ${result.revision}.`);
    } catch (error) {
      setMessage(formatUiError(error, "Unable to replace virtual-bus routes."));
    } finally { setBusy(false); }
  };
  return <section className="panel virtual-route-panel" aria-labelledby="virtual-route-heading"><div className="section-heading"><div><p className="eyebrow">Explicit cross-session routing</p><h2 id="virtual-route-heading">Virtual-bus routes</h2></div><div className="actions"><span className="badge">rev {state?.revision ?? "-"}</span><button type="button" className="secondary" onClick={refresh} disabled={!backend.connected || busy}>Refresh</button></div></div><p className="muted">Routes are replaced as one revisioned document. The backend validates bus identities, sessions, cycles, authorization, and idempotency before changing desired state.</p>{state?.routes.length ? <ul aria-label="Explicit cross-session routes">{state.routes.map((route) => <li key={`${route.busId}-${route.producerSessionId}-${route.consumerSessionId}`}><code>{route.busId}</code> · {route.producerSessionId} → {route.consumerSessionId}</li>)}</ul> : <p className="muted">No explicit cross-session routes are currently listed.</p>}<fieldset disabled={!backend.connected || busy}><legend>Revisioned replacement</legend><label>Base revision<input aria-label="Virtual-route base revision" inputMode="numeric" value={revisionText} onChange={(event) => setRevisionText(event.target.value)} /></label><label>Routes JSON<textarea aria-label="Virtual-route JSON" value={routeText} onChange={(event) => setRouteText(event.target.value)} rows={6} spellCheck={false} /></label><button type="button" className="secondary" onClick={() => void replace()} disabled={!backend.connected || busy}>Replace routes</button></fieldset>{message && <p className="muted" role="status" aria-live="polite">{message}</p>}<p className="muted">Disconnected preview mode never mutates route state. Replacement requires the backend’s <code>deviceAdministration</code> permission and does not activate endpoints by itself.</p></section>;
}

function endpointBindingStorageKey(sessionId: string) {
  return `audiorouter.ui.endpoint-binding.${sessionId}`;
}

function readEndpointBindingHint(sessionId: string): { captureEndpointId?: string; renderEndpointId?: string } {
  try {
    const value: unknown = JSON.parse(window.localStorage.getItem(endpointBindingStorageKey(sessionId)) ?? "null");
    if (!value || typeof value !== "object") return {};
    const record = value as Record<string, unknown>;
    return {
      captureEndpointId: typeof record.captureEndpointId === "string" ? record.captureEndpointId.slice(0, 32768) : undefined,
      renderEndpointId: typeof record.renderEndpointId === "string" ? record.renderEndpointId.slice(0, 32768) : undefined,
    };
  } catch {
    return {};
  }
}

function writeEndpointBindingHint(sessionId: string, captureEndpointId: string, renderEndpointId: string) {
  try {
    window.localStorage.setItem(endpointBindingStorageKey(sessionId), JSON.stringify({ captureEndpointId, renderEndpointId }));
  } catch {
    // Local presentation persistence is best effort and never blocks routing.
  }
}

/**
 * Return a pair only when the read-only inventory contains one unambiguous
 * active VB-Cable capture/render endpoint.  Friendly names are used only as
 * a user-facing convenience; the returned values are still the exact stable
 * endpoint IDs sent to the backend.
 */
export function findVbCableEndpointPair(devices: DeviceListItem[]): { captureEndpointId: string; renderEndpointId: string } | null {
  const isVbCable = (name: string) => {
    const normalized = name.toLocaleLowerCase();
    return normalized.includes("vb-audio") || normalized.includes("vb audio") || normalized.includes("vb-cable") || normalized.includes("vb cable");
  };
  const capture = devices.filter((device) => device.id === findVbCableCaptureEndpointId(devices));
  const render = devices.filter((device) => device.state === "active" && device.direction === "render" && isVbCable(device.name) && device.name.toLocaleLowerCase().includes("cable input"));
  return capture.length === 1 && render.length === 1 ? { captureEndpointId: capture[0].id, renderEndpointId: render[0].id } : null;
}

/** Return the unambiguous active VB-Cable capture endpoint, if present. */
export function findVbCableCaptureEndpointId(devices: DeviceListItem[]): string | null {
  const capture = devices.filter((device) => {
    const normalized = device.name.toLocaleLowerCase();
    return device.state === "active" && device.direction === "capture" &&
      (normalized.includes("vb-audio") || normalized.includes("vb audio") || normalized.includes("vb-cable") || normalized.includes("vb cable")) &&
      normalized.includes("cable output");
  });
  return capture.length === 1 ? capture[0].id : null;
}

function NativeEndpointPanel({ backend, sessionId, devices, sessionRunning, onStart, onStop }: { backend: UiBackend; sessionId: string; devices: DeviceListItem[]; sessionRunning: boolean; onStart: () => Promise<void>; onStop: () => Promise<void> }) {
  const activeCapture = devices.filter((device): device is Extract<DeviceListItem, { state: "active" }> => device.state === "active" && device.direction === "capture");
  const activeRender = devices
    .filter((device): device is Extract<DeviceListItem, { state: "active" }> => device.state === "active" && device.direction === "render");
  const savedHint = readEndpointBindingHint(sessionId);
  const missingCaptureHint = devices.length > 0 && Boolean(savedHint.captureEndpointId) && !activeCapture.some((device) => device.id === savedHint.captureEndpointId);
  const missingRenderHint = devices.length > 0 && Boolean(savedHint.renderEndpointId) && !activeRender.some((device) => device.id === savedHint.renderEndpointId);
  const vbCablePair = findVbCableEndpointPair(devices);
  const vbCableCaptureEndpointId = findVbCableCaptureEndpointId(devices);
  const [captureEndpointId, setCaptureEndpointId] = useState(() => readEndpointBindingHint(sessionId).captureEndpointId ?? "");
  const [renderEndpointId, setRenderEndpointId] = useState(() => readEndpointBindingHint(sessionId).renderEndpointId ?? "");
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    const hint = readEndpointBindingHint(sessionId);
    setCaptureEndpointId(hint.captureEndpointId ?? "");
    setRenderEndpointId(hint.renderEndpointId ?? "");
  }, [sessionId]);
  useEffect(() => {
    if (devices.length === 0) return;
    // An empty value with a saved hint is deliberate: it represents a stale
    // binding awaiting replacement. Do not let an inventory effect scheduled
    // before a button click overwrite a newly selected pair.
    if (captureEndpointId && !activeCapture.some((device) => device.id === captureEndpointId)) setCaptureEndpointId("");
    if (renderEndpointId && !activeRender.some((device) => device.id === renderEndpointId)) setRenderEndpointId("");
  }, [devices, sessionId, captureEndpointId, renderEndpointId]);
  const prepare = async () => {
    if (busy || !backend.connected) return;
    if (!backend.prepareNativeEndpoint) { setMessage("Native endpoint preparation is unavailable in this backend."); return; }
    if (!captureEndpointId || !renderEndpointId) { setMessage("Select both an active capture and render endpoint."); return; }
    setBusy(true); setMessage("Preparing exact endpoints in stopped state...");
    try { const result = await backend.prepareNativeEndpoint(sessionId, captureEndpointId, renderEndpointId); setMessage(`Prepared ${result.state}; start the session to activate audio.`); }
    catch (error) { setMessage(formatUiError(error, "Native endpoint preparation failed.")); }
    finally { setBusy(false); }
  };
  const detach = async () => {
    if (busy || !backend.connected) return;
    if (!backend.detachNativeEndpoint) { setMessage("Native endpoint detachment is unavailable in this backend."); return; }
    setBusy(true); setMessage("Detaching the stopped native worker...");
    try { const result = await backend.detachNativeEndpoint(sessionId); setMessage(`Native worker ${result.state}; select new endpoints before preparing again.`); }
    catch (error) { setMessage(formatUiError(error, "Native endpoint detachment failed.")); }
    finally { setBusy(false); }
  };
  const selectVbCable = () => {
    if (!vbCablePair) { setMessage("An unambiguous active VB-Cable input/output pair was not found."); return; }
    setCaptureEndpointId(vbCablePair.captureEndpointId);
    setRenderEndpointId(vbCablePair.renderEndpointId);
    writeEndpointBindingHint(sessionId, vbCablePair.captureEndpointId, vbCablePair.renderEndpointId);
    setMessage("VB-Cable pair selected. Review the graph, then prepare and start the session.");
  };
  const selectVbCableCapture = () => {
    if (!vbCableCaptureEndpointId) { setMessage("An unambiguous active VB-Cable capture endpoint was not found."); return; }
    setCaptureEndpointId(vbCableCaptureEndpointId);
    writeEndpointBindingHint(sessionId, vbCableCaptureEndpointId, renderEndpointId);
    setMessage("VB-Cable capture selected. Choose the physical render output, then prepare and start the session.");
  };
  return <section id="native-endpoint-panel" className="panel native-endpoint-panel" aria-labelledby="native-endpoint-heading"><div className="section-heading"><div><p className="eyebrow">Native adapter</p><h2 id="native-endpoint-heading">Endpoint binding</h2></div><span className="badge">{sessionRunning ? "running" : "stopped"}</span></div><p className="muted">Select exact active endpoints for this session. Preparation opens stopped clients only; start the session when the graph is ready to move audio.</p>{missingCaptureHint && <p className="muted" role="status">Saved capture endpoint is unavailable. Select a replacement deliberately.</p>}{missingRenderHint && <p className="muted" role="status">Saved render endpoint is unavailable. Select a replacement deliberately.</p>}<div className="actions"><button type="button" className="secondary" onClick={selectVbCableCapture} disabled={!backend.connected || !vbCableCaptureEndpointId || sessionRunning} title={vbCableCaptureEndpointId ? "Select the exact active VB-Cable capture endpoint" : "No unambiguous active VB-Cable capture endpoint found"}>Select VB-Cable capture</button><button type="button" className="secondary" onClick={selectVbCable} disabled={!backend.connected || !vbCablePair || sessionRunning} title={vbCablePair ? "Select the exact active VB-Cable loopback endpoints" : "No unambiguous active VB-Cable loopback pair found"}>Select VB-Cable loopback pair</button>{vbCablePair && <small>Active loopback pair detected</small>}</div><label>Capture endpoint<select aria-label="Native capture endpoint" value={captureEndpointId} disabled={!backend.connected || activeCapture.length === 0 || sessionRunning} onChange={(event) => { setCaptureEndpointId(event.target.value); writeEndpointBindingHint(sessionId, event.target.value, renderEndpointId); }}><option value="">Select capture endpoint</option>{activeCapture.map((device) => <option key={device.id} value={device.id}>{device.name} · {device.format.sampleRateHz} Hz · {device.format.channels} ch</option>)}</select></label><label>Render endpoint<select aria-label="Native render endpoint" value={renderEndpointId} disabled={!backend.connected || activeRender.length === 0 || sessionRunning} onChange={(event) => { setRenderEndpointId(event.target.value); writeEndpointBindingHint(sessionId, captureEndpointId, event.target.value); }}><option value="">Select render endpoint</option>{activeRender.map((device) => <option key={device.id} value={device.id}>{device.name} · {device.format.sampleRateHz} Hz · {device.format.channels} ch</option>)}</select></label><div className="actions"><button type="button" className="secondary" onClick={() => void prepare()} disabled={!backend.connected || !backend.prepareNativeEndpoint || !captureEndpointId || !renderEndpointId || sessionRunning}>Prepare native endpoints</button><button type="button" className="secondary" onClick={() => void detach()} disabled={!backend.connected || !backend.detachNativeEndpoint || sessionRunning}>Detach stopped worker</button><button type="button" className={sessionRunning ? "secondary" : "primary"} onClick={() => void (sessionRunning ? onStop() : onStart())} disabled={!backend.connected}>{sessionRunning ? "Stop session" : "Start session"}</button></div>{message && <p className="muted" role="status" aria-live="polite">{message}</p>}<p className="muted">Requires the authenticated <code>deviceAdministration</code> scope for preparation and detachment. Endpoint defaults, volume, and mute are never changed. Selected IDs are retained only as local UI hints; a missing saved ID stays unselected until you deliberately choose a replacement. Use the loopback action only for a deliberate CABLE Input → CABLE Output test; for normal monitoring choose a physical render endpoint.</p></section>;
}

function NodeCard({ node, selected, onSelect }: { node: Node; selected: boolean; onSelect: () => void }) {
  return <article className={`node-card${selected ? " selected" : ""}`} tabIndex={0} aria-label={`${node.name}, ${node.kind}`} aria-current={selected ? "true" : undefined} onClick={onSelect} onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); onSelect(); } }}><span className="node-kind">{node.kind}</span><h3>{node.name}</h3><p>{node.ports.length} port{node.ports.length === 1 ? "" : "s"} - {nodeStateLabel(node)}</p><div className="port-list">{node.ports.map((port) => <span key={port.name} className={`port ${port.direction}`}>{port.direction}: {port.name} - {port.channels}ch</span>)}</div></article>;
}

function LegacyNodeList({ session, selectedNodeId, onSelect, onRemoveConnection, onToggleConnection }: { session: import("@audiorouter/contracts").Session; selectedNodeId: string; onSelect: (id: string) => void; onRemoveConnection: (id: string) => void; onToggleConnection: (id: string, enabled: boolean) => void }) {
  const names = new Map(session.nodes.map((node) => [node.id, node.name]));
  return <div className="graph-list" aria-label="Graph nodes and connections"><ol aria-label="Nodes">{session.nodes.map((node) => <li key={node.id}><button type="button" className={node.id === selectedNodeId ? "selected" : ""} aria-current={node.id === selectedNodeId ? "true" : undefined} onClick={() => onSelect(node.id)}>{node.name} <small>{node.kind}, {node.enabled ? "enabled" : "disabled"}</small><span className="list-port-summary">{nodePortLabels(node).join(" · ")}</span></button></li>)}</ol><h3>Connections</h3>{session.edges.length === 0 ? <p className="muted">No committed connections.</p> : <ul aria-label="Connections">{session.edges.map((edge) => <li key={edge.id}><span>{names.get(edge.sourceNode) ?? edge.sourceNode}:{edge.sourcePort} → {names.get(edge.destinationNode) ?? edge.destinationNode}:{edge.destinationPort} <small>{edge.enabled ? "enabled" : "disabled"}</small></span><button type="button" className="secondary" onClick={() => onToggleConnection(edge.id, !edge.enabled)}>{edge.enabled ? "Disable" : "Enable"}</button><button type="button" className="secondary" onClick={() => onRemoveConnection(edge.id)}>Remove</button></li>)}</ul>}</div>;
}

function LegacyDraftConnectionList({ session, onRemove, onToggle }: { session: import("@audiorouter/contracts").Session; onRemove: (id: string) => void; onToggle: (id: string, enabled: boolean) => void }) {
  const names = new Map(session.nodes.map((node) => [node.id, node.name]));
  return <section className="draft-connections" aria-labelledby="draft-connections-heading"><h3 id="draft-connections-heading">Draft connections</h3>{session.edges.length === 0 ? <p className="muted">No draft connections.</p> : <ul aria-label="Draft connections">{session.edges.map((edge) => <li key={edge.id}><span>{names.get(edge.sourceNode) ?? edge.sourceNode}:{edge.sourcePort} → {names.get(edge.destinationNode) ?? edge.destinationNode}:{edge.destinationPort} <small>{edge.enabled ? "enabled" : "disabled"}</small></span><button type="button" className="secondary" onClick={() => onToggle(edge.id, !edge.enabled)}>{edge.enabled ? "Disable" : "Enable"}</button><button type="button" className="secondary" onClick={() => onRemove(edge.id)}>Remove</button></li>)}</ul>}</section>;
}

function AppContent({ backend = defaultBackend }: { backend?: UiBackend } = {}) {
  const [snapshotCache] = useState(() => new SnapshotCache());
  const [snapshotState, setSnapshotState] = useState(snapshotCache.current());
  const [selectedSessionId, setSelectedSessionId] = useState(demoSession.id);
  const [selectedNodeId, setSelectedNodeId] = useState(demoSession.nodes[0].id);
  const [selectedNodeIds, setSelectedNodeIdsState] = useState<string[]>([demoSession.nodes[0].id]);
  const setSelectedNodeIds = (ids: string[]) => { setSelectedNodeIdsState((current) => current.length === ids.length && current.every((id, index) => id === ids[index]) ? current : ids); };
  const [draft, setDraft] = useState(demoSession);
  const [draftHistory, setDraftHistory] = useState<DraftHistory>({ past: [], future: [] });
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [routeInspection, setRouteInspection] = useState<RouteInspection | null>(null);
  const routeInspectionRequest = useRef(0);
  const [recordings, setRecordings] = useState<import("@audiorouter/contracts").RecordingRow[]>([]);
  const [recorderStatuses, setRecorderStatuses] = useState<RecorderStatus[]>([]);
  const [recorderStatusAvailable, setRecorderStatusAvailable] = useState(!backend.connected);
  const [applications, setApplications] = useState<ApplicationRow[]>([]);
  const [devices, setDevices] = useState<DeviceListItem[]>([]);
  const [devicesError, setDevicesError] = useState<string | null>(null);
  const [applicationsError, setApplicationsError] = useState<string | null>(null);
  const [applicationCaptureMode, setApplicationCaptureMode] = useState<"include" | "exclude">("include");
  const [recordingsError, setRecordingsError] = useState<string | null>(null);
  const [previewMessage, setPreviewMessage] = useState<string | null>(null);
  const [recoveryMessage, setRecoveryMessage] = useState<string | null>(null);
  const previewRequest = useRef(0);
  const recoveryRequest = useRef(0);
  const [metadataTitles, setMetadataTitles] = useState<Record<string, string>>({});
  const [metadataArtists, setMetadataArtists] = useState<Record<string, string>>({});
  const [metadataComments, setMetadataComments] = useState<Record<string, string>>({});
  const [recordingSearch, setRecordingSearch] = useState("");
  const [pendingWarnings, setPendingWarnings] = useState<string[]>([]);
  const [acknowledgedWarnings, setAcknowledgedWarnings] = useState<Set<string>>(() => new Set());
  const [pendingOperation, setPendingOperation] = useState<string | null>(null);
  const [graphBusy, setGraphBusy] = useState(false);
  const sessionBusy = useRef(false);
  const [sessionActionBusy, setSessionActionBusy] = useState(false);
  const sessionCrudBusy = useRef(false);
  const [sessionCrudBusyState, setSessionCrudBusyState] = useState(false);
  const safetyActionBusy = useRef(false);
  const [safetyActionBusyState, setSafetyActionBusyState] = useState(false);
  const recordingMutationBusy = useRef(false);
  const [recordingMutationBusyState, setRecordingMutationBusyState] = useState(false);
  const [privacyMuted, setPrivacyMuted] = useState(true);
  const [listView, setListView] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<TemplateId>("gaming-discord");
  const [librarySearch, setLibrarySearch] = useState("");
  const [processors, setProcessors] = useState<ProcessorDescriptor[] | null>(null);
  const [processorError, setProcessorError] = useState<string | null>(null);
  const [pluginParameters, setPluginParameters] = useState<PluginParametersResult | null>(null);
  const [pluginParameterError, setPluginParameterError] = useState<string | null>(null);
  useEffect(() => {
    // Keep drafts created by older UI snapshots compatible with the current
    // template identifier before the select is rendered again.
    if ((selectedTemplate as string) === "mix-minus conversation") setSelectedTemplate("mix-minus");
  }, [selectedTemplate]);
  const [presets, setPresets] = useState<import("@audiorouter/contracts").DiscoveryDocument["presets"] | null>(null);
  const [presetError, setPresetError] = useState<string | null>(null);
  const [theme, setTheme] = useState<ThemeMode>(() => readTheme(typeof window === "undefined" ? null : window.localStorage));
  const [compactStatus, setCompactStatus] = useState(() => readCompactStatus(typeof window === "undefined" ? null : window.localStorage));
  const [shortcuts, setShortcuts] = useState<ShortcutBinding>(() => readShortcuts(typeof window === "undefined" ? null : window.localStorage, defaultShortcutBinding));
  const [shortcutMessage, setShortcutMessage] = useState<string | null>(null);
  const [connectionSource, setConnectionSource] = useState("");
  const [connectionDestination, setConnectionDestination] = useState("");
  const [connectionDialogOpen, setConnectionDialogOpen] = useState(false);
  const connectionDialogReturnFocus = useRef<HTMLElement | null>(null);
  const connectionDialog = useRef<HTMLElement | null>(null);
  const connectionDialogSource = useRef<HTMLSelectElement | null>(null);
  const [createdSessions, setCreatedSessions] = useState<import("@audiorouter/contracts").Session[]>([]);
  const [listedSessions, setListedSessions] = useState<import("@audiorouter/contracts").Session[]>(backend.connected ? [] : demoSessions);
  const [sessionInventoryError, setSessionInventoryError] = useState<string | null>(null);
  const [nativeGeneration, setNativeGeneration] = useState<number | null>(null);
  const [nativePumpStats, setNativePumpStats] = useState<NativePumpStats | null>(null);
  const eventCursor = useRef({ backendEpoch: 0, sequence: 0 });
  const applicationRefreshGeneration = useRef(0);
  const deviceRefreshGeneration = useRef(0);
  const sessionRefreshGeneration = useRef(0);
  const recordingRefreshGeneration = useRef(0);
  const recorderRefreshGeneration = useRef(0);
  useEffect(() => { let mounted = true; void snapshotCache.refresh(backend).then((nextState) => { if (mounted) { setSnapshotState(nextState); if (nextState.snapshot) eventCursor.current = { backendEpoch: nextState.snapshot.status.eventCursor.backendEpoch, sequence: nextState.snapshot.status.eventCursor.latestSequence }; } }); return () => { mounted = false; }; }, [backend, snapshotCache]);
  const refreshApplications = () => {
    const generation = ++applicationRefreshGeneration.current;
    void backend.listApplications().then((items) => { if (generation !== applicationRefreshGeneration.current) return; setApplications(items); setApplicationsError(null); }).catch((error) => { if (generation !== applicationRefreshGeneration.current) return; setApplications([]); setApplicationsError(formatUiError(error, "Application inventory unavailable")); });
  };
  const refreshDevices = () => {
    const generation = ++deviceRefreshGeneration.current;
    void backend.listDevices(true).then((items) => { if (generation !== deviceRefreshGeneration.current) return; setDevices(items); setDevicesError(null); }).catch((error) => { if (generation !== deviceRefreshGeneration.current) return; setDevices([]); setDevicesError(formatUiError(error, "Device inventory unavailable")); });
  };
  const refreshSessions = () => {
    const generation = ++sessionRefreshGeneration.current;
    void backend.listSessions().then((items) => { if (generation !== sessionRefreshGeneration.current) return; setListedSessions(items); setSessionInventoryError(null); }).catch((error) => { if (generation !== sessionRefreshGeneration.current) return; setSessionInventoryError(formatUiError(error, "Session inventory unavailable")); if (!backend.connected) setListedSessions(demoSessions); else setListedSessions([]); });
  };
  const refreshRecordings = (sessionId: string) => {
    const generation = ++recordingRefreshGeneration.current;
    void backend.listRecordings(sessionId).then((items) => { if (generation !== recordingRefreshGeneration.current) return; setRecordings(items); setRecordingsError(null); }).catch((error) => { if (generation !== recordingRefreshGeneration.current) return; setRecordings([]); setRecordingsError(formatUiError(error, "Recording library unavailable")); });
  };
  const refreshRecorders = () => {
    const generation = ++recorderRefreshGeneration.current;
    void backend.listRecorders().then((items) => { if (generation !== recorderRefreshGeneration.current) return; setRecorderStatuses(items); setRecorderStatusAvailable(true); }).catch(() => { if (generation !== recorderRefreshGeneration.current) return; setRecorderStatuses([]); setRecorderStatusAvailable(false); });
  };
  const refresh = () => {
    void snapshotCache.refresh(backend).then(setSnapshotState);
    refreshSessions();
    refreshRecordings(session.id);
    refreshRecorders();
    refreshApplications();
    refreshDevices();
  };
  const snapshot = snapshotState.snapshot;
  useEffect(() => { writeTheme(typeof window === "undefined" ? null : window.localStorage, theme); }, [theme]);
  useEffect(() => { writeCompactStatus(typeof window === "undefined" ? null : window.localStorage, compactStatus); }, [compactStatus]);
  useEffect(() => { writeShortcuts(typeof window === "undefined" ? null : window.localStorage, shortcuts); }, [shortcuts]);
  useEffect(() => { if (snapshot) setPrivacyMuted(snapshot.status.privacyMute.muted); }, [snapshot]);
  const availableSessions = mergeSessionInventory(listedSessions, snapshot?.session ?? null, createdSessions);
  const session = availableSessions.find((item) => item.id === selectedSessionId) ?? availableSessions[0] ?? demoSession;
  const selectedNode = draft.nodes.find((node) => node.id === selectedNodeId) ?? draft.nodes[0];
  const sessionRunning = snapshot?.status.activeSessionIds.includes(session.id) ?? false;
  useEffect(() => { setDraft(session); setDraftHistory({ past: [], future: [] }); setSelectedNodeId(session.nodes[0]?.id ?? ""); setSelectedNodeIds(session.nodes[0] ? [session.nodes[0].id] : []); setConnectionSource(""); setConnectionDestination(""); setActionMessage(null); setRouteInspection(null); setPendingWarnings([]); setAcknowledgedWarnings(new Set()); setPendingOperation(null); }, [session]);
  useEffect(() => {
    let active = true;
    void backend.listPresets().then((items) => { if (active) { setPresets(items); setPresetError(null); } }).catch((error) => { if (active) { setPresets(null); setPresetError(formatUiError(error, "Preset catalog unavailable")); } });
    return () => { active = false; };
  }, [backend]);
  useEffect(() => {
    let active = true;
    void backend.listProcessors().then((items) => { if (active) { setProcessors(items); setProcessorError(null); } }).catch((error) => { if (active) { setProcessors(null); setProcessorError(formatUiError(error, "Processor catalog unavailable")); } });
    return () => { active = false; };
  }, [backend]);
  useEffect(() => {
    if (selectedNode?.kind !== "plugin" || !backend.connected) { setPluginParameters(null); setPluginParameterError(null); return; }
    const path = selectedNode.parameters.path;
    const fingerprint = selectedNode.parameters.fingerprint;
    if (typeof path !== "string" || typeof fingerprint !== "string") { setPluginParameters(null); setPluginParameterError("The plugin placeholder is missing its verified path or fingerprint."); return; }
    let active = true;
    setPluginParameters(null); setPluginParameterError(null);
    void backend.describePluginParameters(path).then((result) => {
      if (!active) return;
      if (result.sha256 !== fingerprint) { setPluginParameterError("Plugin identity changed; scan it again before editing parameters."); return; }
      setPluginParameters(result);
    }).catch((error) => { if (active) setPluginParameterError(formatUiError(error, "Plugin parameters unavailable.")); });
    return () => { active = false; };
  }, [backend, selectedNode?.id, selectedNode?.kind, selectedNode?.parameters.path, selectedNode?.parameters.fingerprint]);
  useEffect(() => {
    let active = true;
    void backend.listSessions().then((items) => { if (active) { setListedSessions(items); setSessionInventoryError(null); } }).catch((error) => { if (active) { setSessionInventoryError(formatUiError(error, "Session inventory unavailable")); setListedSessions(backend.connected ? [] : demoSessions); } });
    return () => { active = false; };
  }, [backend]);
  useEffect(() => {
    let active = true;
    void backend.listRecordings(session.id).then((items) => { if (active) { setRecordings(items); setRecordingsError(null); } }).catch((error) => { if (active) { setRecordings([]); setRecordingsError(formatUiError(error, "Recording library unavailable")); } });
    return () => { active = false; };
  }, [backend, session.id]);
  useEffect(() => {
    let active = true;
    void backend.listRecorders().then((items) => { if (active) { setRecorderStatuses(items); setRecorderStatusAvailable(true); } }).catch(() => { if (active) { setRecorderStatuses([]); setRecorderStatusAvailable(false); } });
    return () => { active = false; };
  }, [backend]);
  useEffect(() => {
    let active = true;
    void backend.listApplications().then((items) => { if (active) { setApplications(items); setApplicationsError(null); } }).catch((error) => { if (active) { setApplications([]); setApplicationsError(formatUiError(error, "Application inventory unavailable")); } });
    return () => { active = false; };
  }, [backend]);
  useEffect(() => {
    let active = true;
    void backend.listDevices(true).then((items) => { if (active) { setDevices(items); setDevicesError(null); } }).catch((error) => { if (active) { setDevices([]); setDevicesError(formatUiError(error, "Device inventory unavailable")); } });
    return () => { active = false; };
  }, [backend]);
  useEffect(() => {
    if (!backend.connected) return;
    let active = true;
    let polling = false;
    const poll = async () => {
      if (!active || polling) return;
      polling = true;
      try {
        const result = await backend.subscribe(eventCursor.current.sequence, session.id, eventCursor.current.backendEpoch, [...WORKSPACE_EVENT_CATEGORIES]);
        if (!active) return;
        const bridgeEvent = result.events.find((event) => event.category === "virtualBridge.failed" || event.category === "virtualBridge.expired");
        if (bridgeEvent) {
          const bus = bridgeEvent.operationId ?? "an affected bus";
          setActionMessage(bridgeEvent.category === "virtualBridge.expired"
            ? `Virtual bridge lease expired for ${bus}; the route is silenced until it is deliberately restarted.`
            : `Virtual bridge failure detected for ${bus}; the route is silenced until it is deliberately recovered.`);
        }
        if (result.resyncRequired || result.events.length > 0) {
          const nextState = await snapshotCache.refresh(backend);
          if (active) {
            setSnapshotState(nextState);
            refreshApplications();
            refreshDevices();
            void backend.listSessions().then((items) => { if (active) { setListedSessions(items); setSessionInventoryError(null); } }).catch((error) => { if (active) setSessionInventoryError(formatUiError(error, "Session inventory unavailable")); });
            void backend.listRecordings(session.id).then((items) => { if (active) { setRecordings(items); setRecordingsError(null); } }).catch((error) => { if (active) setRecordingsError(formatUiError(error, "Recording library unavailable")); });
            void backend.listRecorders().then((items) => { if (active) { setRecorderStatuses(items); setRecorderStatusAvailable(true); } }).catch(() => { if (active) setRecorderStatusAvailable(false); });
            if (!nextState.stale) eventCursor.current = { backendEpoch: result.backendEpoch, sequence: result.nextSequence };
          }
        } else {
          eventCursor.current = { backendEpoch: result.backendEpoch, sequence: result.nextSequence };
        }
      } catch (error) {
        if (active) setSnapshotState((current) => ({ ...current, stale: true, error: formatUiError(error, "Event subscription failed") }));
      } finally {
        polling = false;
      }
    };
    void poll();
    const timer = window.setInterval(() => void poll(), 1000);
    return () => { active = false; window.clearInterval(timer); };
  }, [backend, session.id, snapshotCache]);
  useEffect(() => {
    if (!backend.connected || !sessionRunning) return;
    let active = true;
    let refreshing = false;
    const refreshDiagnostics = async () => {
      if (!active || refreshing) return;
      refreshing = true;
      try {
        const diagnostics = await backend.refreshDiagnostics();
        if (active) setSnapshotState((current) => current.snapshot ? { ...current, snapshot: { ...current.snapshot, diagnostics } } : current);
      } catch {
        // Keep the last known diagnostics; the event/snapshot path reports
        // connection failures and never replaces observations with guesses.
      } finally {
        refreshing = false;
      }
    };
    void refreshDiagnostics();
    const timer = window.setInterval(() => void refreshDiagnostics(), DIAGNOSTICS_REFRESH_INTERVAL_MS);
    return () => { active = false; window.clearInterval(timer); };
  }, [backend, sessionRunning]);
  useEffect(() => {
    const nativeAdapterKind = snapshot?.diagnostics.nativeAdapterKind;
    const pumpNativeEndpoint = backend.pumpNativeEndpoint;
    const pumpNativeDuplex = backend.pumpNativeDuplex;
    const pumpKind = selectNativePump(nativeAdapterKind, Boolean(pumpNativeEndpoint), Boolean(pumpNativeDuplex));
    if (!backend.connected || nativeGeneration === null || !sessionRunning || pumpKind === null) return;
    let active = true;
    let pumping = false;
    let lastReportedAt = 0;
    const pump = async () => {
      if (!active || pumping) return;
      pumping = true;
      try {
        const result = pumpKind === "duplex"
          ? await pumpNativeDuplex!(session.id, nativeGeneration, 64, 64)
          : await pumpNativeEndpoint!(session.id, nativeGeneration, 64);
        const now = Date.now();
        if (now - lastReportedAt >= 1000) { lastReportedAt = now; setNativePumpStats(result); }
      }
      catch { setNativePumpStats(null); /* Diagnostics remain backend-owned; do not retry or substitute endpoints here. */ }
      finally { pumping = false; }
    };
    void pump();
    const timer = window.setInterval(() => void pump(), 20);
    return () => { active = false; window.clearInterval(timer); };
  }, [backend, nativeGeneration, session.id, sessionRunning, snapshot?.diagnostics.nativeAdapterKind]);
  const draftNodeNames = new Map(draft.nodes.map((node) => [node.id, node.name]));
  const outputPorts = draft.nodes.flatMap((node) => node.ports.filter((port) => port.direction === "output").map((port) => ({ nodeId: node.id, nodeName: node.name, portName: port.name, channels: port.channels })));
  const inputPorts = draft.nodes.flatMap((node) => node.ports.filter((port) => port.direction === "input").map((port) => ({ nodeId: node.id, nodeName: node.name, portName: port.name, channels: port.channels })));
  const encodePort = (nodeId: string, portName: string) => `${nodeId}::${portName}`;
  const decodePort = (value: string) => { const separator = value.indexOf("::"); return separator < 0 ? null : { nodeId: value.slice(0, separator), portName: value.slice(separator + 2) }; };
  const visibleRecordings = recordings.filter((recording) => {
    const query = recordingSearch.trim().toLocaleLowerCase();
    if (!query) return true;
    return [recording.id, recording.title, recording.artist, recording.comment, recording.path]
      .filter((value): value is string => value !== null)
      .some((value) => value.toLocaleLowerCase().includes(query));
  });
  const visibleLibraryEntries = filterLibraryEntries(libraryEntries, librarySearch);
  const setupSteps = setupChecklist({ connected: backend.connected, audio: snapshot?.status.audio ?? null, storage: snapshot?.status.storage ?? null, deviceCount: devices.length, applicationCount: applications.length, vbCablePairAvailable: findVbCableEndpointPair(devices) !== null });
  const connectionLabel = backend.connected ? "Backend connected" : "Backend disconnected";
  const nativePumpSummary = formatNativePumpSummary(nativePumpStats, sessionRunning);
  const schedulerTelemetry = snapshot?.diagnostics.schedulerTelemetry;
  const schedulerSummary = schedulerTelemetry ? ` - ${schedulerTelemetry.processedQuanta} quanta / ${schedulerTelemetry.xruns} xruns` : "";
  const statusSummary = `${snapshot ? `${snapshot.status.audio} audio (${snapshot.status.reason}) - ${snapshot.status.storage} storage - ${snapshot.status.sessionCount} session${snapshot.status.sessionCount === 1 ? "" : "s"}` : "Waiting for backend snapshot"}${schedulerSummary}${nativePumpSummary ? ` - ${nativePumpSummary}` : ""}${sessionCrudBusyState ? " - Updating session..." : ""}`;
  const recordDraftChange = (next: import("@audiorouter/contracts").Session) => { setDraftHistory((history) => recordDraft(history, draft, next)); setDraft(next); };
  const undoDraft = () => { const transition = undoDraftHistory(draftHistory, draft); if (transition.current === draft) return; setDraftHistory(transition.history); setDraft(transition.current); setActionMessage("Undid the last draft change."); };
  const redoDraft = () => { const transition = redoDraftHistory(draftHistory, draft); if (transition.current === draft) return; setDraftHistory(transition.history); setDraft(transition.current); setActionMessage("Redid the draft change."); };
  const changeNodeFlag = (flag: "enabled" | "bypass", value: boolean) => { recordDraftChange(setNodeDraftFlag(draft, selectedNode.id, flag, value)); setActionMessage("Draft updated. Review and plan the changes before committing."); };
  const changeNodeName = (name: string) => { try { recordDraftChange(setNodeDraftName(draft, selectedNode.id, name)); setActionMessage("Node name draft updated. Review and plan the changes before committing."); } catch (error) { setActionMessage(formatUiError(error, "Unable to rename node.")); } };
  const changeNodeParameter = (name: string, value: boolean | number | string) => { const error = selectedNode.kind === "plugin" ? (() => { const id = Number(name.slice("pluginParameter:".length)); const descriptor = pluginParameters?.parameters.find((parameter) => parameter.parameterId === id); return !descriptor || typeof value !== "number" || !Number.isFinite(value) || value < descriptor.minimum || value > descriptor.maximum ? "Plugin parameter value is outside the worker-provided range" : null; })() : processorParameterError(processors, selectedNode.kind, name, value); if (error) { setActionMessage(`Draft rejected: ${error}.`); return; } recordDraftChange(setNodeDraftParameter(draft, selectedNode.id, name, value)); setActionMessage("Draft updated. Review and plan the changes before committing."); };
  const resetNodeParameters = () => { recordDraftChange(resetNodeDraftParameters(draft, selectedNode.id)); setActionMessage("Processor parameters reset in the draft. Review and plan the changes before committing."); };
  const changeSessionName = (name: string) => { try { recordDraftChange(setSessionDraftName(draft, name)); setActionMessage("Session name draft updated. Review and plan the change before committing."); } catch (error) { setActionMessage(formatUiError(error, "Unable to rename session.")); } };
  const planChanges = async () => {
    if (graphBusy || !backend.connected) return;
    setGraphBusy(true);
    setActionMessage("Planning changes...");
    try {
      const operation = uiIdempotencyKey("graph-commit");
      const plan = await backend.planGraph(draft);
      if (plan.baseRevision !== draft.revision) throw new Error("Backend returned a plan for a different session revision");
      if (plan.warnings.length > 0) {
        setPendingOperation(operation);
        setPendingWarnings(plan.warnings);
        setAcknowledgedWarnings(new Set());
        setActionMessage("Review and acknowledge every plan warning before committing.");
        return;
      }
      const result = await backend.commitGraph(plan.planId, plan.baseRevision, operation);
      setActionMessage(`Committed revision ${result.revision}. Reconnect to refresh the authoritative view.`);
    } catch (error) {
      if (isRevisionConflict(error)) {
        setPendingWarnings([]); setAcknowledgedWarnings(new Set()); setPendingOperation(null);
        void snapshotCache.refresh(backend).then(setSnapshotState);
        setActionMessage(`${formatUiError(error, "Graph changed elsewhere.")} The authoritative session was refreshed; review the draft again.`);
      } else setActionMessage(formatUiError(error, "Unable to apply graph changes."));
    } finally { setGraphBusy(false); }
  };
  const commitAcknowledgedPlan = async () => {
    if (graphBusy || !backend.connected || !pendingOperation || acknowledgedWarnings.size !== pendingWarnings.length) return;
    setGraphBusy(true);
    setActionMessage("Replanning acknowledged changes...");
    try {
      const result = await applyGraphDraft(backend, draft, pendingOperation, [...acknowledgedWarnings]);
      setPendingWarnings([]); setAcknowledgedWarnings(new Set()); setPendingOperation(null);
      setActionMessage(`Committed revision ${result.revision}. Reconnect to refresh the authoritative view.`);
    } catch (error) {
      if (isRevisionConflict(error)) {
        setPendingWarnings([]); setAcknowledgedWarnings(new Set()); setPendingOperation(null);
        void snapshotCache.refresh(backend).then(setSnapshotState);
        setActionMessage(`${formatUiError(error, "Graph changed elsewhere.")} The authoritative session was refreshed; review the draft again.`);
      } else setActionMessage(formatUiError(error, "Unable to commit acknowledged changes."));
    } finally { setGraphBusy(false); }
  };
  const inspectRoute = async () => { const request = ++routeInspectionRequest.current; setActionMessage("Inspecting route..."); try { const result = await backend.inspectRoute(selectedNode.id); if (request === routeInspectionRequest.current) { setRouteInspection(result); setActionMessage("Route inspection refreshed from the backend."); } } catch (error) { if (request === routeInspectionRequest.current) { setRouteInspection(null); setActionMessage(formatUiError(error, "Unable to inspect route.")); } } };
  const previewRecording = async (recordingId: string) => { const request = ++previewRequest.current; setPreviewMessage("Inspecting recording..."); try { const result = await backend.previewRecording(recordingId); if (request === previewRequest.current) setPreviewMessage(`${String(result.preview.status)} recording preview loaded.`); } catch (error) { if (request === previewRequest.current) setPreviewMessage(formatUiError(error, "Recording preview unavailable.")); } };
  const inspectRecovery = async (recordingId: string) => { const request = ++recoveryRequest.current; setRecoveryMessage("Inspecting recorder recovery..."); try { const result = await backend.getRecordingRecovery(recordingId); if (request === recoveryRequest.current) setRecoveryMessage(result.status === "missing" ? "No persisted recovery checkpoint is available." : `Recovery checkpoint: ${result.checkpoint.state}.`); } catch (error) { if (request === recoveryRequest.current) setRecoveryMessage(formatUiError(error, "Recording recovery unavailable.")); } };
  const saveRecordingMetadata = async (recording: import("@audiorouter/contracts").RecordingRow) => { if (!backend.connected || recordingMutationBusy.current) return; recordingMutationBusy.current = true; setRecordingMutationBusyState(true); try { const title = metadataTitles[recording.id]?.trim() ?? recording.title ?? ""; const artist = metadataArtists[recording.id]?.trim() ?? recording.artist ?? ""; const comment = metadataComments[recording.id]?.trim() ?? recording.comment ?? ""; await backend.setRecordingMetadata(recording.id, { title: title || null, artist: artist || null, comment: comment || null, idempotencyKey: uiIdempotencyKey("recording-metadata") }); setRecordings((current) => current.map((item) => item.id === recording.id ? { ...item, title: title || null, artist: artist || null, comment: comment || null } : item)); setPreviewMessage("Recording metadata saved; the audio file was unchanged."); } catch (error) { setPreviewMessage(formatUiError(error, "Unable to save recording metadata.")); } finally { recordingMutationBusy.current = false; setRecordingMutationBusyState(false); } };
  const removeRecordingEntry = async (recordingId: string) => { if (!window.confirm("Remove this library entry? The audio file will be preserved.")) return; if (!backend.connected || recordingMutationBusy.current) return; recordingMutationBusy.current = true; setRecordingMutationBusyState(true); try { await backend.removeRecordingEntry(recordingId, uiIdempotencyKey("recording-entry-remove")); setRecordings((current) => current.filter((item) => item.id !== recordingId)); setPreviewMessage("Library entry removed; the audio file was preserved."); } catch (error) { setPreviewMessage(formatUiError(error, "Unable to remove recording entry.")); } finally { recordingMutationBusy.current = false; setRecordingMutationBusyState(false); } };
  const renameRecording = async (recordingId: string, newPath: string) => { if (!backend.connected || recordingMutationBusy.current) return; recordingMutationBusy.current = true; setRecordingMutationBusyState(true); try { const result = await backend.renameRecording(recordingId, newPath.trim(), uiIdempotencyKey("recording-rename")); setRecordings((current) => current.map((item) => item.id === recordingId ? { ...item, path: result.path, missing: false } : item)); setPreviewMessage("Recording renamed within the approved directory."); } catch (error) { setPreviewMessage(formatUiError(error, "Unable to rename recording.")); } finally { recordingMutationBusy.current = false; setRecordingMutationBusyState(false); } };
  const revealRecording = async (recordingId: string) => { if (!backend.connected || recordingMutationBusy.current) return; recordingMutationBusy.current = true; setRecordingMutationBusyState(true); try { const result = await backend.revealRecording(recordingId); setPreviewMessage(result.revealed ? "Recording revealed by the operating system." : "Recording is missing; no operating-system action was performed."); } catch (error) { setPreviewMessage(formatUiError(error, "Unable to reveal recording.")); } finally { recordingMutationBusy.current = false; setRecordingMutationBusyState(false); } };
  const recycleRecording = async (recordingId: string, confirm: boolean) => { if (!backend.connected || recordingMutationBusy.current) return; recordingMutationBusy.current = true; setRecordingMutationBusyState(true); try { const result = await backend.recycleRecording(recordingId, confirm, confirm ? uiIdempotencyKey("recording-recycle") : undefined); setPreviewMessage(result.fileAction === "recycled" ? "Recording recycled." : result.fileAction === "recycle" ? "Recycle preview loaded; confirmation is still required." : `Recording was not recycled: ${result.reason}.`); if (result.fileAction === "recycled") setRecordings((current) => current.map((item) => item.id === recordingId ? { ...item, missing: true } : item)); } catch (error) { setPreviewMessage(formatUiError(error, "Unable to recycle recording.")); } finally { recordingMutationBusy.current = false; setRecordingMutationBusyState(false); } };
  const togglePrivacyMute = async () => { if (safetyActionBusy.current || !backend.connected) return; safetyActionBusy.current = true; setSafetyActionBusyState(true); const next = !privacyMuted; setActionMessage(next ? "Enabling privacy mute..." : "Disabling privacy mute..."); try { await backend.setPrivacyMute(next, uiIdempotencyKey("privacy-mute")); setPrivacyMuted(next); setActionMessage(next ? "Privacy mute enabled." : "Privacy mute disabled."); } catch (error) { setPrivacyMuted(true); setActionMessage(formatUiError(error, "Unable to change privacy mute.")); } finally { safetyActionBusy.current = false; setSafetyActionBusyState(false); } };
  const clearRecoverySafeMode = async () => { if (safetyActionBusy.current || !backend.connected) return; safetyActionBusy.current = true; setSafetyActionBusyState(true); setActionMessage("Clearing recovery safe mode..."); try { await backend.clearRecoverySafeMode(uiIdempotencyKey("recovery-clear")); await refresh(); setActionMessage("Recovery safe mode cleared."); } catch (error) { setActionMessage(formatUiError(error, "Unable to clear recovery safe mode.")); } finally { safetyActionBusy.current = false; setSafetyActionBusyState(false); } };
  const createSession = async () => { if (sessionCrudBusy.current || !backend.connected) return; const name = window.prompt("New session name", "New session")?.trim(); if (!name) return; sessionCrudBusy.current = true; setSessionCrudBusyState(true); const id = `session-${Date.now()}`; try { const result = await backend.createSession({ ...demoSession, id, name, revision: 0, nodes: demoSession.nodes.map((node) => ({ ...node, parameters: { ...node.parameters } })), edges: [...demoSession.edges] }, uiIdempotencyKey("session-create")); setCreatedSessions((current) => [...current, result.session]); setSelectedSessionId(result.session.id); setActionMessage(`Created stopped session ${result.session.name}.`); } catch (error) { setActionMessage(formatUiError(error, "Unable to create session.")); } finally { sessionCrudBusy.current = false; setSessionCrudBusyState(false); } };
  const duplicateSession = async () => { if (sessionCrudBusy.current || !backend.connected) return; sessionCrudBusy.current = true; setSessionCrudBusyState(true); const id = `session-copy-${Date.now()}`; const name = `${session.name} (copy)`; try { const result = await backend.duplicateSession(session.id, id, name, uiIdempotencyKey("session-duplicate")); setCreatedSessions((current) => [...current, result.session]); setSelectedSessionId(result.session.id); setActionMessage(`Duplicated stopped session ${result.session.name}.`); } catch (error) { setActionMessage(formatUiError(error, "Unable to duplicate session.")); } finally { sessionCrudBusy.current = false; setSessionCrudBusyState(false); } };
  const deleteSession = async () => { if (sessionCrudBusy.current || !backend.connected || !window.confirm(`Delete stopped session “${session.name}”?`)) return; sessionCrudBusy.current = true; setSessionCrudBusyState(true); try { await backend.deleteSession(session.id, uiIdempotencyKey("session-delete")); setCreatedSessions((current) => current.filter((item) => item.id !== session.id)); const fallback = availableSessions.find((item) => item.id !== session.id); if (fallback) setSelectedSessionId(fallback.id); setActionMessage(`Deleted session ${session.name}.`); } catch (error) { setActionMessage(formatUiError(error, "Unable to delete session.")); } finally { sessionCrudBusy.current = false; setSessionCrudBusyState(false); } };
  const startSession = async () => { if (sessionBusy.current || !backend.connected) return; sessionBusy.current = true; setSessionActionBusy(true); setActionMessage("Starting session..."); setNativePumpStats(null); try { const result = await backend.startSession(session.id, uiIdempotencyKey("session-start")); setNativeGeneration(result.runtime === "native" ? result.generation : null); await refresh(); setActionMessage(`Session is running (generation ${result.generation}).`); } catch (error) { setNativeGeneration(null); setNativePumpStats(null); setActionMessage(formatUiError(error, "Unable to start session.")); } finally { sessionBusy.current = false; setSessionActionBusy(false); } };
  const stopSession = async () => { if (sessionBusy.current || !backend.connected) return; sessionBusy.current = true; setSessionActionBusy(true); setActionMessage("Stopping session..."); try { await backend.stopSession(session.id, uiIdempotencyKey("session-stop")); setNativeGeneration(null); setNativePumpStats(null); await refresh(); setActionMessage("Session stopped."); } catch (error) { setNativeGeneration(null); setNativePumpStats(null); setActionMessage(formatUiError(error, "Unable to stop session.")); } finally { sessionBusy.current = false; setSessionActionBusy(false); } };
  useEffect(() => {
    const onShortcut = (event: KeyboardEvent) => {
      if (isEditableShortcutTarget(event.target)) return;
      const shortcut = shortcutFromKeyboardEvent(event);
      if (!shortcut || shortcutConflicts(shortcuts).length > 0) return;
      if (shortcut === shortcuts.sessionToggle && backend.connected) { event.preventDefault(); void (sessionRunning ? stopSession() : startSession()); }
      else if (shortcut === shortcuts.privacyMute && backend.connected) { event.preventDefault(); void togglePrivacyMute(); }
    };
    window.addEventListener("keydown", onShortcut);
    return () => window.removeEventListener("keydown", onShortcut);
  }, [backend, sessionRunning, shortcuts, privacyMuted, session.id]);
  const captureShortcut = (action: ShortcutAction, event: React.KeyboardEvent<HTMLInputElement>) => {
    event.preventDefault();
    const shortcut = shortcutFromKeyboardEvent(event.nativeEvent);
    if (!shortcut) { setShortcutMessage("Use at least one modifier key and a non-modifier key."); return; }
    const next = { ...shortcuts, [action]: shortcut };
    setShortcuts(next);
    setShortcutMessage(shortcutConflicts(next).length > 0 ? "Shortcut conflict: duplicate bindings are disabled until resolved." : null);
  };
  const addLibraryNode = (kind: LibraryNodeKind, _position?: { x: number; y: number }) => { if (!backend.connected) { setActionMessage("Connect the backend before changing the draft."); return; } const next = appendLibraryNode(draft, kind); const inserted = next.nodes[next.nodes.length - 1]; recordDraftChange(next); setSelectedNodeId(inserted.id); setActionMessage(`${inserted.name} added to the draft. Review and plan the changes before committing.`); return inserted.id; };
  const addConnection = () => { const source = decodePort(connectionSource); const destination = decodePort(connectionDestination); if (!source || !destination) { setActionMessage("Choose an output and input port first."); return false; } try { const next = appendDraftConnection(draft, source.nodeId, source.portName, destination.nodeId, destination.portName); recordDraftChange(next); setActionMessage("Connection added to the draft. Review and plan the changes before committing."); return true; } catch (error) { setActionMessage(formatUiError(error, "Unable to add connection.")); return false; } };
  const insertProcessor = (edgeId: string, kind: InsertableProcessorKind) => { if (!backend.connected) { setActionMessage("Connect the backend before changing draft topology."); return; } try { const next = insertDraftProcessor(draft, edgeId, kind); const inserted = next.nodes.at(-1); recordDraftChange(next); if (inserted) setSelectedNodeId(inserted.id); setActionMessage(`${inserted?.name ?? kind} inserted into the draft. Review and plan the changes before committing.`); } catch (error) { setActionMessage(formatUiError(error, "Unable to insert processor.")); } };
  const appendPreset = (presetId: EqPresetId) => { if (!backend.connected) { setActionMessage("Connect the backend before adding a preset."); return; } try { const next = appendEqPresetNode(draft, presetId); const inserted = next.nodes.at(-1); recordDraftChange(next); if (inserted) setSelectedNodeId(inserted.id); setActionMessage(`${inserted?.name ?? "EQ preset"} added to the draft. Review and plan the changes before committing.`); } catch (error) { setActionMessage(formatUiError(error, "Unable to add preset.")); } };
  const appendVoicePreset = (presetId: VoiceChainPresetId) => { if (!backend.connected) { setActionMessage("Connect the backend before adding a preset."); return; } try { const next = appendVoiceChainPreset(draft, presetId); const added = next.nodes.slice(draft.nodes.length); recordDraftChange(next); if (added[0]) setSelectedNodeId(added[0].id); setActionMessage(`${added.map((node) => node.name).join(", ")} added to the draft. Review and plan the changes before committing.`); } catch (error) { setActionMessage(formatUiError(error, "Unable to add voice preset.")); } };
  useEffect(() => {
    const handleInsertProcessor = (event: Event) => {
      const detail = (event as CustomEvent<{ edgeId?: string; kind?: InsertableProcessorKind }>).detail;
      if (detail.edgeId && detail.kind) insertProcessor(detail.edgeId, detail.kind);
    };
    globalThis.addEventListener("audiorouter:insert-processor", handleInsertProcessor);
    return () => globalThis.removeEventListener("audiorouter:insert-processor", handleInsertProcessor);
  }, [backend, draft]);
  useEffect(() => {
    const handleRemoveNode = (event: Event) => {
      const nodeId = (event as CustomEvent<{ nodeId?: string }>).detail?.nodeId;
      if (!nodeId || !backend.connected || !window.confirm("Remove this node and its draft connections?")) return;
      try {
        const next = removeDraftNode(draft, nodeId);
        recordDraftChange(next);
        setSelectedNodeId(next.nodes[0]?.id ?? "");
        setActionMessage("Node removed from the draft. Review and plan the changes before committing.");
      } catch (error) {
        setActionMessage(formatUiError(error, "Unable to remove node."));
      }
    };
    globalThis.addEventListener("audiorouter:remove-node", handleRemoveNode);
    return () => globalThis.removeEventListener("audiorouter:remove-node", handleRemoveNode);
  }, [backend, draft]);
  useEffect(() => {
    const handleAppendEqPreset = (event: Event) => {
      const detail = (event as CustomEvent<{ presetId?: EqPresetId }>).detail;
      if (detail.presetId) appendPreset(detail.presetId);
    };
    globalThis.addEventListener("audiorouter:append-eq-preset", handleAppendEqPreset);
    return () => globalThis.removeEventListener("audiorouter:append-eq-preset", handleAppendEqPreset);
  }, [backend, draft]);
  useEffect(() => {
    const handleAppendVoicePreset = (event: Event) => {
      const detail = (event as CustomEvent<{ presetId?: VoiceChainPresetId }>).detail;
      if (detail.presetId) appendVoicePreset(detail.presetId);
    };
    globalThis.addEventListener("audiorouter:append-voice-preset", handleAppendVoicePreset);
    return () => globalThis.removeEventListener("audiorouter:append-voice-preset", handleAppendVoicePreset);
  }, [backend, draft]);
  const connectCanvas = (connection: Connection) => {
    if (!backend.connected) { setActionMessage("Connect the backend before adding a canvas connection."); return; }
    if (connection.source === LIBRARY_DROP_SOURCE && connection.sourceHandle) {
      addLibraryNode(connection.sourceHandle as LibraryNodeKind);
      return;
    }
    if (!connection.source || !connection.sourceHandle || !connection.target || !connection.targetHandle) { setActionMessage("Choose a named output and input port."); return; }
    try {
      recordDraftChange(appendDraftConnection(draft, connection.source, connection.sourceHandle, connection.target, connection.targetHandle));
      setActionMessage("Connection added to the draft. Review and plan the changes before committing.");
    } catch (error) { setActionMessage(formatUiError(error, "Unable to add canvas connection.")); }
  };
  const openConnectionDialog = (event: React.MouseEvent<HTMLButtonElement>) => { connectionDialogReturnFocus.current = event.currentTarget; setConnectionDialogOpen(true); };
  const closeConnectionDialog = () => { setConnectionDialogOpen(false); window.setTimeout(() => connectionDialogReturnFocus.current?.focus(), 0); };
  useEffect(() => {
    if (!connectionDialogOpen) return;
    connectionDialogSource.current?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") { event.preventDefault(); closeConnectionDialog(); return; }
      if (event.key !== "Tab") return;
      const focusable = connectionDialog.current?.querySelectorAll<HTMLElement>("button:not([disabled]), select:not([disabled]), input:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])");
      if (!focusable?.length) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
      else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [connectionDialogOpen]);
  const removeConnection = (edgeId: string) => { if (!backend.connected) { setActionMessage("Connect the backend before changing draft topology."); return; } const topologyAction = decodeTopologyAction(edgeId); try { if (topologyAction?.kind === "insertMixer") recordDraftChange(insertDraftMixer(draft, topologyAction.id)); else if (topologyAction?.kind === "removeMixer") recordDraftChange(removeSinglePathDraftMixer(draft, topologyAction.id)); else recordDraftChange(removeDraftConnection(draft, edgeId)); setActionMessage(topologyAction?.kind === "insertMixer" ? "Mixer inserted into the draft. Review and plan the changes before committing." : topologyAction?.kind === "removeMixer" ? "Mixer removed and its single path reconnected in the draft. Review and plan the changes before committing." : "Connection removed from the draft. Review and plan the changes before committing."); } catch (error) { setActionMessage(formatUiError(error, topologyAction?.kind === "insertMixer" ? "Unable to insert mixer." : topologyAction?.kind === "removeMixer" ? "Unable to remove and reconnect mixer." : "Unable to remove connection.")); } };
  const toggleConnection = (edgeId: string, enabled: boolean) => { if (!backend.connected) { setActionMessage("Connect the backend before changing draft topology."); return; } try { recordDraftChange(setDraftConnectionEnabled(draft, edgeId, enabled)); setActionMessage(`Connection ${enabled ? "enabled" : "disabled"} in the draft. Review and plan the changes before committing.`); } catch (error) { setActionMessage(formatUiError(error, "Unable to change connection state.")); } };
  const removeSelectedNode = () => { if (!window.confirm(`Remove node “${selectedNode.name}” and its draft connections?`)) return; try { const next = removeDraftNode(draft, selectedNode.id); recordDraftChange(next); setSelectedNodeId(next.nodes[0]?.id ?? ""); setActionMessage("Node removed from the draft. Review and plan the changes before committing."); } catch (error) { setActionMessage(formatUiError(error, "Unable to remove node.")); } };
  const duplicateSelectedNode = () => { try { const next = duplicateDraftNode(draft, selectedNode.id); const copy = next.nodes[next.nodes.length - 1]; recordDraftChange(next); setSelectedNodeId(copy.id); setActionMessage(`${copy.name} added to the draft without connections. Review and plan the changes before committing.`); } catch (error) { setActionMessage(formatUiError(error, "Unable to duplicate node.")); } };
  const applyTemplate = () => { const next = templateSession(selectedTemplate); recordDraftChange({ ...next, id: draft.id, revision: draft.revision }); setSelectedNodeId(next.nodes[0]?.id ?? ""); setActionMessage("Template loaded into the draft. Review device bindings and plan the changes before committing."); };
  const lifecycleActions = <section className="panel lifecycle-panel" aria-labelledby="lifecycle-heading"><h2 id="lifecycle-heading">Session lifecycle</h2><p className="muted">Starting a session uses the shared authorized backend lifecycle API.</p><button type="button" className="secondary" onClick={() => void (sessionRunning ? stopSession() : startSession())} disabled={!backend.connected || sessionActionBusy}>{sessionRunning ? "Stop session" : "Start session"}</button></section>;
  return <PluginParameterContext.Provider value={{ parameters: pluginParameters, error: pluginParameterError }}><div className={`app-shell theme-${theme}${compactStatus ? " compact-status" : ""}`}>{lifecycleActions}<RecorderActions backend={backend} sessionId={session.id} connected={backend.connected} recorderStatuses={recorderStatuses} recorderStatusAvailable={recorderStatusAvailable} /><RecordingActions recordings={recordings} connected={backend.connected} busy={recordingMutationBusyState} onRename={renameRecording} onReveal={revealRecording} onRecycle={recycleRecording} /><VirtualDeviceLifecyclePanel backend={backend} /><VirtualRoutePanel backend={backend} />
    <header className="topbar"><div><p className="eyebrow">AudioRouter</p><h1>Routing workspace</h1></div><div className="status-cluster" aria-live="polite"><span className={`status-dot${backend.connected ? "" : " disconnected"}`} aria-hidden="true" /><span>{connectionLabel}</span><span className="status-detail">{statusSummary}</span><label className="theme-picker">Theme<select aria-label="Color theme" value={theme} onChange={(event) => setTheme(event.target.value as ThemeMode)}><option value="dark">Dark</option><option value="light">Light</option><option value="high-contrast">High contrast</option></select></label><button type="button" className="secondary" aria-pressed={compactStatus} onClick={() => setCompactStatus((current) => !current)}>{compactStatus ? "Full workspace" : "Compact status"}</button><button type="button" onClick={refresh}>Reconnect</button></div></header>
    {compactStatus && <section className="compact-status-panel" aria-label="Compact route status"><div><p className="eyebrow">Compact route status</p><strong>{session.name}</strong><p className="muted">{statusSummary}</p></div><button type="button" onClick={() => void (sessionRunning ? stopSession() : startSession())} disabled={!backend.connected || sessionActionBusy}>{sessionRunning ? "Stop session" : "Start session"}</button><button type="button" className="secondary" aria-pressed={privacyMuted} onClick={() => void togglePrivacyMute()} disabled={!backend.connected || safetyActionBusyState}>{privacyMuted ? "Privacy mute enabled" : "Enable privacy mute"}</button></section>}
      <section className="panel shortcut-panel" aria-labelledby="shortcut-heading"><div className="section-heading"><h2 id="shortcut-heading">Keyboard shortcuts</h2><span className="badge">local</span></div><p className="muted">Shortcuts work while the workspace is focused and never capture text-field typing. They call the same authorized API actions as the buttons.</p><label>Start/stop session<input aria-label="Start or stop session shortcut" value={shortcuts.sessionToggle} readOnly onKeyDown={(event) => captureShortcut("sessionToggle", event)} /></label><label>Privacy mute<input aria-label="Privacy mute shortcut" value={shortcuts.privacyMute} readOnly onKeyDown={(event) => captureShortcut("privacyMute", event)} /></label>{shortcutMessage && <p className="muted" role="alert">{shortcutMessage}</p>}<small>Native tray and OS-wide registration remain platform validation work.</small></section>
      <div className="workspace-grid"><aside className="sidebar" aria-label="Sessions"><div className="section-heading"><h2>Sessions</h2><button type="button" aria-label="Create session" onClick={() => void createSession()} disabled={!backend.connected} title="Session creation requires the connected backend">+</button></div>{sessionInventoryError && <p className="muted" role="status">Session inventory unavailable: {sessionInventoryError}</p>}<label className="session-picker">Preview session<select value={session.id} onChange={(event) => setSelectedSessionId(event.target.value)}>{availableSessions.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</select></label>{availableSessions.map((item) => { const running = snapshot?.status.activeSessionIds.includes(item.id) ?? false; return <button type="button" key={item.id} className={`session-item${item.id === session.id ? " selected" : ""}`} aria-current={item.id === session.id ? "true" : undefined} onClick={() => setSelectedSessionId(item.id)}><span>{item.name}</span><small>{running ? "Running" : "Stopped"} - rev {item.revision}</small></button>; })}<div className="sidebar-note"><strong>Safe startup</strong><p>Monitoring is muted and recording is unarmed until you explicitly start them.</p></div></aside>
      <main className="main-content"><section className="workspace-title"><div><p className="eyebrow">{sessionRunning ? "Running session" : "Stopped session"}</p><label className="session-name">Session name<input value={draft.name} maxLength={120} disabled={!backend.connected} onChange={(event) => changeSessionName(event.target.value)} /></label><p className="muted">Revision {session.revision} - {backend.connected ? "draft changes require plan and commit" : "changes are presentation-only in this preview"}</p></div><div className="actions"><button type="button" className="secondary" onClick={() => void duplicateSession()} disabled={!backend.connected}>Duplicate</button><button type="button" className="secondary" onClick={() => void deleteSession()} disabled={!backend.connected}>Delete</button><button type="button" className="secondary" onClick={undoDraft} disabled={!backend.connected || draftHistory.past.length === 0}>Undo draft</button><button type="button" className="secondary" onClick={redoDraft} disabled={!backend.connected || draftHistory.future.length === 0}>Redo draft</button><button type="button" className="secondary" onClick={() => { setDraft(session); setDraftHistory({ past: [], future: [] }); setActionMessage("Draft discarded."); }} disabled={!backend.connected}>Discard draft</button><button type="button" className="primary" onClick={() => void planChanges()} disabled={!backend.connected}>Plan changes</button></div></section>
        {actionMessage && <p className="muted" role="status" aria-live="polite">{actionMessage}</p>}{pendingWarnings.length > 0 && <section className="warning-panel" aria-labelledby="warning-heading"><h2 id="warning-heading">Plan warnings</h2>{pendingWarnings.map((warning) => <label key={warning}><input type="checkbox" checked={acknowledgedWarnings.has(warning)} onChange={(event) => setAcknowledgedWarnings((current) => { const next = new Set(current); if (event.target.checked) next.add(warning); else next.delete(warning); return next; })} /> I acknowledge: {warning}</label>)}<button type="button" className="primary" disabled={acknowledgedWarnings.size !== pendingWarnings.length} onClick={() => void commitAcknowledgedPlan()}>Commit acknowledged plan</button></section>}{snapshotState.stale && snapshotState.error && <p className="muted" role="status">Last known backend state is stale: {snapshotState.error}</p>}
        <section className="notice" role="status"><strong>{backend.connected ? "Connected editor" : "Read-only preview"}</strong><span>{backend.connected ? "Drafts are validated and committed through the authoritative backend." : "The control backend is disconnected. No route, device, or recording action can be applied."}</span></section>
        <section className="panel quick-route-panel" aria-labelledby="quick-route-heading"><div className="section-heading"><div><p className="eyebrow">First run</p><h2 id="quick-route-heading">Quick route</h2></div><span className="badge">3 steps</span></div><p className="muted">Configure an existing VB-Cable route, add processing in the visual graph, and start it only after reviewing the draft.</p><ol aria-label="Quick route steps"><li><a href="#native-endpoint-panel">Select endpoints</a><span>Choose VB-Cable capture and your physical render output, or use the explicit loopback action for testing.</span></li><li><a href="#signal-flow-panel">Build the graph</a><span>Drag a built-in EQ, gate, compressor, limiter, pitch, or other processor onto the canvas, then plan and commit the draft.</span></li><li><a href="#native-endpoint-panel">Start the session</a><span>Prepare the exact endpoints first, then start or stop the session from the same binding panel.</span></li></ol></section>
        <section className="panel setup-panel" aria-labelledby="setup-heading"><div className="section-heading"><div><p className="eyebrow">Guided setup</p><h2 id="setup-heading">Readiness checklist</h2></div><span className="badge">{setupSteps.filter((step) => step.state === "ready").length}/{setupSteps.length} ready</span></div><ul aria-label="Guided setup readiness">{setupSteps.map((step) => <li key={step.id} className={`setup-step ${step.state}`}><strong>{step.label}</strong><span>{step.detail}</span></li>)}</ul><p className="muted">External applications such as Discord and OBS must be pointed to AudioRouter through their own settings; this checklist never changes them automatically.</p></section>
        <ProcessorCatalog processors={processors} error={processorError} node={selectedNode} backend={backend} /><PresetCatalog presets={presets} error={presetError} /><SessionTransferPanel backend={backend} session={session} onImported={(imported) => { setCreatedSessions((current) => [...current.filter((item) => item.id !== imported.id), imported]); setSelectedSessionId(imported.id); void refresh(); }} /><PluginScanPanel backend={backend} onAddPlaceholder={(entry) => { try { const next = appendPluginPlaceholderNode(draft, entry); recordDraftChange(next); setActionMessage("Added a stopped plugin placeholder to the draft. Bind a worker before activation."); } catch (error) { setActionMessage(error instanceof Error ? error.message : "Could not add plugin placeholder."); } }} /><StartupPanel backend={backend} /><OsTransitionPanel backend={backend} onRefresh={refresh} />
        <ApplicationIdentityPanel applications={applications} />
        <section className="panel recovery-panel" aria-labelledby="recovery-heading"><div className="section-heading"><div><p className="eyebrow">Crash recovery</p><h2 id="recovery-heading">{snapshot?.status.recovery.safeMode ? "Safe mode is active" : "Normal startup mode"}</h2></div><span className="badge">{snapshot?.status.recovery.recentCrashes ?? 0} recent crash{(snapshot?.status.recovery.recentCrashes ?? 0) === 1 ? "" : "es"}</span></div><p className="muted">{snapshot?.status.recovery.persistence === "durable" ? "Recovery state is persisted by the backend." : "Recovery state is held in memory for this preview."}</p><button type="button" className="secondary" onClick={() => void clearRecoverySafeMode()} disabled={!backend.connected || safetyActionBusyState || !snapshot?.status.recovery.safeMode}>Clear safe mode</button></section>
        <RecoveryCheckpointPanel backend={backend} />
        <section className="panel" aria-labelledby="applications-heading"><div className="section-heading"><div><p className="eyebrow">Audio sources</p><h2 id="applications-heading">Applications</h2></div><span className="badge">{applicationsError ? "unavailable" : applications.length}</span></div><label>Application capture policy<select aria-label="Application capture policy" value={applicationCaptureMode} disabled={!backend.connected || sessionRunning} onChange={(event) => setApplicationCaptureMode(event.target.value as "include" | "exclude")}><option value="include">Include selected application</option><option value="exclude">Exclude selected application</option></select></label>{applicationsError ? <p className="muted">Application inventory unavailable: {applicationsError}</p> : applications.length === 0 ? <p className="muted">No process audio sessions are exposed by the backend snapshot.</p> : <ul aria-label="Running audio applications">{applications.map((application) => <li key={`${application.processId}-${application.creationTime100ns ?? "unknown"}`}><strong>{application.audioDisplayNames[0] ?? application.executable}</strong> <small>{application.executable} · PID {application.processId} · {application.audioActivity} · {application.captureCapability === "observed" ? "capture observed" : "capture not observed"} · render sessions {application.renderSessionCount}</small><button type="button" className="secondary" onClick={() => { try { const next = appendApplicationCaptureNode(draft, application); const added = next.nodes.at(-1); if (!added) throw new Error("Application capture node was not created"); recordDraftChange(next); setSelectedNodeId(added.id); setActionMessage("Added a stopped application-capture source; review identity and plan the graph before committing."); } catch (error) { setActionMessage(error instanceof Error ? error.message : "Could not add application capture."); } }} disabled={!backend.connected || application.captureCapability !== "observed"}>Add capture source</button>{application.creationTime100ns && <button type="button" className="secondary" onClick={() => { const renderEndpointId = readEndpointBindingHint(session.id).renderEndpointId; if (!backend.prepareNativeApplication || !renderEndpointId) { setActionMessage("Select an exact active render endpoint in Endpoint binding before preparing application capture."); return; } if (sessionRunning) { setActionMessage("Stop the session before preparing a new application capture worker."); return; } void backend.prepareNativeApplication({ sessionId: session.id, processId: application.processId, executable: application.executable, executablePath: application.executablePath, creationTime100ns: application.creationTime100ns, mode: applicationCaptureMode, renderEndpointId }).then((result) => setActionMessage(`Prepared ${result.executable} in ${result.state}; start the session to activate application capture.`)).catch((error) => setActionMessage(formatUiError(error, "Application capture preparation failed."))); }} disabled={!backend.connected || application.captureCapability !== "observed" || sessionRunning} title="Requires a committed matching application-capture node and an exact selected render endpoint">Prepare application worker</button>}</li>)}</ul>}<p className="muted">Capture binds to the observed executable identity. Restart or ambiguity stays silent until a deliberate re-selection. Preparation requires the matching graph node to be committed and an exact render binding; it opens stopped clients only.</p></section>
        <section className="panel" aria-labelledby="devices-heading"><div className="section-heading"><div><p className="eyebrow">Windows endpoints</p><h2 id="devices-heading">Devices</h2></div><span className="badge">{devicesError ? "unavailable" : devices.length}</span></div>{devicesError ? <p className="muted">Device inventory unavailable: {devicesError}</p> : devices.length === 0 ? <p className="muted">No endpoint metadata is exposed by the backend.</p> : <ul aria-label="Audio devices">{devices.map((device) => <li key={`${device.direction}-${device.id}`}><strong>{device.direction === "capture" ? "Capture" : "Render"}</strong> <small>{device.name} · {device.id} · {device.state}{device.state === "active" ? ` · ${device.format.sampleRateHz} Hz · ${device.format.channels} ch · ${device.periods.default100ns / 10000} ms period` : " · format unavailable"}{device.defaultRoles.length ? ` · defaults: ${device.defaultRoles.join(", ")}` : ""}</small></li>)}</ul>}</section>
        <NativeEndpointPanel backend={backend} sessionId={session.id} devices={devices} sessionRunning={sessionRunning} onStart={startSession} onStop={stopSession} />
        <section id="signal-flow-panel" className="canvas-panel" aria-labelledby="canvas-heading"><div className="section-heading"><div><p className="eyebrow">Signal flow</p><h2 id="canvas-heading">{listView ? "Graph list" : "Canvas"}</h2></div><button type="button" className="secondary" aria-pressed={listView} onClick={() => setListView((current) => !current)}>{listView ? "Canvas view" : "List view"}</button></div>{listView ? <NodeList session={draft} selectedNodeId={selectedNode.id} onSelect={(id) => { setSelectedNodeId(id); setSelectedNodeIds([id]); }} onRemoveConnection={removeConnection} onToggleConnection={toggleConnection} onInsertProcessor={insertProcessor} /> : <><SessionFlowCanvas session={draft} selectedNodeId={selectedNode.id} selectedNodeIds={selectedNodeIds} onSelect={(id) => { setSelectedNodeId(id); setSelectedNodeIds([id]); }} onSelectMany={(ids) => { setSelectedNodeIds(ids); if (ids[0]) setSelectedNodeId(ids[0]); }} onConnect={connectCanvas} onRemoveConnection={removeConnection} onAddLibraryNode={addLibraryNode} canEdit={backend.connected} /><DraftConnectionList session={draft} onRemove={removeConnection} onToggle={toggleConnection} /></>}<fieldset className="connection-editor" disabled={!backend.connected}><legend>Add connection to draft</legend><label>Output<select aria-label="Source output port" value={connectionSource} onChange={(event) => setConnectionSource(event.target.value)}><option value="">Choose source</option>{outputPorts.map((port) => <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>{port.nodeName} · {port.portName} · {port.channels}ch</option>)}</select></label><span aria-hidden="true">→</span><label>Input<select aria-label="Destination input port" value={connectionDestination} onChange={(event) => setConnectionDestination(event.target.value)}><option value="">Choose destination</option>{inputPorts.map((port) => <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>{port.nodeName} · {port.portName} · {port.channels}ch</option>)}</select></label><button type="button" className="secondary" onClick={addConnection}>Add connection</button><button type="button" className="secondary" onClick={openConnectionDialog}>Keyboard connection dialog</button></fieldset>{connectionDialogOpen && <div className="dialog-backdrop" role="presentation"><section ref={connectionDialog} className="connection-dialog" role="dialog" aria-modal="true" aria-labelledby="connection-dialog-heading" aria-describedby="connection-dialog-description"><div className="section-heading"><h2 id="connection-dialog-heading">Keyboard connection</h2><button type="button" className="secondary" onClick={closeConnectionDialog} aria-label="Close keyboard connection dialog">Close</button></div><p id="connection-dialog-description" className="muted">Choose an output and input, then add the connection to the draft. Press Escape to close.</p><label>Output<select ref={connectionDialogSource} aria-label="Keyboard source output port" value={connectionSource} onChange={(event) => setConnectionSource(event.target.value)}><option value="">Choose source</option>{outputPorts.map((port) => <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>{port.nodeName} · {port.portName} · {port.channels}ch</option>)}</select></label><label>Input<select aria-label="Keyboard destination input port" value={connectionDestination} onChange={(event) => setConnectionDestination(event.target.value)}><option value="">Choose destination</option>{inputPorts.map((port) => <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>{port.nodeName} · {port.portName} · {port.channels}ch</option>)}</select></label><div className="actions"><button type="button" className="primary" onClick={() => { if (addConnection()) closeConnectionDialog(); }}>Add connection to draft</button><button type="button" className="secondary" onClick={closeConnectionDialog}>Cancel</button></div></section></div>}</section>
        <section className="panel inspector" aria-labelledby="inspector-heading"><div className="section-heading"><div><p className="eyebrow">Selected node</p><h2 id="inspector-heading">{selectedNode.name}</h2></div><span className="badge">{selectedNode.kind}</span></div><InspectorChangeSummary draftNode={selectedNode} authoritativeNode={session.nodes.find((node) => node.id === selectedNode.id)} /><NodeTelemetryPanel node={selectedNode} snapshot={snapshot?.diagnostics ?? null} /><div className="inspector-grid"><label>Node name<input type="text" maxLength={120} value={selectedNode.name} disabled={!backend.connected} onChange={(event) => changeNodeName(event.target.value)} /></label><label>Enabled<input type="checkbox" checked={selectedNode.enabled} disabled={!backend.connected} onChange={(event) => changeNodeFlag("enabled", event.target.checked)} /></label><label>Bypass<input type="checkbox" checked={selectedNode.bypass} disabled={!backend.connected} onChange={(event) => changeNodeFlag("bypass", event.target.checked)} /></label><ProcessorParameterEditor node={selectedNode} processors={processors} connected={backend.connected} onChange={changeNodeParameter} />{processors?.some((processor) => processor.id === selectedNode.kind && processor.parameters.length > 0) && <button type="button" className="secondary" onClick={resetNodeParameters} disabled={!backend.connected}>Reset parameters</button>}<button type="button" className="secondary" onClick={duplicateSelectedNode} disabled={!backend.connected}>Duplicate node to draft</button><button type="button" className="secondary" onClick={removeSelectedNode} disabled={!backend.connected}>Remove node from draft</button><button type="button" onClick={() => void togglePrivacyMute()} disabled={!backend.connected} aria-pressed={privacyMuted}>{privacyMuted ? "Privacy mute enabled" : "Enable privacy mute"}</button><p className="muted">{backend.connected ? "Changes are local drafts until Plan changes is committed. Privacy mute is an immediate safety latch." : "Controls are disabled while disconnected. Selection is local presentation state only."}</p></div></section>
        <section className="lower-grid"><div className="panel"><div className="section-heading"><h2>Library</h2><button type="button" className="secondary" onClick={() => document.getElementById("library-search")?.focus()}>Search</button></div><label className="library-search">Search nodes<input id="library-search" type="search" value={librarySearch} onChange={(event) => setLibrarySearch(event.target.value.slice(0, 80))} placeholder="Gain, effect, meter..." /></label><div className="library-grid">{visibleLibraryEntries.length === 0 ? <p className="muted">No library entries match this search.</p> : visibleLibraryEntries.map((entry) => entry.kind ? <button type="button" key={entry.id} aria-label={libraryEntryAccessibleLabel(entry)} onClick={() => addLibraryNode(entry.kind!)} disabled={!backend.connected}>{entry.label}<small>{entry.category} · add to draft</small></button> : <button type="button" key={entry.id} aria-label={libraryEntryAccessibleLabel(entry)} disabled title={entry.unavailableReason}>{entry.label}<small>{entry.category} · unavailable: {entry.unavailableReason}</small></button>)}</div><p className="muted">Built-in processors are added as local drafts; use Plan changes to validate and commit them.</p><div className="template-picker"><label>Guided template<select aria-label="Guided setup template" value={selectedTemplate} onChange={(event) => setSelectedTemplate(event.target.value as TemplateId)}><option value="gaming-discord">Gaming + Discord</option><option value="processed-microphone">Processed microphone</option><option value="mix-minus conversation">Mix-minus conversation</option></select></label><button type="button" className="secondary" onClick={applyTemplate} disabled={!backend.connected}>Load template to draft</button><small>Templates remain stopped and require device review before commit.</small></div></div><div className="panel"><div className="section-heading"><h2>Recordings</h2><span className="badge">{recordingsError ? "unavailable" : `${visibleRecordings.length}${recordingSearch.trim() ? ` of ${recordings.length}` : ""} file${visibleRecordings.length === 1 ? "" : "s"}`}</span></div><label className="recording-search">Search recordings<input id="recording-search" type="search" value={recordingSearch} onChange={(event) => setRecordingSearch(event.target.value.slice(0, 160))} placeholder="Title, path, or status" /></label>{recordingsError ? <p className="muted">Recording library unavailable: {recordingsError}</p> : recordings.length === 0 ? <p className="muted">No recording has been armed. Completed recordings will appear here with path and status.</p> : visibleRecordings.length === 0 ? <p className="muted">No recording matches this search.</p> : visibleRecordings.map((recording) => <p className="muted" key={recording.id}><label>Title <input aria-label={`Title for ${recording.id}`} value={metadataTitles[recording.id] ?? recording.title ?? ""} onChange={(event) => setMetadataTitles((current) => ({ ...current, [recording.id]: event.target.value }))} /></label> <label>Artist <input aria-label={`Artist for ${recording.id}`} value={metadataArtists[recording.id] ?? recording.artist ?? ""} onChange={(event) => setMetadataArtists((current) => ({ ...current, [recording.id]: event.target.value }))} /></label> <label>Comment <input aria-label={`Comment for ${recording.id}`} value={metadataComments[recording.id] ?? recording.comment ?? ""} onChange={(event) => setMetadataComments((current) => ({ ...current, [recording.id]: event.target.value }))} /></label> - {recording.state}{recording.missing ? " - missing" : ""}<br /><small>{recording.path}</small><br /><small>Duration {formatRecordingDuration(recording.frames, recording.sampleRate)} · {recording.fileBytes} bytes</small> <button type="button" className="secondary" onClick={() => void saveRecordingMetadata(recording)} disabled={!backend.connected}>Save metadata</button> <button type="button" className="secondary" onClick={() => void previewRecording(recording.id)} disabled={!backend.connected}>Preview</button> <button type="button" className="secondary" onClick={() => void inspectRecovery(recording.id)} disabled={!backend.connected}>Recovery</button> <button type="button" className="secondary" onClick={() => void removeRecordingEntry(recording.id)} disabled={!backend.connected}>Remove entry</button></p>)}{previewMessage && <p className="muted" role="status">{previewMessage}</p>}{recoveryMessage && <p className="muted" role="status">{recoveryMessage}</p>}</div></section>
        <section className="panel route-inspection" aria-labelledby="route-heading"><div className="section-heading"><div><p className="eyebrow">Backend explanation</p><h2 id="route-heading">Receives audio from</h2></div><button type="button" className="secondary" onClick={() => void inspectRoute()} disabled={!backend.connected}>Refresh</button></div>{routeInspection === null ? <p className="muted">No route inspection loaded. The backend is authoritative; no path is inferred.</p> : <p className="muted">{routeInspection.reachable ? `${routeInspection.paths.length} reachable path${routeInspection.paths.length === 1 ? "" : "s"}${routeInspection.complete ? "" : " (partial; path limit reached)"} reported to ${draftNodeNames.get(routeInspection.destinationNode) ?? routeInspection.destinationNode}.` : "No reachable route reported by the backend."}</p>}{routeInspection?.paths.length ? <ol aria-label="Reported audio paths">{routeInspection.paths.map((path, index) => <li key={`${path.nodes.join("-")}-${index}`} className="muted"><span>Path {index + 1}: {routeNodeLabels(draft, path.nodes).join(" → ") || "empty"} ({path.edges.length} edge{path.edges.length === 1 ? "" : "s"}; {routeLatencyText(path.latencySamples)})</span>{path.nodes.some((nodeId) => draft.nodes.some((node) => node.id === nodeId && node.bypass)) && <small> · includes a bypassed stage</small>}{path.nodes.some((nodeId) => draft.nodes.some((node) => node.id === nodeId && !node.enabled)) && <small> · includes a disabled/silent stage</small>}<br /><small>Channel map: {path.channelMaps.length === 0 ? "none" : path.channelMaps.map((row) => `[${row.join(", ")}]`).join(" ")}</small></li>)}</ol> : null}</section>
        <section className="panel recording-encoding-panel" aria-labelledby="recording-encoding-heading"><div className="section-heading"><div><p className="eyebrow">File details</p><h2 id="recording-encoding-heading">Recording encoding</h2></div><span className="badge">{recordings.length}</span></div>{recordings.length === 0 ? <p className="muted">Encoding details appear after a recording is finalized.</p> : <ul aria-label="Recording encoding details">{recordings.map((recording) => <li key={`encoding-${recording.id}`}><strong>{recording.id}</strong><small>{recording.format} - {recording.sampleRate} Hz - {recording.channels} channel{recording.channels === 1 ? "" : "s"} - {recording.dither ? "TPDF dither" : "no dither"} - {recording.conversion}</small></li>)}</ul>}<p className="muted">These values are persisted by the backend at finalization and describe the file that was written.</p></section>
      </main></div>
  </div></PluginParameterContext.Provider>;
}
