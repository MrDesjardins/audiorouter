import { useEffect, useRef, useState } from "react";
import type { Node, RouteInspection } from "@audiorouter/contracts";
import { SessionFlowCanvas } from "./SessionFlowCanvas";
import { createDisconnectedBackend, formatUiError, isRevisionConflict, SnapshotCache, type ApplicationRow, type UiBackend } from "./backend";
import type { DeviceInfo } from "@audiorouter/contracts";
import { appendDraftConnection, appendLibraryNode, applyGraphDraft, duplicateDraftNode, removeDraftConnection, removeDraftNode, resetNodeDraftParameters, setDraftConnectionEnabled, setNodeDraftFlag, setNodeDraftName, setNodeDraftParameter, setSessionDraftName, type LibraryNodeKind } from "./draft";
import { demoSession, demoSessions } from "./fixtures";
import { recordDraft, redoDraft as redoDraftHistory, undoDraft as undoDraftHistory, type DraftHistory } from "./history";
import { templateSession, type TemplateId } from "./templates";
import { filterLibraryEntries, libraryEntries, libraryEntryAccessibleLabel } from "./library";
import { nodePortLabels, routeLatencyText, routeNodeLabels } from "./graphView";
import { readTheme, writeTheme, type ThemeMode } from "./preferences";
import { ApplicationIdentityPanel } from "./ApplicationIdentityPanel";
import { setupChecklist } from "./setup";
import { uiIdempotencyKey } from "./idempotency";
import { processorAvailabilityText, processorLatencyText, processorParameterError, processorParametersText, type ProcessorDescriptor } from "./processorCatalog";
import { mergeSessionInventory } from "./sessionInventory";

const defaultBackend = createDisconnectedBackend();

function StartupPanel({ backend }: { backend: UiBackend }) {
  const [status, setStatus] = useState<import("@audiorouter/contracts").StartupStatus | null>(null);
  const [enabled, setEnabled] = useState(false);
  const [plan, setPlan] = useState<import("@audiorouter/contracts").StartupPlanResult | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const refresh = () => {
    void backend.getStartup().then((result) => { setStatus(result); setEnabled(result.enabled); setMessage(null); }).catch((error) => setMessage(formatUiError(error, "Startup status unavailable.")));
  };
  useEffect(() => { setPlan(null); refresh(); }, [backend]);
  const createPlan = async () => {
    setMessage("Planning sign-in startup policy...");
    try { const result = await backend.planStartup(enabled); setPlan(result); setMessage(result.reason); }
    catch (error) { setMessage(formatUiError(error, "Startup planning unavailable.")); }
  };
  const applyPlan = async () => {
    if (!plan) return;
    setMessage("Applying startup policy...");
    try { const result = await backend.applyStartup(plan.planId, uiIdempotencyKey("startup-apply")); setPlan(null); setMessage(result.reason); refresh(); }
    catch (error) { setMessage(formatUiError(error, "Startup apply unavailable.")); }
  };
  return <section className="panel startup-panel" aria-labelledby="startup-heading"><div className="section-heading"><div><p className="eyebrow">Background lifecycle</p><h2 id="startup-heading">Start at sign-in</h2></div><button type="button" className="secondary" onClick={refresh}>Refresh</button></div><p className="muted">{status?.reason ?? "Loading startup capability..."}</p><label>Desired policy<select aria-label="Desired sign-in startup policy" value={enabled ? "enabled" : "disabled"} onChange={(event) => { setEnabled(event.target.value === "enabled"); setPlan(null); }} disabled={!backend.connected}><option value="disabled">Disabled</option><option value="enabled">Enabled</option></select></label><div className="actions"><button type="button" className="secondary" onClick={() => void createPlan()} disabled={!backend.connected}>Plan startup policy</button>{plan && <button type="button" className="secondary" onClick={() => void applyPlan()}>Apply planned policy</button>}</div>{message && <p className="muted" role="status">{message}</p>}<p className="muted">Registration is currently unavailable in this build. Planning and applying only exercise the authorized backend boundary; they do not register Windows startup.</p></section>;
}

function ProcessorCatalog({ processors, error, node, backend }: { processors: ProcessorDescriptor[] | null; error: string | null; node: Node; backend: UiBackend }) {
  return <section className="panel processor-catalog" aria-labelledby="processor-catalog-heading"><div className="section-heading"><div><p className="eyebrow">DSP catalog</p><h2 id="processor-catalog-heading">Built-in processors</h2></div><span className="badge">{processors?.length ?? 0}</span></div>{error ? <p className="muted" role="status">Processor catalog unavailable: {error}</p> : processors === null ? <p className="muted">Connect to the backend to load the authoritative processor catalog.</p> : processors.length === 0 ? <p className="muted">No built-in processors are advertised.</p> : <ul aria-label="Built-in processor catalog">{processors.map((processor) => <li key={`${processor.id}@${processor.version}`}><strong>{processor.id}</strong> <small>{processor.category} · {processorAvailabilityText(processor)} · {processorLatencyText(processor)}</small><br /><small>Parameters: {processorParametersText(processor)}</small></li>)}</ul>}<p className="muted">This catalog is read-only. Unavailable processors cannot be added or activated.</p><EqResponsePreview node={node} backend={backend} /></section>;
}

function ProcessorParameterEditor({ node, processors, connected, onChange }: { node: Node; processors: ProcessorDescriptor[] | null; connected: boolean; onChange: (name: string, value: boolean | number | string) => void }) {
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
    return <label key={parameter.name}>{parameter.name}{parameter.unit ? ` (${parameter.unit})` : ""}<input type="number" value={numericValue} min={parameter.minimum} max={parameter.maximum} step={parameter.unit === "Hz" ? 1 : 0.1} disabled={!connected} onChange={(event) => onChange(parameter.name, Number(event.target.value))} /></label>;
  })}</>;
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
    void backend.processorResponse({ sampleRateHz: 48000, bands, frequenciesHz: EQ_RESPONSE_FREQUENCIES }).then((value) => { if (active) { setResponse(value); setError(null); } }).catch((reason) => { if (active) { setResponse(null); setError(reason instanceof Error ? reason.message : "EQ response unavailable."); } });
    return () => { active = false; };
  }, [backend, node.kind, node.parameters]);
  if (node.kind !== "parametricEq") return null;
  const points = response?.frequenciesHz.map((frequency, index) => { const magnitude = response.magnitudeDb[index] ?? 0; const x = 8 + (Math.log10(frequency / 20) / 3) * 284; const bounded = Math.max(-24, Math.min(24, magnitude)); const y = 56 - ((bounded + 24) / 48) * 48; return `${x.toFixed(1)},${y.toFixed(1)}`; }).join(" ");
  return <section className="eq-response" aria-labelledby="eq-response-heading"><h3 id="eq-response-heading">EQ response</h3>{error ? <p className="muted" role="status">{error}</p> : !response ? <p className="muted">Loading the authoritative response...</p> : <svg viewBox="0 0 300 64" role="img" aria-label="Parametric EQ magnitude response"><line x1="8" y1="32" x2="292" y2="32" stroke="currentColor" opacity="0.35" /><polyline points={points} fill="none" stroke="currentColor" strokeWidth="1.5" /></svg>}</section>;
}

function PresetCatalog({ presets, error }: { presets: import("@audiorouter/contracts").DiscoveryDocument["presets"] | null; error: string | null }) {
  const entries = presets ? [...presets.voiceChains.map((preset) => ({ ...preset, category: "Voice chain" })), ...presets.eq.map((preset) => ({ ...preset, category: "EQ" }))] : [];
  return <section className="panel preset-catalog" aria-labelledby="preset-catalog-heading"><div className="section-heading"><div><p className="eyebrow">Saved starting points</p><h2 id="preset-catalog-heading">Presets</h2></div><span className="badge">{entries.length}</span></div>{error ? <p className="muted" role="status">Preset catalog unavailable: {error}</p> : presets === null ? <p className="muted">Connect to the backend to load the authoritative preset catalog.</p> : entries.length === 0 ? <p className="muted">No presets are advertised.</p> : <ul aria-label="Available presets">{entries.map((preset) => <li key={`${preset.category}-${preset.id}`}><strong>{preset.name}</strong> <small>{preset.category} · {preset.description}</small></li>)}</ul>}<p className="muted">This catalog is read-only. Loading a preset does not change the current session draft.</p></section>;
}

function PluginScanPanel({ backend }: { backend: UiBackend }) {
  const [directory, setDirectory] = useState("");
  const [result, setResult] = useState<import("@audiorouter/contracts").PluginScanResult | null>(null);
  const [inspectionPath, setInspectionPath] = useState("");
  const [inspection, setInspection] = useState<import("@audiorouter/contracts").PluginScanEntry | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const scan = async () => {
    if (!directory.trim()) { setMessage("Enter an absolute plugin directory."); return; }
    setMessage("Scanning selected directory...");
    try { setResult(await backend.scanPlugins(directory.trim())); setMessage("Plugin scan completed without loading plugin code."); }
    catch (error) { setResult(null); setMessage(formatUiError(error, "Plugin scan unavailable.")); }
  };
  const list = async () => {
    if (!directory.trim()) { setMessage("Enter an absolute plugin directory."); return; }
    setMessage("Loading the last explicit plugin scan...");
    try { setResult(await backend.listPlugins(directory.trim())); setMessage("Loaded the last backend scan without rescanning."); }
    catch (error) { setResult(null); setMessage(formatUiError(error, "Plugin inventory unavailable.")); }
  };
  const inspect = async () => {
    if (!inspectionPath.trim()) { setMessage("Enter an absolute plugin path."); return; }
    setMessage("Inspecting selected plugin path...");
    try { setInspection(await backend.inspectPlugin(inspectionPath.trim())); setMessage("Plugin inspection completed without loading plugin code."); }
    catch (error) { setInspection(null); setMessage(formatUiError(error, "Plugin inspection unavailable.")); }
  };
  const retry = async () => {
    if (!directory.trim()) { setMessage("Enter an absolute plugin directory."); return; }
    setMessage("Retrying selected directory scan...");
    try { setResult(await backend.retryPlugins(directory.trim(), uiIdempotencyKey("plugins-retry"))); setMessage("Plugin scan retry completed without loading plugin code."); }
    catch (error) { setMessage(formatUiError(error, "Plugin scan retry unavailable.")); }
  };
  const selectInspectionPath = (path: string) => { setInspectionPath(path); setMessage("Selected the discovered path; inspect it explicitly when ready."); };
  return <section className="panel plugin-scan-panel" aria-labelledby="plugin-scan-heading"><div className="section-heading"><div><p className="eyebrow">VST3 and VST2 discovery</p><h2 id="plugin-scan-heading">Plugin scan</h2></div><span className="badge">{result?.entries.length ?? 0}</span></div><p className="muted">Choose a directory explicitly. Discovery inspects bounded metadata only; it does not load or execute plugins.</p><label>Absolute plugin directory<input aria-label="Absolute plugin directory" value={directory} onChange={(event) => setDirectory(event.target.value)} disabled={!backend.connected} placeholder="C:\\Plugins" /></label><button type="button" className="secondary" onClick={() => void scan()} disabled={!backend.connected}>Scan directory</button><button type="button" className="secondary" onClick={() => void list()} disabled={!backend.connected}>Load last scan</button><button type="button" className="secondary" onClick={() => void retry()} disabled={!backend.connected}>Retry scan</button>{result && <ul aria-label="Plugin scan results">{result.entries.length === 0 ? <li className="muted">No VST3, VST2, or other DLL candidates found.</li> : result.entries.map((entry) => <li key={entry.path}><strong>{entry.path}</strong> <small>{entry.identity ? `${entry.identity.format} · ${entry.identity.architecture} · ${entry.identity.compatibility}` : `${entry.errorCode ?? "unknown"}: ${entry.error ?? "inspection failed"}`}</small><button type="button" className="secondary" onClick={() => selectInspectionPath(entry.path)} disabled={!backend.connected}>Select for inspection</button></li>)}</ul>}<label>Absolute plugin path<input aria-label="Absolute plugin path" value={inspectionPath} onChange={(event) => setInspectionPath(event.target.value)} disabled={!backend.connected} placeholder="C:\\Plugins\\effect.vst3 or effect.dll" /></label><button type="button" className="secondary" onClick={() => void inspect()} disabled={!backend.connected}>Inspect path</button>{inspection && <p className="muted" role="status">{inspection.identity ? `${inspection.identity.format} ${inspection.identity.architecture} · ${inspection.identity.compatibility}` : `${inspection.errorCode ?? "unknown"}: ${inspection.error ?? "inspection failed"}`}</p>}{message && <p className="muted" role="status" aria-live="polite">{message}</p>}<p className="muted">The explicit <code>pluginScan</code> permission is required by the backend; selecting a result only copies its path, and inspection remains explicit. Loading the last scan never triggers a new filesystem scan.</p></section>;
}

function RecorderActions({ backend, sessionId, connected }: { backend: UiBackend; sessionId: string; connected: boolean }) {
  const [frameText, setFrameText] = useState("0");
  const [state, setState] = useState("idle");
  const [message, setMessage] = useState<string | null>(null);
  const frame = Number.parseInt(frameText, 10);
  const validFrame = Number.isSafeInteger(frame) && frame >= 0;
  const run = async (action: string, operation: () => Promise<{ state: string }>) => {
    setMessage(`${action}...`);
    try {
      const result = await operation();
      setState(result.state);
      setMessage(`Recorder ${result.state} at frame ${frame}.`);
    } catch (error) {
      setMessage(formatUiError(error, `Unable to ${action.toLowerCase()} recorder.`));
    }
  };
  return <section className="panel recorder-actions" aria-labelledby="recorder-actions-heading">
    <div className="section-heading"><div><p className="eyebrow">Frame boundary control</p><h2 id="recorder-actions-heading">Recorder</h2></div><span className="badge">{state}</span></div>
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
    <p className="muted">Actions are sent only to a connected backend and use explicit engine frame boundaries. The preview backend never arms or starts recording.</p>
  </section>;
}

function RecordingActions({ recordings, connected, onRename, onReveal, onRecycle }: { recordings: import("@audiorouter/contracts").RecordingRow[]; connected: boolean; onRename: (recordingId: string, newPath: string) => Promise<void>; onReveal: (recordingId: string) => Promise<void>; onRecycle: (recordingId: string, confirm: boolean) => Promise<void> }) {
  const [selectedId, setSelectedId] = useState("");
  const [newPath, setNewPath] = useState("");
  const selected = recordings.find((recording) => recording.id === selectedId) ?? recordings[0];
  useEffect(() => { if (selected) { setSelectedId(selected.id); setNewPath(selected.path); } else { setSelectedId(""); setNewPath(""); } }, [selected?.id, selected?.path]);
  if (!selected) return null;
  return <section className="panel recording-actions" aria-labelledby="recording-actions-heading"><div className="section-heading"><div><p className="eyebrow">File actions</p><h2 id="recording-actions-heading">Recording operations</h2></div><span className="badge">explicit</span></div><label>Recording<select aria-label="Recording file action target" value={selected.id} onChange={(event) => { const next = recordings.find((recording) => recording.id === event.target.value); setSelectedId(event.target.value); setNewPath(next?.path ?? ""); }}>{recordings.map((recording) => <option key={recording.id} value={recording.id}>{recording.title || recording.id} · {recording.state}</option>)}</select></label><label>New path<input type="text" value={newPath} onChange={(event) => setNewPath(event.target.value)} disabled={!connected} /></label><div className="actions"><button type="button" className="secondary" onClick={() => void onRename(selected.id, newPath)} disabled={!connected || !newPath.trim() || newPath === selected.path}>Rename in approved directory</button><button type="button" className="secondary" onClick={() => void onReveal(selected.id)} disabled={!connected}>Reveal</button><button type="button" className="secondary" onClick={() => void onRecycle(selected.id, false)} disabled={!connected}>Preview recycle</button><button type="button" className="secondary" onClick={() => { if (window.confirm("Recycle this recording through the operating system?")) void onRecycle(selected.id, true); }} disabled={!connected}>Recycle recording</button></div><p className="muted">Rename is restricted by the backend to the approved directory. Reveal and recycle never run while disconnected; recycle requires an explicit confirmation.</p></section>;
}

function VirtualDeviceLifecyclePanel({ backend }: { backend: UiBackend }) {
  const [devices, setDevices] = useState<import("@audiorouter/contracts").VirtualDeviceInfo[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [action, setAction] = useState<"create" | "rename" | "setEnabled" | "delete">("create");
  const [busId, setBusId] = useState("virtual-bus");
  const [busName, setBusName] = useState("AudioRouter Bus");
  const [plan, setPlan] = useState<import("@audiorouter/contracts").VirtualDevicePlanResult | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const refresh = () => { void backend.listVirtualDevices().then((items) => { setDevices(items); if (!selectedId && items[0]) { setSelectedId(items[0].id); setBusId(items[0].id); setBusName(items[0].name); } }).catch((error) => setMessage(formatUiError(error, "Virtual-device inventory unavailable."))); };
  useEffect(() => { if (backend.connected) refresh(); else { setDevices([]); setPlan(null); } }, [backend]);
  const selected = devices.find((device) => device.id === selectedId);
  const chooseAction = (next: "create" | "rename" | "setEnabled" | "delete") => { setAction(next); setPlan(null); if (next === "create") { setBusId("virtual-bus"); setBusName("AudioRouter Bus"); } else if (selected) { setBusId(selected.id); setBusName(selected.name); } };
  const createPlan = async () => { const operation: import("@audiorouter/contracts").VirtualDeviceOperation = action === "create" ? { action, id: busId.trim(), name: busName.trim() } : action === "rename" ? { action, id: selectedId, name: busName.trim() } : action === "setEnabled" ? { action, id: selectedId, enabled: !(selected?.enabled ?? false) } : { action, id: selectedId }; if (!operation.id || ("name" in operation && !operation.name)) { setMessage("Provide a valid bus ID and name."); return; } setMessage("Planning managed virtual-bus change..."); try { const result = await backend.planVirtualDevice(operation); setPlan(result); setMessage(result.availability.reason); } catch (error) { setMessage(formatUiError(error, "Unable to plan managed virtual-bus change.")); } };
  const applyPlan = async () => { if (!plan) return; setMessage("Applying desired virtual-bus state..."); try { const result = await backend.applyVirtualDevice(plan.planId, uiIdempotencyKey("virtual-device-apply")); setPlan(null); setMessage(`${result.availability.reason}; no native endpoint is active in this build.`); refresh(); } catch (error) { setMessage(formatUiError(error, "Unable to apply virtual-bus plan.")); } };
  return <section className="panel virtual-device-lifecycle" aria-labelledby="virtual-device-lifecycle-heading"><div className="section-heading"><div><p className="eyebrow">Managed buses</p><h2 id="virtual-device-lifecycle-heading">Virtual-device lifecycle</h2></div><button type="button" className="secondary" onClick={refresh} disabled={!backend.connected}>Refresh</button></div>{devices.length === 0 ? <p className="muted">No managed virtual buses are currently listed.</p> : <ul aria-label="Managed virtual devices">{devices.map((device) => <li key={device.id}><strong>{device.name}</strong> <small>{device.enabled ? "enabled" : "disabled"} · {device.availability.reason} · lease {device.leaseOwner ?? "none"}</small></li>)}</ul>}<fieldset disabled={!backend.connected}><legend>Desired-state operation</legend><label>Action<select aria-label="Virtual-device action" value={action} onChange={(event) => chooseAction(event.target.value as typeof action)}><option value="create">Create</option><option value="rename" disabled={!selected}>Rename</option><option value="setEnabled" disabled={!selected}>Enable/disable</option><option value="delete" disabled={!selected}>Delete</option></select></label>{action !== "create" && <label>Existing bus<select aria-label="Existing virtual-device target" value={selectedId} onChange={(event) => { const next = devices.find((device) => device.id === event.target.value); setSelectedId(event.target.value); setBusName(next?.name ?? ""); }}>{devices.map((device) => <option key={device.id} value={device.id}>{device.name}</option>)}</select></label>}<label>Bus ID<input value={busId} maxLength={64} disabled={action !== "create"} onChange={(event) => setBusId(event.target.value)} /></label>{action !== "delete" && <label>Bus name<input value={busName} maxLength={120} disabled={action === "setEnabled"} onChange={(event) => setBusName(event.target.value)} /></label>}<button type="button" className="secondary" onClick={() => void createPlan()}>Plan {action}</button>{plan && <button type="button" className="secondary" onClick={() => void applyPlan()}>Apply planned state</button>}</fieldset>{message && <p className="muted" role="status">{message}</p>}<p className="muted">Every change requires explicit plan/apply and <code>deviceAdministration</code>. The unavailable managed driver means no endpoint is synthesized or activated.</p></section>;
}

function NodeCard({ node, selected, onSelect }: { node: Node; selected: boolean; onSelect: () => void }) {
  return <article className={`node-card${selected ? " selected" : ""}`} tabIndex={0} aria-label={`${node.name}, ${node.kind}`} aria-current={selected ? "true" : undefined} onClick={onSelect} onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); onSelect(); } }}><span className="node-kind">{node.kind}</span><h3>{node.name}</h3><p>{node.ports.length} port{node.ports.length === 1 ? "" : "s"} - {node.enabled ? "enabled" : "disabled"}</p><div className="port-list">{node.ports.map((port) => <span key={port.name} className={`port ${port.direction}`}>{port.direction}: {port.name} - {port.channels}ch</span>)}</div></article>;
}

function NodeList({ session, selectedNodeId, onSelect, onRemoveConnection, onToggleConnection }: { session: import("@audiorouter/contracts").Session; selectedNodeId: string; onSelect: (id: string) => void; onRemoveConnection: (id: string) => void; onToggleConnection: (id: string, enabled: boolean) => void }) {
  const names = new Map(session.nodes.map((node) => [node.id, node.name]));
  return <div className="graph-list" aria-label="Graph nodes and connections"><ol aria-label="Nodes">{session.nodes.map((node) => <li key={node.id}><button type="button" className={node.id === selectedNodeId ? "selected" : ""} aria-current={node.id === selectedNodeId ? "true" : undefined} onClick={() => onSelect(node.id)}>{node.name} <small>{node.kind}, {node.enabled ? "enabled" : "disabled"}</small><span className="list-port-summary">{nodePortLabels(node).join(" · ")}</span></button></li>)}</ol><h3>Connections</h3>{session.edges.length === 0 ? <p className="muted">No committed connections.</p> : <ul aria-label="Connections">{session.edges.map((edge) => <li key={edge.id}><span>{names.get(edge.sourceNode) ?? edge.sourceNode}:{edge.sourcePort} → {names.get(edge.destinationNode) ?? edge.destinationNode}:{edge.destinationPort} <small>{edge.enabled ? "enabled" : "disabled"}</small></span><button type="button" className="secondary" onClick={() => onToggleConnection(edge.id, !edge.enabled)}>{edge.enabled ? "Disable" : "Enable"}</button><button type="button" className="secondary" onClick={() => onRemoveConnection(edge.id)}>Remove</button></li>)}</ul>}</div>;
}

function DraftConnectionList({ session, onRemove, onToggle }: { session: import("@audiorouter/contracts").Session; onRemove: (id: string) => void; onToggle: (id: string, enabled: boolean) => void }) {
  const names = new Map(session.nodes.map((node) => [node.id, node.name]));
  return <section className="draft-connections" aria-labelledby="draft-connections-heading"><h3 id="draft-connections-heading">Draft connections</h3>{session.edges.length === 0 ? <p className="muted">No draft connections.</p> : <ul aria-label="Draft connections">{session.edges.map((edge) => <li key={edge.id}><span>{names.get(edge.sourceNode) ?? edge.sourceNode}:{edge.sourcePort} → {names.get(edge.destinationNode) ?? edge.destinationNode}:{edge.destinationPort} <small>{edge.enabled ? "enabled" : "disabled"}</small></span><button type="button" className="secondary" onClick={() => onToggle(edge.id, !edge.enabled)}>{edge.enabled ? "Disable" : "Enable"}</button><button type="button" className="secondary" onClick={() => onRemove(edge.id)}>Remove</button></li>)}</ul>}</section>;
}

export function App({ backend = defaultBackend }: { backend?: UiBackend } = {}) {
  const [snapshotCache] = useState(() => new SnapshotCache());
  const [snapshotState, setSnapshotState] = useState(snapshotCache.current());
  const [selectedSessionId, setSelectedSessionId] = useState(demoSession.id);
  const [selectedNodeId, setSelectedNodeId] = useState(demoSession.nodes[0].id);
  const [draft, setDraft] = useState(demoSession);
  const [draftHistory, setDraftHistory] = useState<DraftHistory>({ past: [], future: [] });
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [routeInspection, setRouteInspection] = useState<RouteInspection | null>(null);
  const [recordings, setRecordings] = useState<import("@audiorouter/contracts").RecordingRow[]>([]);
  const [applications, setApplications] = useState<ApplicationRow[]>([]);
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [devicesError, setDevicesError] = useState<string | null>(null);
  const [applicationsError, setApplicationsError] = useState<string | null>(null);
  const [recordingsError, setRecordingsError] = useState<string | null>(null);
  const [previewMessage, setPreviewMessage] = useState<string | null>(null);
  const [recoveryMessage, setRecoveryMessage] = useState<string | null>(null);
  const [metadataTitles, setMetadataTitles] = useState<Record<string, string>>({});
  const [recordingSearch, setRecordingSearch] = useState("");
  const [pendingWarnings, setPendingWarnings] = useState<string[]>([]);
  const [acknowledgedWarnings, setAcknowledgedWarnings] = useState<Set<string>>(() => new Set());
  const [pendingOperation, setPendingOperation] = useState<string | null>(null);
  const [privacyMuted, setPrivacyMuted] = useState(true);
  const [listView, setListView] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<TemplateId>("gaming-discord");
  const [librarySearch, setLibrarySearch] = useState("");
  const [processors, setProcessors] = useState<ProcessorDescriptor[] | null>(null);
  const [processorError, setProcessorError] = useState<string | null>(null);
  const [presets, setPresets] = useState<import("@audiorouter/contracts").DiscoveryDocument["presets"] | null>(null);
  const [presetError, setPresetError] = useState<string | null>(null);
  const [theme, setTheme] = useState<ThemeMode>(() => readTheme(typeof window === "undefined" ? null : window.localStorage));
  const [connectionSource, setConnectionSource] = useState("");
  const [connectionDestination, setConnectionDestination] = useState("");
  const [createdSessions, setCreatedSessions] = useState<import("@audiorouter/contracts").Session[]>([]);
  const [listedSessions, setListedSessions] = useState<import("@audiorouter/contracts").Session[]>(backend.connected ? [] : demoSessions);
  const [sessionInventoryError, setSessionInventoryError] = useState<string | null>(null);
  const eventCursor = useRef({ backendEpoch: 0, sequence: 0 });
  useEffect(() => { let mounted = true; void snapshotCache.refresh(backend).then((nextState) => { if (mounted) { setSnapshotState(nextState); if (nextState.snapshot) eventCursor.current = { backendEpoch: nextState.snapshot.status.eventCursor.backendEpoch, sequence: nextState.snapshot.status.eventCursor.latestSequence }; } }); return () => { mounted = false; }; }, [backend, snapshotCache]);
  const refreshApplications = () => {
    void backend.listApplications().then((items) => { setApplications(items); setApplicationsError(null); }).catch((error) => { setApplications([]); setApplicationsError(formatUiError(error, "Application inventory unavailable")); });
  };
  const refreshDevices = () => {
    void backend.listDevices().then((items) => { setDevices(items); setDevicesError(null); }).catch((error) => { setDevices([]); setDevicesError(formatUiError(error, "Device inventory unavailable")); });
  };
  const refresh = () => {
    void snapshotCache.refresh(backend).then(setSnapshotState);
    void backend.listSessions().then((items) => { setListedSessions(items); setSessionInventoryError(null); }).catch((error) => { setSessionInventoryError(formatUiError(error, "Session inventory unavailable")); if (!backend.connected) setListedSessions(demoSessions); else setListedSessions([]); });
    void backend.listRecordings(session.id).then((items) => { setRecordings(items); setRecordingsError(null); }).catch((error) => { setRecordings([]); setRecordingsError(formatUiError(error, "Recording library unavailable")); });
    refreshApplications();
    refreshDevices();
  };
  const snapshot = snapshotState.snapshot;
  useEffect(() => { writeTheme(typeof window === "undefined" ? null : window.localStorage, theme); }, [theme]);
  useEffect(() => { if (snapshot) setPrivacyMuted(snapshot.status.privacyMute.muted); }, [snapshot]);
  const availableSessions = mergeSessionInventory(listedSessions, snapshot?.session ?? null, createdSessions);
  const session = availableSessions.find((item) => item.id === selectedSessionId) ?? availableSessions[0] ?? demoSession;
  const sessionRunning = snapshot?.status.activeSessionIds.includes(session.id) ?? false;
  useEffect(() => { setDraft(session); setDraftHistory({ past: [], future: [] }); setSelectedNodeId(session.nodes[0]?.id ?? ""); setConnectionSource(""); setConnectionDestination(""); setActionMessage(null); setRouteInspection(null); setPendingWarnings([]); setAcknowledgedWarnings(new Set()); setPendingOperation(null); }, [session]);
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
    void backend.listApplications().then((items) => { if (active) { setApplications(items); setApplicationsError(null); } }).catch((error) => { if (active) { setApplications([]); setApplicationsError(formatUiError(error, "Application inventory unavailable")); } });
    return () => { active = false; };
  }, [backend]);
  useEffect(() => {
    let active = true;
    void backend.listDevices().then((items) => { if (active) { setDevices(items); setDevicesError(null); } }).catch((error) => { if (active) { setDevices([]); setDevicesError(formatUiError(error, "Device inventory unavailable")); } });
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
        const result = await backend.subscribe(eventCursor.current.sequence, session.id, eventCursor.current.backendEpoch);
        if (!active) return;
        if (result.resyncRequired || result.events.length > 0) {
          const nextState = await snapshotCache.refresh(backend);
          if (active) {
            setSnapshotState(nextState);
            refreshApplications();
            refreshDevices();
            void backend.listSessions().then((items) => { if (active) { setListedSessions(items); setSessionInventoryError(null); } }).catch((error) => { if (active) setSessionInventoryError(formatUiError(error, "Session inventory unavailable")); });
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
  const selectedNode = draft.nodes.find((node) => node.id === selectedNodeId) ?? draft.nodes[0];
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
  const setupSteps = setupChecklist({ connected: backend.connected, audio: snapshot?.status.audio ?? null, storage: snapshot?.status.storage ?? null, deviceCount: devices.length, applicationCount: applications.length });
  const connectionLabel = backend.connected ? "Backend connected" : "Backend disconnected";
  const statusSummary = snapshot ? `${snapshot.status.audio} audio - ${snapshot.status.storage} storage - ${snapshot.status.sessionCount} session${snapshot.status.sessionCount === 1 ? "" : "s"}` : "Waiting for backend snapshot";
  const recordDraftChange = (next: import("@audiorouter/contracts").Session) => { setDraftHistory((history) => recordDraft(history, draft, next)); setDraft(next); };
  const undoDraft = () => { const transition = undoDraftHistory(draftHistory, draft); if (transition.current === draft) return; setDraftHistory(transition.history); setDraft(transition.current); setActionMessage("Undid the last draft change."); };
  const redoDraft = () => { const transition = redoDraftHistory(draftHistory, draft); if (transition.current === draft) return; setDraftHistory(transition.history); setDraft(transition.current); setActionMessage("Redid the draft change."); };
  const changeNodeFlag = (flag: "enabled" | "bypass", value: boolean) => { recordDraftChange(setNodeDraftFlag(draft, selectedNode.id, flag, value)); setActionMessage("Draft updated. Review and plan the changes before committing."); };
  const changeNodeName = (name: string) => { try { recordDraftChange(setNodeDraftName(draft, selectedNode.id, name)); setActionMessage("Node name draft updated. Review and plan the changes before committing."); } catch (error) { setActionMessage(formatUiError(error, "Unable to rename node.")); } };
  const changeNodeParameter = (name: string, value: boolean | number | string) => { const error = processorParameterError(processors, selectedNode.kind, name, value); if (error) { setActionMessage(`Draft rejected: ${error}.`); return; } recordDraftChange(setNodeDraftParameter(draft, selectedNode.id, name, value)); setActionMessage("Draft updated. Review and plan the changes before committing."); };
  const resetNodeParameters = () => { recordDraftChange(resetNodeDraftParameters(draft, selectedNode.id)); setActionMessage("Processor parameters reset in the draft. Review and plan the changes before committing."); };
  const changeSessionName = (name: string) => { try { recordDraftChange(setSessionDraftName(draft, name)); setActionMessage("Session name draft updated. Review and plan the change before committing."); } catch (error) { setActionMessage(formatUiError(error, "Unable to rename session.")); } };
  const planChanges = async () => {
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
    }
  };
  const commitAcknowledgedPlan = async () => {
    if (!pendingOperation || acknowledgedWarnings.size !== pendingWarnings.length) return;
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
    }
  };
  const inspectRoute = async () => { setActionMessage("Inspecting route..."); try { setRouteInspection(await backend.inspectRoute(selectedNode.id)); setActionMessage("Route inspection refreshed from the backend."); } catch (error) { setRouteInspection(null); setActionMessage(formatUiError(error, "Unable to inspect route.")); } };
  const previewRecording = async (recordingId: string) => { setPreviewMessage("Inspecting recording..."); try { const result = await backend.previewRecording(recordingId); setPreviewMessage(`${String(result.preview.status)} recording preview loaded.`); } catch (error) { setPreviewMessage(formatUiError(error, "Recording preview unavailable.")); } };
  const inspectRecovery = async (recordingId: string) => { setRecoveryMessage("Inspecting recorder recovery..."); try { const result = await backend.getRecordingRecovery(recordingId); setRecoveryMessage(result.status === "missing" ? "No persisted recovery checkpoint is available." : `Recovery checkpoint: ${result.checkpoint.state}.`); } catch (error) { setRecoveryMessage(formatUiError(error, "Recording recovery unavailable.")); } };
  const saveRecordingTitle = async (recordingId: string) => { try { const title = metadataTitles[recordingId]?.trim() ?? ""; await backend.setRecordingMetadata(recordingId, { title: title || null, idempotencyKey: uiIdempotencyKey("recording-metadata") }); setRecordings((current) => current.map((item) => item.id === recordingId ? { ...item, title: title || null } : item)); setPreviewMessage("Recording metadata saved; the audio file was unchanged."); } catch (error) { setPreviewMessage(formatUiError(error, "Unable to save recording metadata.")); } };
  const removeRecordingEntry = async (recordingId: string) => { if (!window.confirm("Remove this library entry? The audio file will be preserved.")) return; try { await backend.removeRecordingEntry(recordingId, uiIdempotencyKey("recording-entry-remove")); setRecordings((current) => current.filter((item) => item.id !== recordingId)); setPreviewMessage("Library entry removed; the audio file was preserved."); } catch (error) { setPreviewMessage(formatUiError(error, "Unable to remove recording entry.")); } };
  const renameRecording = async (recordingId: string, newPath: string) => { try { const result = await backend.renameRecording(recordingId, newPath.trim(), uiIdempotencyKey("recording-rename")); setRecordings((current) => current.map((item) => item.id === recordingId ? { ...item, path: result.path, missing: false } : item)); setPreviewMessage("Recording renamed within the approved directory."); } catch (error) { setPreviewMessage(formatUiError(error, "Unable to rename recording.")); } };
  const revealRecording = async (recordingId: string) => { try { const result = await backend.revealRecording(recordingId); setPreviewMessage(result.revealed ? "Recording revealed by the operating system." : "Recording is missing; no operating-system action was performed."); } catch (error) { setPreviewMessage(formatUiError(error, "Unable to reveal recording.")); } };
  const recycleRecording = async (recordingId: string, confirm: boolean) => { try { const result = await backend.recycleRecording(recordingId, confirm, confirm ? uiIdempotencyKey("recording-recycle") : undefined); setPreviewMessage(result.fileAction === "recycled" ? "Recording recycled." : result.fileAction === "recycle" ? "Recycle preview loaded; confirmation is still required." : `Recording was not recycled: ${result.reason}.`); if (result.fileAction === "recycled") setRecordings((current) => current.map((item) => item.id === recordingId ? { ...item, missing: true } : item)); } catch (error) { setPreviewMessage(formatUiError(error, "Unable to recycle recording.")); } };
  const togglePrivacyMute = async () => { const next = !privacyMuted; setActionMessage(next ? "Enabling privacy mute..." : "Disabling privacy mute..."); try { await backend.setPrivacyMute(next, uiIdempotencyKey("privacy-mute")); setPrivacyMuted(next); setActionMessage(next ? "Privacy mute enabled." : "Privacy mute disabled."); } catch (error) { setPrivacyMuted(true); setActionMessage(formatUiError(error, "Unable to change privacy mute.")); } };
  const clearRecoverySafeMode = async () => { setActionMessage("Clearing recovery safe mode..."); try { await backend.clearRecoverySafeMode(uiIdempotencyKey("recovery-clear")); await refresh(); setActionMessage("Recovery safe mode cleared."); } catch (error) { setActionMessage(formatUiError(error, "Unable to clear recovery safe mode.")); } };
  const createSession = async () => { const name = window.prompt("New session name", "New session")?.trim(); if (!name) return; const id = `session-${Date.now()}`; try { const result = await backend.createSession({ ...demoSession, id, name, revision: 0, nodes: demoSession.nodes.map((node) => ({ ...node, parameters: { ...node.parameters } })), edges: [...demoSession.edges] }, uiIdempotencyKey("session-create")); setCreatedSessions((current) => [...current, result.session]); setSelectedSessionId(result.session.id); setActionMessage(`Created stopped session ${result.session.name}.`); } catch (error) { setActionMessage(formatUiError(error, "Unable to create session.")); } };
  const duplicateSession = async () => { const id = `session-copy-${Date.now()}`; const name = `${session.name} (copy)`; try { const result = await backend.duplicateSession(session.id, id, name, uiIdempotencyKey("session-duplicate")); setCreatedSessions((current) => [...current, result.session]); setSelectedSessionId(result.session.id); setActionMessage(`Duplicated stopped session ${result.session.name}.`); } catch (error) { setActionMessage(formatUiError(error, "Unable to duplicate session.")); } };
  const deleteSession = async () => { if (!window.confirm(`Delete stopped session “${session.name}”?`)) return; try { await backend.deleteSession(session.id, uiIdempotencyKey("session-delete")); setCreatedSessions((current) => current.filter((item) => item.id !== session.id)); const fallback = availableSessions.find((item) => item.id !== session.id); if (fallback) setSelectedSessionId(fallback.id); setActionMessage(`Deleted session ${session.name}.`); } catch (error) { setActionMessage(formatUiError(error, "Unable to delete session.")); } };
  const startSession = async () => { setActionMessage("Starting session..."); try { const result = await backend.startSession(session.id, uiIdempotencyKey("session-start")); await refresh(); setActionMessage(`Session is running (generation ${result.generation}).`); } catch (error) { setActionMessage(formatUiError(error, "Unable to start session.")); } };
  const stopSession = async () => { setActionMessage("Stopping session..."); try { await backend.stopSession(session.id, uiIdempotencyKey("session-stop")); await refresh(); setActionMessage("Session stopped."); } catch (error) { setActionMessage(formatUiError(error, "Unable to stop session.")); } };
  const addLibraryNode = (kind: LibraryNodeKind) => { const next = appendLibraryNode(draft, kind); recordDraftChange(next); setSelectedNodeId(next.nodes[next.nodes.length - 1].id); setActionMessage(`${next.nodes[next.nodes.length - 1].name} added to the draft. Review and plan the changes before committing.`); };
  const addConnection = () => { const source = decodePort(connectionSource); const destination = decodePort(connectionDestination); if (!source || !destination) { setActionMessage("Choose an output and input port first."); return; } try { const next = appendDraftConnection(draft, source.nodeId, source.portName, destination.nodeId, destination.portName); recordDraftChange(next); setActionMessage("Connection added to the draft. Review and plan the changes before committing."); } catch (error) { setActionMessage(formatUiError(error, "Unable to add connection.")); } };
  const removeConnection = (edgeId: string) => { try { recordDraftChange(removeDraftConnection(draft, edgeId)); setActionMessage("Connection removed from the draft. Review and plan the changes before committing."); } catch (error) { setActionMessage(formatUiError(error, "Unable to remove connection.")); } };
  const toggleConnection = (edgeId: string, enabled: boolean) => { try { recordDraftChange(setDraftConnectionEnabled(draft, edgeId, enabled)); setActionMessage(`Connection ${enabled ? "enabled" : "disabled"} in the draft. Review and plan the changes before committing.`); } catch (error) { setActionMessage(formatUiError(error, "Unable to change connection state.")); } };
  const removeSelectedNode = () => { if (!window.confirm(`Remove node “${selectedNode.name}” and its draft connections?`)) return; try { const next = removeDraftNode(draft, selectedNode.id); recordDraftChange(next); setSelectedNodeId(next.nodes[0]?.id ?? ""); setActionMessage("Node removed from the draft. Review and plan the changes before committing."); } catch (error) { setActionMessage(formatUiError(error, "Unable to remove node.")); } };
  const duplicateSelectedNode = () => { try { const next = duplicateDraftNode(draft, selectedNode.id); const copy = next.nodes[next.nodes.length - 1]; recordDraftChange(next); setSelectedNodeId(copy.id); setActionMessage(`${copy.name} added to the draft without connections. Review and plan the changes before committing.`); } catch (error) { setActionMessage(formatUiError(error, "Unable to duplicate node.")); } };
  const applyTemplate = () => { const next = templateSession(selectedTemplate); recordDraftChange({ ...next, id: draft.id, revision: draft.revision }); setSelectedNodeId(next.nodes[0]?.id ?? ""); setActionMessage("Template loaded into the draft. Review device bindings and plan the changes before committing."); };
  const lifecycleActions = <section className="panel lifecycle-panel" aria-labelledby="lifecycle-heading"><h2 id="lifecycle-heading">Session lifecycle</h2><p className="muted">Starting a session uses the shared authorized backend lifecycle API.</p><button type="button" className="secondary" onClick={() => void (sessionRunning ? stopSession() : startSession())} disabled={!backend.connected}>{sessionRunning ? "Stop session" : "Start session"}</button></section>;
  return <div className={`app-shell theme-${theme}`}>{lifecycleActions}<RecorderActions backend={backend} sessionId={session.id} connected={backend.connected} /><RecordingActions recordings={recordings} connected={backend.connected} onRename={renameRecording} onReveal={revealRecording} onRecycle={recycleRecording} /><VirtualDeviceLifecyclePanel backend={backend} />
    <header className="topbar"><div><p className="eyebrow">AudioRouter</p><h1>Routing workspace</h1></div><div className="status-cluster" aria-live="polite"><span className={`status-dot${backend.connected ? "" : " disconnected"}`} aria-hidden="true" /><span>{connectionLabel}</span><span className="status-detail">{statusSummary}</span><label className="theme-picker">Theme<select aria-label="Color theme" value={theme} onChange={(event) => setTheme(event.target.value as ThemeMode)}><option value="dark">Dark</option><option value="light">Light</option><option value="high-contrast">High contrast</option></select></label><button type="button" onClick={refresh}>Reconnect</button></div></header>
      <div className="workspace-grid"><aside className="sidebar" aria-label="Sessions"><div className="section-heading"><h2>Sessions</h2><button type="button" aria-label="Create session" onClick={() => void createSession()} disabled={!backend.connected} title="Session creation requires the connected backend">+</button></div>{sessionInventoryError && <p className="muted" role="status">Session inventory unavailable: {sessionInventoryError}</p>}<label className="session-picker">Preview session<select value={session.id} onChange={(event) => setSelectedSessionId(event.target.value)}>{availableSessions.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</select></label>{availableSessions.map((item) => { const running = snapshot?.status.activeSessionIds.includes(item.id) ?? false; return <button type="button" key={item.id} className={`session-item${item.id === session.id ? " selected" : ""}`} aria-current={item.id === session.id ? "true" : undefined} onClick={() => setSelectedSessionId(item.id)}><span>{item.name}</span><small>{running ? "Running" : "Stopped"} - rev {item.revision}</small></button>; })}<div className="sidebar-note"><strong>Safe startup</strong><p>Monitoring is muted and recording is unarmed until you explicitly start them.</p></div></aside>
      <main className="main-content"><section className="workspace-title"><div><p className="eyebrow">{sessionRunning ? "Running session" : "Stopped session"}</p><label className="session-name">Session name<input value={draft.name} maxLength={120} disabled={!backend.connected} onChange={(event) => changeSessionName(event.target.value)} /></label><p className="muted">Revision {session.revision} - {backend.connected ? "draft changes require plan and commit" : "changes are presentation-only in this preview"}</p></div><div className="actions"><button type="button" className="secondary" onClick={() => void duplicateSession()} disabled={!backend.connected}>Duplicate</button><button type="button" className="secondary" onClick={() => void deleteSession()} disabled={!backend.connected}>Delete</button><button type="button" className="secondary" onClick={undoDraft} disabled={!backend.connected || draftHistory.past.length === 0}>Undo draft</button><button type="button" className="secondary" onClick={redoDraft} disabled={!backend.connected || draftHistory.future.length === 0}>Redo draft</button><button type="button" className="secondary" onClick={() => { setDraft(session); setDraftHistory({ past: [], future: [] }); setActionMessage("Draft discarded."); }} disabled={!backend.connected}>Discard draft</button><button type="button" className="primary" onClick={() => void planChanges()} disabled={!backend.connected}>Plan changes</button></div></section>
        {actionMessage && <p className="muted" role="status" aria-live="polite">{actionMessage}</p>}{pendingWarnings.length > 0 && <section className="warning-panel" aria-labelledby="warning-heading"><h2 id="warning-heading">Plan warnings</h2>{pendingWarnings.map((warning) => <label key={warning}><input type="checkbox" checked={acknowledgedWarnings.has(warning)} onChange={(event) => setAcknowledgedWarnings((current) => { const next = new Set(current); if (event.target.checked) next.add(warning); else next.delete(warning); return next; })} /> I acknowledge: {warning}</label>)}<button type="button" className="primary" disabled={acknowledgedWarnings.size !== pendingWarnings.length} onClick={() => void commitAcknowledgedPlan()}>Commit acknowledged plan</button></section>}{snapshotState.stale && snapshotState.error && <p className="muted" role="status">Last known backend state is stale: {snapshotState.error}</p>}
        <section className="notice" role="status"><strong>{backend.connected ? "Connected editor" : "Read-only preview"}</strong><span>{backend.connected ? "Drafts are validated and committed through the authoritative backend." : "The control backend is disconnected. No route, device, or recording action can be applied."}</span></section>
        <section className="panel setup-panel" aria-labelledby="setup-heading"><div className="section-heading"><div><p className="eyebrow">Guided setup</p><h2 id="setup-heading">Readiness checklist</h2></div><span className="badge">{setupSteps.filter((step) => step.state === "ready").length}/{setupSteps.length} ready</span></div><ul aria-label="Guided setup readiness">{setupSteps.map((step) => <li key={step.id} className={`setup-step ${step.state}`}><strong>{step.label}</strong><span>{step.detail}</span></li>)}</ul><p className="muted">External applications such as Discord and OBS must be pointed to AudioRouter through their own settings; this checklist never changes them automatically.</p></section>
        <ProcessorCatalog processors={processors} error={processorError} node={selectedNode} backend={backend} /><PresetCatalog presets={presets} error={presetError} /><PluginScanPanel backend={backend} /><StartupPanel backend={backend} />
        <ApplicationIdentityPanel applications={applications} />
        <section className="panel recovery-panel" aria-labelledby="recovery-heading"><div className="section-heading"><div><p className="eyebrow">Crash recovery</p><h2 id="recovery-heading">{snapshot?.status.recovery.safeMode ? "Safe mode is active" : "Normal startup mode"}</h2></div><span className="badge">{snapshot?.status.recovery.recentCrashes ?? 0} recent crash{(snapshot?.status.recovery.recentCrashes ?? 0) === 1 ? "" : "es"}</span></div><p className="muted">{snapshot?.status.recovery.persistence === "durable" ? "Recovery state is persisted by the backend." : "Recovery state is held in memory for this preview."}</p><button type="button" className="secondary" onClick={() => void clearRecoverySafeMode()} disabled={!backend.connected || !snapshot?.status.recovery.safeMode}>Clear safe mode</button></section>
        <section className="panel" aria-labelledby="applications-heading"><div className="section-heading"><div><p className="eyebrow">Audio sources</p><h2 id="applications-heading">Applications</h2></div><span className="badge">{applicationsError ? "unavailable" : applications.length}</span></div>{applicationsError ? <p className="muted">Application inventory unavailable: {applicationsError}</p> : applications.length === 0 ? <p className="muted">No process audio sessions are exposed by the backend snapshot.</p> : <ul aria-label="Running audio applications">{applications.map((application) => <li key={`${application.processId}-${application.creationTime100ns ?? "unknown"}`}><strong>{application.audioDisplayNames[0] ?? application.executable}</strong> <small>{application.executable} · PID {application.processId} · {application.audioActivity} · {application.captureCapability === "observed" ? "capture observed" : "capture not observed"} · render sessions {application.renderSessionCount}</small></li>)}</ul>}</section>
        <section className="panel" aria-labelledby="devices-heading"><div className="section-heading"><div><p className="eyebrow">Windows endpoints</p><h2 id="devices-heading">Devices</h2></div><span className="badge">{devicesError ? "unavailable" : devices.length}</span></div>{devicesError ? <p className="muted">Device inventory unavailable: {devicesError}</p> : devices.length === 0 ? <p className="muted">No active endpoint metadata is exposed by the backend.</p> : <ul aria-label="Active audio devices">{devices.map((device) => <li key={device.id}><strong>{device.direction === "capture" ? "Capture" : "Render"}</strong> <small>{device.id} · {device.format.sampleRateHz} Hz · {device.format.channels} ch · {device.periods.default100ns / 10000} ms period</small></li>)}</ul>}</section>
        <section className="canvas-panel" aria-labelledby="canvas-heading"><div className="section-heading"><div><p className="eyebrow">Signal flow</p><h2 id="canvas-heading">{listView ? "Graph list" : "Canvas"}</h2></div><button type="button" className="secondary" aria-pressed={listView} onClick={() => setListView((current) => !current)}>{listView ? "Canvas view" : "List view"}</button></div>{listView ? <NodeList session={draft} selectedNodeId={selectedNode.id} onSelect={setSelectedNodeId} onRemoveConnection={removeConnection} onToggleConnection={toggleConnection} /> : <><SessionFlowCanvas session={draft} selectedNodeId={selectedNode.id} onSelect={setSelectedNodeId} /><DraftConnectionList session={draft} onRemove={removeConnection} onToggle={toggleConnection} /></>}<fieldset className="connection-editor" disabled={!backend.connected}><legend>Add connection to draft</legend><label>Output<select aria-label="Source output port" value={connectionSource} onChange={(event) => setConnectionSource(event.target.value)}><option value="">Choose source</option>{outputPorts.map((port) => <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>{port.nodeName} · {port.portName} · {port.channels}ch</option>)}</select></label><span aria-hidden="true">→</span><label>Input<select aria-label="Destination input port" value={connectionDestination} onChange={(event) => setConnectionDestination(event.target.value)}><option value="">Choose destination</option>{inputPorts.map((port) => <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>{port.nodeName} · {port.portName} · {port.channels}ch</option>)}</select></label><button type="button" className="secondary" onClick={addConnection}>Add connection</button></fieldset></section>
        <section className="panel inspector" aria-labelledby="inspector-heading"><div className="section-heading"><div><p className="eyebrow">Selected node</p><h2 id="inspector-heading">{selectedNode.name}</h2></div><span className="badge">{selectedNode.kind}</span></div><div className="inspector-grid"><label>Node name<input type="text" maxLength={120} value={selectedNode.name} disabled={!backend.connected} onChange={(event) => changeNodeName(event.target.value)} /></label><label>Enabled<input type="checkbox" checked={selectedNode.enabled} disabled={!backend.connected} onChange={(event) => changeNodeFlag("enabled", event.target.checked)} /></label><label>Bypass<input type="checkbox" checked={selectedNode.bypass} disabled={!backend.connected} onChange={(event) => changeNodeFlag("bypass", event.target.checked)} /></label><ProcessorParameterEditor node={selectedNode} processors={processors} connected={backend.connected} onChange={changeNodeParameter} />{processors?.some((processor) => processor.id === selectedNode.kind && processor.parameters.length > 0) && <button type="button" className="secondary" onClick={resetNodeParameters} disabled={!backend.connected}>Reset parameters</button>}<button type="button" className="secondary" onClick={duplicateSelectedNode} disabled={!backend.connected}>Duplicate node to draft</button><button type="button" className="secondary" onClick={removeSelectedNode} disabled={!backend.connected}>Remove node from draft</button><button type="button" onClick={() => void togglePrivacyMute()} disabled={!backend.connected} aria-pressed={privacyMuted}>{privacyMuted ? "Privacy mute enabled" : "Enable privacy mute"}</button><p className="muted">{backend.connected ? "Changes are local drafts until Plan changes is committed. Privacy mute is an immediate safety latch." : "Controls are disabled while disconnected. Selection is local presentation state only."}</p></div></section>
        <section className="lower-grid"><div className="panel"><div className="section-heading"><h2>Library</h2><button type="button" className="secondary" onClick={() => document.getElementById("library-search")?.focus()}>Search</button></div><label className="library-search">Search nodes<input id="library-search" type="search" value={librarySearch} onChange={(event) => setLibrarySearch(event.target.value.slice(0, 80))} placeholder="Gain, effect, meter..." /></label><div className="library-grid">{visibleLibraryEntries.length === 0 ? <p className="muted">No library entries match this search.</p> : visibleLibraryEntries.map((entry) => entry.kind ? <button type="button" key={entry.id} aria-label={libraryEntryAccessibleLabel(entry)} onClick={() => addLibraryNode(entry.kind!)} disabled={!backend.connected}>{entry.label}<small>{entry.category} · add to draft</small></button> : <button type="button" key={entry.id} aria-label={libraryEntryAccessibleLabel(entry)} disabled title={entry.unavailableReason}>{entry.label}<small>{entry.category} · unavailable: {entry.unavailableReason}</small></button>)}</div><p className="muted">Built-in processors are added as local drafts; use Plan changes to validate and commit them.</p><div className="template-picker"><label>Guided template<select aria-label="Guided setup template" value={selectedTemplate} onChange={(event) => setSelectedTemplate(event.target.value as TemplateId)}><option value="gaming-discord">Gaming + Discord</option><option value="processed-microphone">Processed microphone</option><option value="mix-minus">Mix-minus conversation</option></select></label><button type="button" className="secondary" onClick={applyTemplate} disabled={!backend.connected}>Load template to draft</button><small>Templates remain stopped and require device review before commit.</small></div></div><div className="panel"><div className="section-heading"><h2>Recordings</h2><span className="badge">{recordingsError ? "unavailable" : `${visibleRecordings.length}${recordingSearch.trim() ? ` of ${recordings.length}` : ""} file${visibleRecordings.length === 1 ? "" : "s"}`}</span></div><label className="recording-search">Search recordings<input id="recording-search" type="search" value={recordingSearch} onChange={(event) => setRecordingSearch(event.target.value.slice(0, 160))} placeholder="Title, path, or status" /></label>{recordingsError ? <p className="muted">Recording library unavailable: {recordingsError}</p> : recordings.length === 0 ? <p className="muted">No recording has been armed. Completed recordings will appear here with path and status.</p> : visibleRecordings.length === 0 ? <p className="muted">No recording matches this search.</p> : visibleRecordings.map((recording) => <p className="muted" key={recording.id}><label>Title <input aria-label={`Title for ${recording.id}`} value={metadataTitles[recording.id] ?? recording.title ?? ""} onChange={(event) => setMetadataTitles((current) => ({ ...current, [recording.id]: event.target.value }))} /></label> - {recording.state}{recording.missing ? " - missing" : ""}<br /><small>{recording.path}</small> <button type="button" className="secondary" onClick={() => void saveRecordingTitle(recording.id)} disabled={!backend.connected}>Save metadata</button> <button type="button" className="secondary" onClick={() => void previewRecording(recording.id)} disabled={!backend.connected}>Preview</button> <button type="button" className="secondary" onClick={() => void inspectRecovery(recording.id)} disabled={!backend.connected}>Recovery</button> <button type="button" className="secondary" onClick={() => void removeRecordingEntry(recording.id)} disabled={!backend.connected}>Remove entry</button></p>)}{previewMessage && <p className="muted" role="status">{previewMessage}</p>}{recoveryMessage && <p className="muted" role="status">{recoveryMessage}</p>}</div></section>
        <section className="panel route-inspection" aria-labelledby="route-heading"><div className="section-heading"><div><p className="eyebrow">Backend explanation</p><h2 id="route-heading">Receives audio from</h2></div><button type="button" className="secondary" onClick={() => void inspectRoute()}>Refresh</button></div>{routeInspection === null ? <p className="muted">No route inspection loaded. The backend is authoritative; no path is inferred.</p> : <p className="muted">{routeInspection.reachable ? `${routeInspection.paths.length} reachable path${routeInspection.paths.length === 1 ? "" : "s"}${routeInspection.complete ? "" : " (partial; path limit reached)"} reported to ${draftNodeNames.get(routeInspection.destinationNode) ?? routeInspection.destinationNode}.` : "No reachable route reported by the backend."}</p>}{routeInspection?.paths.map((path, index) => <p key={`${path.nodes.join("-")}-${index}`} className="muted">Path {index + 1}: {routeNodeLabels(draft, path.nodes).join(" → ") || "empty"} ({path.edges.length} edge{path.edges.length === 1 ? "" : "s"}; {routeLatencyText(path.latencySamples)})<br /><small>Channel map: {path.channelMaps.length === 0 ? "none" : path.channelMaps.map((row) => `[${row.join(", ")}]`).join(" ")}</small></p>)}</section>
      </main></div>
  </div>;
}
