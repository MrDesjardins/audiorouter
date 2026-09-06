import { useEffect, useRef, useState } from "react";
import type { Node, RouteInspection } from "@audiorouter/contracts";
import { SessionFlowCanvas } from "./SessionFlowCanvas";
import { createDisconnectedBackend, SnapshotCache, type ApplicationRow, type UiBackend } from "./backend";
import type { DeviceInfo } from "@audiorouter/contracts";
import { appendDraftConnection, appendLibraryNode, applyGraphDraft, duplicateDraftNode, removeDraftConnection, removeDraftNode, setDraftConnectionEnabled, setNodeDraftFlag, setNodeDraftName, setNodeDraftParameter } from "./draft";
import { demoSession, demoSessions } from "./fixtures";
import { recordDraft, redoDraft as redoDraftHistory, undoDraft as undoDraftHistory, type DraftHistory } from "./history";
import { templateSession, type TemplateId } from "./templates";
import { filterLibraryEntries, libraryEntries } from "./library";
import { nodePortLabels } from "./graphView";
import { routeNodeLabels } from "./graphView";
import { readTheme, writeTheme, type ThemeMode } from "./preferences";

const defaultBackend = createDisconnectedBackend();

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
  const [theme, setTheme] = useState<ThemeMode>(() => readTheme(typeof window === "undefined" ? null : window.localStorage));
  const [connectionSource, setConnectionSource] = useState("");
  const [connectionDestination, setConnectionDestination] = useState("");
  const [createdSessions, setCreatedSessions] = useState<import("@audiorouter/contracts").Session[]>([]);
  const eventCursor = useRef({ backendEpoch: 0, sequence: 0 });
  useEffect(() => { let mounted = true; void snapshotCache.refresh(backend).then((nextState) => { if (mounted) { setSnapshotState(nextState); if (nextState.snapshot) eventCursor.current = { backendEpoch: nextState.snapshot.status.eventCursor.backendEpoch, sequence: nextState.snapshot.status.eventCursor.latestSequence }; } }); return () => { mounted = false; }; }, [backend, snapshotCache]);
  const refreshApplications = () => {
    void backend.listApplications().then((items) => { setApplications(items); setApplicationsError(null); }).catch((error) => { setApplications([]); setApplicationsError(error instanceof Error ? error.message : "Application inventory unavailable"); });
  };
  const refreshDevices = () => {
    void backend.listDevices().then((items) => { setDevices(items); setDevicesError(null); }).catch((error) => { setDevices([]); setDevicesError(error instanceof Error ? error.message : "Device inventory unavailable"); });
  };
  const refresh = () => {
    void snapshotCache.refresh(backend).then(setSnapshotState);
    void backend.listRecordings(session.id).then((items) => { setRecordings(items); setRecordingsError(null); }).catch((error) => { setRecordings([]); setRecordingsError(error instanceof Error ? error.message : "Recording library unavailable"); });
    refreshApplications();
    refreshDevices();
  };
  const snapshot = snapshotState.snapshot;
  useEffect(() => { writeTheme(typeof window === "undefined" ? null : window.localStorage, theme); }, [theme]);
  useEffect(() => { if (snapshot) setPrivacyMuted(snapshot.status.privacyMute.muted); }, [snapshot]);
  const availableSessions = [...(snapshot ? [snapshot.session, ...demoSessions.filter((item) => item.id !== snapshot.session.id)] : demoSessions), ...createdSessions.filter((item) => !snapshot || item.id !== snapshot.session.id)];
  const session = availableSessions.find((item) => item.id === selectedSessionId) ?? availableSessions[0];
  const sessionRunning = snapshot?.status.activeSessionIds.includes(session.id) ?? false;
  useEffect(() => { setDraft(session); setDraftHistory({ past: [], future: [] }); setSelectedNodeId(session.nodes[0]?.id ?? ""); setConnectionSource(""); setConnectionDestination(""); setActionMessage(null); setRouteInspection(null); setPendingWarnings([]); setAcknowledgedWarnings(new Set()); setPendingOperation(null); }, [session]);
  useEffect(() => {
    let active = true;
    void backend.listRecordings(session.id).then((items) => { if (active) { setRecordings(items); setRecordingsError(null); } }).catch((error) => { if (active) { setRecordings([]); setRecordingsError(error instanceof Error ? error.message : "Recording library unavailable"); } });
    return () => { active = false; };
  }, [backend, session.id]);
  useEffect(() => {
    let active = true;
    void backend.listApplications().then((items) => { if (active) { setApplications(items); setApplicationsError(null); } }).catch((error) => { if (active) { setApplications([]); setApplicationsError(error instanceof Error ? error.message : "Application inventory unavailable"); } });
    return () => { active = false; };
  }, [backend]);
  useEffect(() => {
    let active = true;
    void backend.listDevices().then((items) => { if (active) { setDevices(items); setDevicesError(null); } }).catch((error) => { if (active) { setDevices([]); setDevicesError(error instanceof Error ? error.message : "Device inventory unavailable"); } });
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
            if (!nextState.stale) eventCursor.current = { backendEpoch: result.backendEpoch, sequence: result.nextSequence };
          }
        } else {
          eventCursor.current = { backendEpoch: result.backendEpoch, sequence: result.nextSequence };
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
  const connectionLabel = backend.connected ? "Backend connected" : "Backend disconnected";
  const statusSummary = snapshot ? `${snapshot.status.audio} audio - ${snapshot.status.storage} storage - ${snapshot.status.sessionCount} session${snapshot.status.sessionCount === 1 ? "" : "s"}` : "Waiting for backend snapshot";
  const recordDraftChange = (next: import("@audiorouter/contracts").Session) => { setDraftHistory((history) => recordDraft(history, draft, next)); setDraft(next); };
  const undoDraft = () => { const transition = undoDraftHistory(draftHistory, draft); if (transition.current === draft) return; setDraftHistory(transition.history); setDraft(transition.current); setActionMessage("Undid the last draft change."); };
  const redoDraft = () => { const transition = redoDraftHistory(draftHistory, draft); if (transition.current === draft) return; setDraftHistory(transition.history); setDraft(transition.current); setActionMessage("Redid the draft change."); };
  const changeNodeFlag = (flag: "enabled" | "bypass", value: boolean) => { recordDraftChange(setNodeDraftFlag(draft, selectedNode.id, flag, value)); setActionMessage("Draft updated. Review and plan the changes before committing."); };
  const changeNodeName = (name: string) => { try { recordDraftChange(setNodeDraftName(draft, selectedNode.id, name)); setActionMessage("Node name draft updated. Review and plan the changes before committing."); } catch (error) { setActionMessage(error instanceof Error ? error.message : "Unable to rename node."); } };
  const changeNodeParameter = (name: string, value: boolean | number) => { if (typeof value === "number" && !Number.isFinite(value)) return; recordDraftChange(setNodeDraftParameter(draft, selectedNode.id, name, value)); setActionMessage("Draft updated. Review and plan the changes before committing."); };
  const changeSessionName = (name: string) => { recordDraftChange({ ...draft, name }); setActionMessage("Session name draft updated. Review and plan the change before committing."); };
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
  const previewRecording = async (recordingId: string) => { setPreviewMessage("Inspecting recording..."); try { const result = await backend.previewRecording(recordingId); setPreviewMessage(`${String(result.preview.status)} recording preview loaded.`); } catch (error) { setPreviewMessage(error instanceof Error ? error.message : "Recording preview unavailable."); } };
  const inspectRecovery = async (recordingId: string) => { setRecoveryMessage("Inspecting recorder recovery..."); try { const result = await backend.getRecordingRecovery(recordingId); setRecoveryMessage(result.status === "missing" ? "No persisted recovery checkpoint is available." : `Recovery checkpoint: ${result.checkpoint.state}.`); } catch (error) { setRecoveryMessage(error instanceof Error ? error.message : "Recording recovery unavailable."); } };
  const saveRecordingTitle = async (recordingId: string) => { try { const title = metadataTitles[recordingId]?.trim() ?? ""; await backend.setRecordingMetadata(recordingId, { title: title || null }); setRecordings((current) => current.map((item) => item.id === recordingId ? { ...item, title: title || null } : item)); setPreviewMessage("Recording metadata saved; the audio file was unchanged."); } catch (error) { setPreviewMessage(error instanceof Error ? error.message : "Unable to save recording metadata."); } };
  const removeRecordingEntry = async (recordingId: string) => { if (!window.confirm("Remove this library entry? The audio file will be preserved.")) return; try { await backend.removeRecordingEntry(recordingId); setRecordings((current) => current.filter((item) => item.id !== recordingId)); setPreviewMessage("Library entry removed; the audio file was preserved."); } catch (error) { setPreviewMessage(error instanceof Error ? error.message : "Unable to remove recording entry."); } };
  const togglePrivacyMute = async () => { const next = !privacyMuted; setActionMessage(next ? "Enabling privacy mute..." : "Disabling privacy mute..."); try { await backend.setPrivacyMute(next); setPrivacyMuted(next); setActionMessage(next ? "Privacy mute enabled." : "Privacy mute disabled."); } catch (error) { setPrivacyMuted(true); setActionMessage(error instanceof Error ? error.message : "Unable to change privacy mute."); } };
  const clearRecoverySafeMode = async () => { setActionMessage("Clearing recovery safe mode..."); try { await backend.clearRecoverySafeMode(); await refresh(); setActionMessage("Recovery safe mode cleared."); } catch (error) { setActionMessage(error instanceof Error ? error.message : "Unable to clear recovery safe mode."); } };
  const createSession = async () => { const name = window.prompt("New session name", "New session")?.trim(); if (!name) return; const id = `session-${Date.now()}`; try { const result = await backend.createSession({ ...demoSession, id, name, revision: 0, nodes: demoSession.nodes.map((node) => ({ ...node, parameters: { ...node.parameters } })), edges: [...demoSession.edges] }); setCreatedSessions((current) => [...current, result.session]); setSelectedSessionId(result.session.id); setActionMessage(`Created stopped session ${result.session.name}.`); } catch (error) { setActionMessage(error instanceof Error ? error.message : "Unable to create session."); } };
  const duplicateSession = async () => { const id = `session-copy-${Date.now()}`; const name = `${session.name} (copy)`; try { const result = await backend.duplicateSession(session.id, id, name); setCreatedSessions((current) => [...current, result.session]); setSelectedSessionId(result.session.id); setActionMessage(`Duplicated stopped session ${result.session.name}.`); } catch (error) { setActionMessage(error instanceof Error ? error.message : "Unable to duplicate session."); } };
  const deleteSession = async () => { if (!window.confirm(`Delete stopped session “${session.name}”?`)) return; try { await backend.deleteSession(session.id); setCreatedSessions((current) => current.filter((item) => item.id !== session.id)); const fallback = availableSessions.find((item) => item.id !== session.id); if (fallback) setSelectedSessionId(fallback.id); setActionMessage(`Deleted session ${session.name}.`); } catch (error) { setActionMessage(error instanceof Error ? error.message : "Unable to delete session."); } };
  const startSession = async () => { setActionMessage("Starting session..."); try { const result = await backend.startSession(session.id); await refresh(); setActionMessage(`Session is running (generation ${result.generation}).`); } catch (error) { setActionMessage(error instanceof Error ? error.message : "Unable to start session."); } };
  const stopSession = async () => { setActionMessage("Stopping session..."); try { await backend.stopSession(session.id); await refresh(); setActionMessage("Session stopped."); } catch (error) { setActionMessage(error instanceof Error ? error.message : "Unable to stop session."); } };
  const addLibraryNode = (kind: "mixer" | "gain" | "mute" | "meter") => { const next = appendLibraryNode(draft, kind); recordDraftChange(next); setSelectedNodeId(next.nodes[next.nodes.length - 1].id); setActionMessage(`${next.nodes[next.nodes.length - 1].name} added to the draft. Review and plan the changes before committing.`); };
  const addConnection = () => { const source = decodePort(connectionSource); const destination = decodePort(connectionDestination); if (!source || !destination) { setActionMessage("Choose an output and input port first."); return; } try { const next = appendDraftConnection(draft, source.nodeId, source.portName, destination.nodeId, destination.portName); recordDraftChange(next); setActionMessage("Connection added to the draft. Review and plan the changes before committing."); } catch (error) { setActionMessage(error instanceof Error ? error.message : "Unable to add connection."); } };
  const removeConnection = (edgeId: string) => { try { recordDraftChange(removeDraftConnection(draft, edgeId)); setActionMessage("Connection removed from the draft. Review and plan the changes before committing."); } catch (error) { setActionMessage(error instanceof Error ? error.message : "Unable to remove connection."); } };
  const toggleConnection = (edgeId: string, enabled: boolean) => { try { recordDraftChange(setDraftConnectionEnabled(draft, edgeId, enabled)); setActionMessage(`Connection ${enabled ? "enabled" : "disabled"} in the draft. Review and plan the changes before committing.`); } catch (error) { setActionMessage(error instanceof Error ? error.message : "Unable to change connection state."); } };
  const removeSelectedNode = () => { if (!window.confirm(`Remove node “${selectedNode.name}” and its draft connections?`)) return; try { const next = removeDraftNode(draft, selectedNode.id); recordDraftChange(next); setSelectedNodeId(next.nodes[0]?.id ?? ""); setActionMessage("Node removed from the draft. Review and plan the changes before committing."); } catch (error) { setActionMessage(error instanceof Error ? error.message : "Unable to remove node."); } };
  const duplicateSelectedNode = () => { try { const next = duplicateDraftNode(draft, selectedNode.id); const copy = next.nodes[next.nodes.length - 1]; recordDraftChange(next); setSelectedNodeId(copy.id); setActionMessage(`${copy.name} added to the draft without connections. Review and plan the changes before committing.`); } catch (error) { setActionMessage(error instanceof Error ? error.message : "Unable to duplicate node."); } };
  const applyTemplate = () => { const next = templateSession(selectedTemplate); recordDraftChange({ ...next, id: draft.id, revision: draft.revision }); setSelectedNodeId(next.nodes[0]?.id ?? ""); setActionMessage("Template loaded into the draft. Review device bindings and plan the changes before committing."); };
  const lifecycleActions = <section className="panel lifecycle-panel" aria-labelledby="lifecycle-heading"><h2 id="lifecycle-heading">Session lifecycle</h2><p className="muted">Starting a session uses the shared authorized backend lifecycle API.</p><button type="button" className="secondary" onClick={() => void (sessionRunning ? stopSession() : startSession())} disabled={!backend.connected}>{sessionRunning ? "Stop session" : "Start session"}</button></section>;
  return <div className={`app-shell theme-${theme}`}>{lifecycleActions}
    <header className="topbar"><div><p className="eyebrow">AudioRouter</p><h1>Routing workspace</h1></div><div className="status-cluster" aria-live="polite"><span className={`status-dot${backend.connected ? "" : " disconnected"}`} aria-hidden="true" /><span>{connectionLabel}</span><span className="status-detail">{statusSummary}</span><label className="theme-picker">Theme<select aria-label="Color theme" value={theme} onChange={(event) => setTheme(event.target.value as ThemeMode)}><option value="dark">Dark</option><option value="light">Light</option><option value="high-contrast">High contrast</option></select></label><button type="button" onClick={refresh}>Reconnect</button></div></header>
      <div className="workspace-grid"><aside className="sidebar" aria-label="Sessions"><div className="section-heading"><h2>Sessions</h2><button type="button" aria-label="Create session" onClick={() => void createSession()} disabled={!backend.connected} title="Session creation requires the connected backend">+</button></div><label className="session-picker">Preview session<select value={session.id} onChange={(event) => setSelectedSessionId(event.target.value)}>{availableSessions.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</select></label>{availableSessions.map((item) => { const running = snapshot?.status.activeSessionIds.includes(item.id) ?? false; return <button type="button" key={item.id} className={`session-item${item.id === session.id ? " selected" : ""}`} aria-current={item.id === session.id ? "true" : undefined} onClick={() => setSelectedSessionId(item.id)}><span>{item.name}</span><small>{running ? "Running" : "Stopped"} - rev {item.revision}</small></button>; })}<div className="sidebar-note"><strong>Safe startup</strong><p>Monitoring is muted and recording is unarmed until you explicitly start them.</p></div></aside>
      <main className="main-content"><section className="workspace-title"><div><p className="eyebrow">{sessionRunning ? "Running session" : "Stopped session"}</p><label className="session-name">Session name<input value={draft.name} maxLength={120} disabled={!backend.connected} onChange={(event) => changeSessionName(event.target.value)} /></label><p className="muted">Revision {session.revision} - {backend.connected ? "draft changes require plan and commit" : "changes are presentation-only in this preview"}</p></div><div className="actions"><button type="button" className="secondary" onClick={() => void duplicateSession()} disabled={!backend.connected}>Duplicate</button><button type="button" className="secondary" onClick={() => void deleteSession()} disabled={!backend.connected}>Delete</button><button type="button" className="secondary" onClick={undoDraft} disabled={!backend.connected || draftHistory.past.length === 0}>Undo draft</button><button type="button" className="secondary" onClick={redoDraft} disabled={!backend.connected || draftHistory.future.length === 0}>Redo draft</button><button type="button" className="secondary" onClick={() => { setDraft(session); setDraftHistory({ past: [], future: [] }); setActionMessage("Draft discarded."); }} disabled={!backend.connected}>Discard draft</button><button type="button" className="primary" onClick={() => void planChanges()} disabled={!backend.connected}>Plan changes</button></div></section>
        {actionMessage && <p className="muted" role="status" aria-live="polite">{actionMessage}</p>}{pendingWarnings.length > 0 && <section className="warning-panel" aria-labelledby="warning-heading"><h2 id="warning-heading">Plan warnings</h2>{pendingWarnings.map((warning) => <label key={warning}><input type="checkbox" checked={acknowledgedWarnings.has(warning)} onChange={(event) => setAcknowledgedWarnings((current) => { const next = new Set(current); if (event.target.checked) next.add(warning); else next.delete(warning); return next; })} /> I acknowledge: {warning}</label>)}<button type="button" className="primary" disabled={acknowledgedWarnings.size !== pendingWarnings.length} onClick={() => void commitAcknowledgedPlan()}>Commit acknowledged plan</button></section>}{snapshotState.stale && snapshotState.error && <p className="muted" role="status">Last known backend state is stale: {snapshotState.error}</p>}
        <section className="notice" role="status"><strong>{backend.connected ? "Connected editor" : "Read-only preview"}</strong><span>{backend.connected ? "Drafts are validated and committed through the authoritative backend." : "The control backend is disconnected. No route, device, or recording action can be applied."}</span></section>
        <section className="panel recovery-panel" aria-labelledby="recovery-heading"><div className="section-heading"><div><p className="eyebrow">Crash recovery</p><h2 id="recovery-heading">{snapshot?.status.recovery.safeMode ? "Safe mode is active" : "Normal startup mode"}</h2></div><span className="badge">{snapshot?.status.recovery.recentCrashes ?? 0} recent crash{(snapshot?.status.recovery.recentCrashes ?? 0) === 1 ? "" : "es"}</span></div><p className="muted">{snapshot?.status.recovery.persistence === "durable" ? "Recovery state is persisted by the backend." : "Recovery state is held in memory for this preview."}</p><button type="button" className="secondary" onClick={() => void clearRecoverySafeMode()} disabled={!backend.connected || !snapshot?.status.recovery.safeMode}>Clear safe mode</button></section>
        <section className="panel" aria-labelledby="applications-heading"><div className="section-heading"><div><p className="eyebrow">Audio sources</p><h2 id="applications-heading">Applications</h2></div><span className="badge">{applicationsError ? "unavailable" : applications.length}</span></div>{applicationsError ? <p className="muted">Application inventory unavailable: {applicationsError}</p> : applications.length === 0 ? <p className="muted">No process audio sessions are exposed by the backend snapshot.</p> : <ul aria-label="Running audio applications">{applications.map((application) => <li key={`${application.processId}-${application.creationTime100ns ?? "unknown"}`}><strong>{application.audioDisplayNames[0] ?? application.executable}</strong> <small>{application.executable} · PID {application.processId} · {application.audioActivity} · {application.captureCapability === "observed" ? "capture observed" : "capture not observed"}</small></li>)}</ul>}</section>
        <section className="panel" aria-labelledby="devices-heading"><div className="section-heading"><div><p className="eyebrow">Windows endpoints</p><h2 id="devices-heading">Devices</h2></div><span className="badge">{devicesError ? "unavailable" : devices.length}</span></div>{devicesError ? <p className="muted">Device inventory unavailable: {devicesError}</p> : devices.length === 0 ? <p className="muted">No active endpoint metadata is exposed by the backend.</p> : <ul aria-label="Active audio devices">{devices.map((device) => <li key={device.id}><strong>{device.direction === "capture" ? "Capture" : "Render"}</strong> <small>{device.id} · {device.format.sampleRateHz} Hz · {device.format.channels} ch · {device.periods.default100ns / 10000} ms period</small></li>)}</ul>}</section>
        <section className="canvas-panel" aria-labelledby="canvas-heading"><div className="section-heading"><div><p className="eyebrow">Signal flow</p><h2 id="canvas-heading">{listView ? "Graph list" : "Canvas"}</h2></div><button type="button" className="secondary" aria-pressed={listView} onClick={() => setListView((current) => !current)}>{listView ? "Canvas view" : "List view"}</button></div>{listView ? <NodeList session={draft} selectedNodeId={selectedNode.id} onSelect={setSelectedNodeId} onRemoveConnection={removeConnection} onToggleConnection={toggleConnection} /> : <><SessionFlowCanvas session={draft} selectedNodeId={selectedNode.id} onSelect={setSelectedNodeId} /><DraftConnectionList session={draft} onRemove={removeConnection} onToggle={toggleConnection} /></>}<fieldset className="connection-editor" disabled={!backend.connected}><legend>Add connection to draft</legend><label>Output<select aria-label="Source output port" value={connectionSource} onChange={(event) => setConnectionSource(event.target.value)}><option value="">Choose source</option>{outputPorts.map((port) => <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>{port.nodeName} · {port.portName} · {port.channels}ch</option>)}</select></label><span aria-hidden="true">→</span><label>Input<select aria-label="Destination input port" value={connectionDestination} onChange={(event) => setConnectionDestination(event.target.value)}><option value="">Choose destination</option>{inputPorts.map((port) => <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>{port.nodeName} · {port.portName} · {port.channels}ch</option>)}</select></label><button type="button" className="secondary" onClick={addConnection}>Add connection</button></fieldset></section>
        <section className="panel inspector" aria-labelledby="inspector-heading"><div className="section-heading"><div><p className="eyebrow">Selected node</p><h2 id="inspector-heading">{selectedNode.name}</h2></div><span className="badge">{selectedNode.kind}</span></div><div className="inspector-grid"><label>Node name<input type="text" maxLength={120} value={selectedNode.name} disabled={!backend.connected} onChange={(event) => changeNodeName(event.target.value)} /></label><label>Enabled<input type="checkbox" checked={selectedNode.enabled} disabled={!backend.connected} onChange={(event) => changeNodeFlag("enabled", event.target.checked)} /></label><label>Bypass<input type="checkbox" checked={selectedNode.bypass} disabled={!backend.connected} onChange={(event) => changeNodeFlag("bypass", event.target.checked)} /></label>{selectedNode.kind === "gain" && <label>Gain (dB)<input type="number" min="-60" max="12" step="0.1" value={typeof selectedNode.parameters.gainDb === "number" ? selectedNode.parameters.gainDb : 0} disabled={!backend.connected} onChange={(event) => changeNodeParameter("gainDb", Number(event.target.value))} /></label>}{selectedNode.kind === "mute" && <label>Muted<input type="checkbox" checked={selectedNode.parameters.muted === true} disabled={!backend.connected} onChange={(event) => changeNodeParameter("muted", event.target.checked)} /></label>}<button type="button" className="secondary" onClick={duplicateSelectedNode} disabled={!backend.connected}>Duplicate node to draft</button><button type="button" className="secondary" onClick={removeSelectedNode} disabled={!backend.connected}>Remove node from draft</button><button type="button" onClick={() => void togglePrivacyMute()} disabled={!backend.connected} aria-pressed={privacyMuted}>{privacyMuted ? "Privacy mute enabled" : "Enable privacy mute"}</button><p className="muted">{backend.connected ? "Changes are local drafts until Plan changes is committed. Privacy mute is an immediate safety latch." : "Controls are disabled while disconnected. Selection is local presentation state only."}</p></div></section>
        <section className="lower-grid"><div className="panel"><div className="section-heading"><h2>Library</h2><button type="button" className="secondary" onClick={() => document.getElementById("library-search")?.focus()}>Search</button></div><label className="library-search">Search nodes<input id="library-search" type="search" value={librarySearch} onChange={(event) => setLibrarySearch(event.target.value.slice(0, 80))} placeholder="Gain, effect, meter..." /></label><div className="library-grid">{visibleLibraryEntries.length === 0 ? <p className="muted">No library entries match this search.</p> : visibleLibraryEntries.map((entry) => entry.kind ? <button type="button" key={entry.id} onClick={() => addLibraryNode(entry.kind!)} disabled={!backend.connected}>{entry.label}<small>{entry.category} · add to draft</small></button> : <button type="button" key={entry.id} disabled title={entry.unavailableReason}>{entry.label}<small>{entry.category} · unavailable</small></button>)}</div><p className="muted">Built-in processors are added as local drafts; use Plan changes to validate and commit them.</p><div className="template-picker"><label>Guided template<select aria-label="Guided setup template" value={selectedTemplate} onChange={(event) => setSelectedTemplate(event.target.value as TemplateId)}><option value="gaming-discord">Gaming + Discord</option><option value="processed-microphone">Processed microphone</option><option value="mix-minus">Mix-minus conversation</option></select></label><button type="button" className="secondary" onClick={applyTemplate} disabled={!backend.connected}>Load template to draft</button><small>Templates remain stopped and require device review before commit.</small></div></div><div className="panel"><div className="section-heading"><h2>Recordings</h2><span className="badge">{recordingsError ? "unavailable" : `${visibleRecordings.length}${recordingSearch.trim() ? ` of ${recordings.length}` : ""} file${visibleRecordings.length === 1 ? "" : "s"}`}</span></div><label className="recording-search">Search recordings<input id="recording-search" type="search" value={recordingSearch} onChange={(event) => setRecordingSearch(event.target.value.slice(0, 160))} placeholder="Title, path, or status" /></label>{recordingsError ? <p className="muted">Recording library unavailable: {recordingsError}</p> : recordings.length === 0 ? <p className="muted">No recording has been armed. Completed recordings will appear here with path and status.</p> : visibleRecordings.length === 0 ? <p className="muted">No recording matches this search.</p> : visibleRecordings.map((recording) => <p className="muted" key={recording.id}><label>Title <input aria-label={`Title for ${recording.id}`} value={metadataTitles[recording.id] ?? recording.title ?? ""} onChange={(event) => setMetadataTitles((current) => ({ ...current, [recording.id]: event.target.value }))} /></label> - {recording.state}{recording.missing ? " - missing" : ""}<br /><small>{recording.path}</small> <button type="button" className="secondary" onClick={() => void saveRecordingTitle(recording.id)} disabled={!backend.connected}>Save metadata</button> <button type="button" className="secondary" onClick={() => void previewRecording(recording.id)} disabled={!backend.connected}>Preview</button> <button type="button" className="secondary" onClick={() => void inspectRecovery(recording.id)} disabled={!backend.connected}>Recovery</button> <button type="button" className="secondary" onClick={() => void removeRecordingEntry(recording.id)} disabled={!backend.connected}>Remove entry</button></p>)}{previewMessage && <p className="muted" role="status">{previewMessage}</p>}{recoveryMessage && <p className="muted" role="status">{recoveryMessage}</p>}</div></section>
        <section className="panel route-inspection" aria-labelledby="route-heading"><div className="section-heading"><div><p className="eyebrow">Backend explanation</p><h2 id="route-heading">Receives audio from</h2></div><button type="button" className="secondary" onClick={() => void inspectRoute()}>Refresh</button></div>{routeInspection === null ? <p className="muted">No route inspection loaded. The backend is authoritative; no path is inferred.</p> : <p className="muted">{routeInspection.reachable ? `${routeInspection.paths.length} reachable path${routeInspection.paths.length === 1 ? "" : "s"} reported to ${draftNodeNames.get(routeInspection.destinationNode) ?? routeInspection.destinationNode}.` : "No reachable route reported by the backend."}</p>}{routeInspection?.paths.map((path, index) => <p key={`${path.nodes.join("-")}-${index}`} className="muted">Path {index + 1}: {routeNodeLabels(draft, path.nodes).join(" → ") || "empty"} ({path.edges.length} edge{path.edges.length === 1 ? "" : "s"})<br /><small>Channel map: {path.channelMaps.length === 0 ? "none" : path.channelMaps.map((row) => `[${row.join(", ")}]`).join(" ")}</small></p>)}</section>
      </main></div>
  </div>;
}
