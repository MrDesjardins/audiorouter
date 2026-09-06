import { useEffect, useRef, useState } from "react";
import type { Node, RouteInspection } from "@audiorouter/contracts";
import { createDisconnectedBackend, SnapshotCache, type UiBackend } from "./backend";
import { applyGraphDraft, setNodeDraftFlag, setNodeDraftParameter } from "./draft";
import { demoSession, demoSessions } from "./fixtures";

const defaultBackend = createDisconnectedBackend();

function NodeCard({ node, selected, onSelect }: { node: Node; selected: boolean; onSelect: () => void }) {
  return <article className={`node-card${selected ? " selected" : ""}`} tabIndex={0} aria-label={`${node.name}, ${node.kind}`} aria-current={selected ? "true" : undefined} onClick={onSelect} onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); onSelect(); } }}><span className="node-kind">{node.kind}</span><h3>{node.name}</h3><p>{node.ports.length} port{node.ports.length === 1 ? "" : "s"} - {node.enabled ? "enabled" : "disabled"}</p><div className="port-list">{node.ports.map((port) => <span key={port.name} className={`port ${port.direction}`}>{port.direction}: {port.name} - {port.channels}ch</span>)}</div></article>;
}

function NodeList({ session, selectedNodeId, onSelect }: { session: import("@audiorouter/contracts").Session; selectedNodeId: string; onSelect: (id: string) => void }) {
  const names = new Map(session.nodes.map((node) => [node.id, node.name]));
  return <div className="graph-list" aria-label="Graph nodes and connections"><ol aria-label="Nodes">{session.nodes.map((node) => <li key={node.id}><button type="button" className={node.id === selectedNodeId ? "selected" : ""} aria-current={node.id === selectedNodeId ? "true" : undefined} onClick={() => onSelect(node.id)}>{node.name} <small>{node.kind}, {node.enabled ? "enabled" : "disabled"}</small></button></li>)}</ol><h3>Connections</h3>{session.edges.length === 0 ? <p className="muted">No committed connections.</p> : <ul aria-label="Connections">{session.edges.map((edge) => <li key={edge.id}>{names.get(edge.sourceNode) ?? edge.sourceNode}:{edge.sourcePort} → {names.get(edge.destinationNode) ?? edge.destinationNode}:{edge.destinationPort} <small>{edge.enabled ? "enabled" : "disabled"}</small></li>)}</ul>}</div>;
}

export function App({ backend = defaultBackend }: { backend?: UiBackend } = {}) {
  const [snapshotCache] = useState(() => new SnapshotCache());
  const [snapshotState, setSnapshotState] = useState(snapshotCache.current());
  const [selectedSessionId, setSelectedSessionId] = useState(demoSession.id);
  const [selectedNodeId, setSelectedNodeId] = useState(demoSession.nodes[0].id);
  const [draft, setDraft] = useState(demoSession);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [routeInspection, setRouteInspection] = useState<RouteInspection | null>(null);
  const [recordings, setRecordings] = useState<import("@audiorouter/contracts").RecordingRow[]>([]);
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
  const eventCursor = useRef({ backendEpoch: 0, sequence: 0 });
  useEffect(() => { let mounted = true; void snapshotCache.refresh(backend).then((nextState) => { if (mounted) setSnapshotState(nextState); }); return () => { mounted = false; }; }, [backend, snapshotCache]);
  const refresh = () => {
    void snapshotCache.refresh(backend).then(setSnapshotState);
    void backend.listRecordings(session.id).then((items) => { setRecordings(items); setRecordingsError(null); }).catch((error) => { setRecordings([]); setRecordingsError(error instanceof Error ? error.message : "Recording library unavailable"); });
  };
  const snapshot = snapshotState.snapshot;
  const availableSessions = snapshot ? [snapshot.session, ...demoSessions.filter((item) => item.id !== snapshot.session.id)] : demoSessions;
  const session = availableSessions.find((item) => item.id === selectedSessionId) ?? availableSessions[0];
  useEffect(() => { setDraft(session); setSelectedNodeId(session.nodes[0]?.id ?? ""); setActionMessage(null); setRouteInspection(null); setPendingWarnings([]); setAcknowledgedWarnings(new Set()); setPendingOperation(null); }, [session]);
  useEffect(() => {
    let active = true;
    void backend.listRecordings(session.id).then((items) => { if (active) { setRecordings(items); setRecordingsError(null); } }).catch((error) => { if (active) { setRecordings([]); setRecordingsError(error instanceof Error ? error.message : "Recording library unavailable"); } });
    return () => { active = false; };
  }, [backend, session.id]);
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
        eventCursor.current = { backendEpoch: result.backendEpoch, sequence: result.nextSequence };
        if (result.resyncRequired || result.events.length > 0) {
          const nextState = await snapshotCache.refresh(backend);
          if (active) setSnapshotState(nextState);
        }
      } catch (error) {
        if (active) setSnapshotState((current) => ({ ...current, stale: true, error: error instanceof Error ? error.message : "Event subscription failed" }));
      } finally {
        polling = false;
      }
    };
    void poll();
    const timer = window.setInterval(() => void poll(), 1000);
    return () => { active = false; window.clearInterval(timer); };
  }, [backend, session.id, snapshotCache]);
  const selectedNode = draft.nodes.find((node) => node.id === selectedNodeId) ?? draft.nodes[0];
  const visibleRecordings = recordings.filter((recording) => {
    const query = recordingSearch.trim().toLocaleLowerCase();
    if (!query) return true;
    return [recording.id, recording.title, recording.artist, recording.comment, recording.path]
      .filter((value): value is string => value !== null)
      .some((value) => value.toLocaleLowerCase().includes(query));
  });
  const connectionLabel = backend.connected ? "Backend connected" : "Backend disconnected";
  const statusSummary = snapshot ? `${snapshot.status.audio} audio - ${snapshot.status.storage} storage - ${snapshot.status.sessionCount} session${snapshot.status.sessionCount === 1 ? "" : "s"}` : "Waiting for backend snapshot";
  const changeNodeFlag = (flag: "enabled" | "bypass", value: boolean) => { setDraft((current) => setNodeDraftFlag(current, selectedNode.id, flag, value)); setActionMessage("Draft updated. Review and plan the changes before committing."); };
  const changeNodeParameter = (name: string, value: boolean | number) => { if (typeof value === "number" && !Number.isFinite(value)) return; setDraft((current) => setNodeDraftParameter(current, selectedNode.id, name, value)); setActionMessage("Draft updated. Review and plan the changes before committing."); };
  const planChanges = async () => {
    setActionMessage("Planning changes...");
    try {
      const operation = `ui-${Date.now()}`;
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
    } catch (error) { setActionMessage(error instanceof Error ? error.message : "Unable to apply graph changes."); }
  };
  const commitAcknowledgedPlan = async () => {
    if (!pendingOperation || acknowledgedWarnings.size !== pendingWarnings.length) return;
    setActionMessage("Replanning acknowledged changes...");
    try {
      const result = await applyGraphDraft(backend, draft, pendingOperation, [...acknowledgedWarnings]);
      setPendingWarnings([]); setAcknowledgedWarnings(new Set()); setPendingOperation(null);
      setActionMessage(`Committed revision ${result.revision}. Reconnect to refresh the authoritative view.`);
    } catch (error) { setActionMessage(error instanceof Error ? error.message : "Unable to commit acknowledged changes."); }
  };
  const inspectRoute = async () => { setActionMessage("Inspecting route..."); try { setRouteInspection(await backend.inspectRoute(selectedNode.id)); setActionMessage("Route inspection refreshed from the backend."); } catch (error) { setRouteInspection(null); setActionMessage(error instanceof Error ? error.message : "Unable to inspect route."); } };
  const previewRecording = async (recordingId: string) => { setPreviewMessage("Inspecting recording..."); try { const result = await backend.previewRecording(recordingId); setPreviewMessage(`${String(result.status ?? "available")} recording preview loaded.`); } catch (error) { setPreviewMessage(error instanceof Error ? error.message : "Recording preview unavailable."); } };
  const inspectRecovery = async (recordingId: string) => { setRecoveryMessage("Inspecting recorder recovery..."); try { const result = await backend.getRecordingRecovery(recordingId); setRecoveryMessage(result.present === false ? "No persisted recovery checkpoint is available." : `Recovery checkpoint: ${String(result.state ?? "available")}.`); } catch (error) { setRecoveryMessage(error instanceof Error ? error.message : "Recording recovery unavailable."); } };
  const saveRecordingTitle = async (recordingId: string) => { try { const title = metadataTitles[recordingId]?.trim() ?? ""; await backend.setRecordingMetadata(recordingId, { title: title || null }); setRecordings((current) => current.map((item) => item.id === recordingId ? { ...item, title: title || null } : item)); setPreviewMessage("Recording metadata saved; the audio file was unchanged."); } catch (error) { setPreviewMessage(error instanceof Error ? error.message : "Unable to save recording metadata."); } };
  const removeRecordingEntry = async (recordingId: string) => { if (!window.confirm("Remove this library entry? The audio file will be preserved.")) return; try { await backend.removeRecordingEntry(recordingId); setRecordings((current) => current.filter((item) => item.id !== recordingId)); setPreviewMessage("Library entry removed; the audio file was preserved."); } catch (error) { setPreviewMessage(error instanceof Error ? error.message : "Unable to remove recording entry."); } };
  const togglePrivacyMute = async () => { const next = !privacyMuted; setActionMessage(next ? "Enabling privacy mute..." : "Disabling privacy mute..."); try { await backend.setPrivacyMute(next); setPrivacyMuted(next); setActionMessage(next ? "Privacy mute enabled." : "Privacy mute disabled."); } catch (error) { setPrivacyMuted(true); setActionMessage(error instanceof Error ? error.message : "Unable to change privacy mute."); } };
  return <div className="app-shell">
    <header className="topbar"><div><p className="eyebrow">AudioRouter</p><h1>Routing workspace</h1></div><div className="status-cluster" aria-live="polite"><span className={`status-dot${backend.connected ? "" : " disconnected"}`} aria-hidden="true" /><span>{connectionLabel}</span><span className="status-detail">{statusSummary}</span><button type="button" onClick={refresh}>Reconnect</button></div></header>
    <div className="workspace-grid"><aside className="sidebar" aria-label="Sessions"><div className="section-heading"><h2>Sessions</h2><button type="button" aria-label="Create session" disabled={!backend.connected} title="Session creation requires the connected backend">+</button></div><label className="session-picker">Preview session<select value={session.id} onChange={(event) => setSelectedSessionId(event.target.value)}>{availableSessions.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</select></label>{availableSessions.map((item) => <button type="button" key={item.id} className={`session-item${item.id === session.id ? " selected" : ""}`} aria-current={item.id === session.id ? "true" : undefined} onClick={() => setSelectedSessionId(item.id)}><span>{item.name}</span><small>Stopped - rev {item.revision}</small></button>)}<div className="sidebar-note"><strong>Safe startup</strong><p>Monitoring is muted and recording is unarmed until you explicitly start them.</p></div></aside>
      <main className="main-content"><section className="workspace-title"><div><p className="eyebrow">Stopped session</p><h2>{session.name}</h2><p className="muted">Revision {session.revision} - {backend.connected ? "draft changes require plan and commit" : "changes are presentation-only in this preview"}</p></div><div className="actions"><button type="button" className="secondary" onClick={() => { setDraft(session); setActionMessage("Draft discarded."); }} disabled={!backend.connected}>Undo</button><button type="button" className="primary" onClick={() => void planChanges()} disabled={!backend.connected}>Plan changes</button></div></section>
        {actionMessage && <p className="muted" role="status" aria-live="polite">{actionMessage}</p>}{pendingWarnings.length > 0 && <section className="warning-panel" aria-labelledby="warning-heading"><h2 id="warning-heading">Plan warnings</h2>{pendingWarnings.map((warning) => <label key={warning}><input type="checkbox" checked={acknowledgedWarnings.has(warning)} onChange={(event) => setAcknowledgedWarnings((current) => { const next = new Set(current); if (event.target.checked) next.add(warning); else next.delete(warning); return next; })} /> I acknowledge: {warning}</label>)}<button type="button" className="primary" disabled={acknowledgedWarnings.size !== pendingWarnings.length} onClick={() => void commitAcknowledgedPlan()}>Commit acknowledged plan</button></section>}{snapshotState.stale && snapshotState.error && <p className="muted" role="status">Last known backend state is stale: {snapshotState.error}</p>}
        <section className="notice" role="status"><strong>{backend.connected ? "Connected editor" : "Read-only preview"}</strong><span>{backend.connected ? "Drafts are validated and committed through the authoritative backend." : "The control backend is disconnected. No route, device, or recording action can be applied."}</span></section>
        <section className="canvas-panel" aria-labelledby="canvas-heading"><div className="section-heading"><div><p className="eyebrow">Signal flow</p><h2 id="canvas-heading">{listView ? "Graph list" : "Canvas"}</h2></div><button type="button" className="secondary" aria-pressed={listView} onClick={() => setListView((current) => !current)}>{listView ? "Canvas view" : "List view"}</button></div>{listView ? <NodeList session={draft} selectedNodeId={selectedNode.id} onSelect={setSelectedNodeId} /> : <div className="node-canvas">{draft.nodes.map((node) => <NodeCard key={node.id} node={node} selected={node.id === selectedNode.id} onSelect={() => setSelectedNodeId(node.id)} />)}</div>}</section>
        <section className="panel inspector" aria-labelledby="inspector-heading"><div className="section-heading"><div><p className="eyebrow">Selected node</p><h2 id="inspector-heading">{selectedNode.name}</h2></div><span className="badge">{selectedNode.kind}</span></div><div className="inspector-grid"><label>Enabled<input type="checkbox" checked={selectedNode.enabled} disabled={!backend.connected} onChange={(event) => changeNodeFlag("enabled", event.target.checked)} /></label><label>Bypass<input type="checkbox" checked={selectedNode.bypass} disabled={!backend.connected} onChange={(event) => changeNodeFlag("bypass", event.target.checked)} /></label>{selectedNode.kind === "gain" && <label>Gain (dB)<input type="number" min="-60" max="12" step="0.1" value={typeof selectedNode.parameters.gainDb === "number" ? selectedNode.parameters.gainDb : 0} disabled={!backend.connected} onChange={(event) => changeNodeParameter("gainDb", Number(event.target.value))} /></label>}{selectedNode.kind === "mute" && <label>Muted<input type="checkbox" checked={selectedNode.parameters.muted === true} disabled={!backend.connected} onChange={(event) => changeNodeParameter("muted", event.target.checked)} /></label>}<button type="button" onClick={() => void togglePrivacyMute()} disabled={!backend.connected} aria-pressed={privacyMuted}>{privacyMuted ? "Privacy mute enabled" : "Enable privacy mute"}</button><p className="muted">{backend.connected ? "Changes are local drafts until Plan changes is committed. Privacy mute is an immediate safety latch." : "Controls are disabled while disconnected. Selection is local presentation state only."}</p></div></section>
        <section className="lower-grid"><div className="panel"><div className="section-heading"><h2>Library</h2><button type="button" className="secondary" onClick={() => document.getElementById("recording-search")?.focus()}>Search</button></div><div className="library-grid"><button type="button">Physical input<small>Source</small></button><button type="button">Gain<small>Effect</small></button><button type="button">Mixer<small>Routing</small></button><button type="button">Recorder<small>Output</small></button></div></div><div className="panel"><div className="section-heading"><h2>Recordings</h2><span className="badge">{recordingsError ? "unavailable" : `${visibleRecordings.length}${recordingSearch.trim() ? ` of ${recordings.length}` : ""} file${visibleRecordings.length === 1 ? "" : "s"}`}</span></div><label className="recording-search">Search recordings<input id="recording-search" type="search" value={recordingSearch} onChange={(event) => setRecordingSearch(event.target.value.slice(0, 160))} placeholder="Title, path, or status" /></label>{recordingsError ? <p className="muted">Recording library unavailable: {recordingsError}</p> : recordings.length === 0 ? <p className="muted">No recording has been armed. Completed recordings will appear here with path and status.</p> : visibleRecordings.length === 0 ? <p className="muted">No recording matches this search.</p> : visibleRecordings.map((recording) => <p className="muted" key={recording.id}><label>Title <input aria-label={`Title for ${recording.id}`} value={metadataTitles[recording.id] ?? recording.title ?? ""} onChange={(event) => setMetadataTitles((current) => ({ ...current, [recording.id]: event.target.value }))} /></label> - {recording.state}{recording.missing ? " - missing" : ""}<br /><small>{recording.path}</small> <button type="button" className="secondary" onClick={() => void saveRecordingTitle(recording.id)} disabled={!backend.connected}>Save metadata</button> <button type="button" className="secondary" onClick={() => void previewRecording(recording.id)} disabled={!backend.connected}>Preview</button> <button type="button" className="secondary" onClick={() => void inspectRecovery(recording.id)} disabled={!backend.connected}>Recovery</button> <button type="button" className="secondary" onClick={() => void removeRecordingEntry(recording.id)} disabled={!backend.connected}>Remove entry</button></p>)}{previewMessage && <p className="muted" role="status">{previewMessage}</p>}{recoveryMessage && <p className="muted" role="status">{recoveryMessage}</p>}</div></section>
        <section className="panel route-inspection" aria-labelledby="route-heading"><div className="section-heading"><h2 id="route-heading">Route inspection</h2><button type="button" className="secondary" onClick={() => void inspectRoute()}>Refresh</button></div>{routeInspection === null ? <p className="muted">No route inspection loaded. The backend is authoritative; no path is inferred.</p> : <p className="muted">{routeInspection.reachable ? `${routeInspection.paths.length} reachable path${routeInspection.paths.length === 1 ? "" : "s"} reported.` : "No reachable route reported by the backend."}</p>}{routeInspection?.paths.map((path, index) => <p key={`${path.nodes.join("-")}-${index}`} className="muted">Path {index + 1}: {path.nodes.join(" -> ") || "empty"} ({path.edges.length} edge{path.edges.length === 1 ? "" : "s"})</p>)}</section>
      </main></div>
  </div>;
}
