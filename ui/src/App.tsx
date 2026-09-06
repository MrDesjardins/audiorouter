import { useEffect, useRef, useState } from "react";
import type { Node, RouteInspection } from "@audiorouter/contracts";
import { createDisconnectedBackend, SnapshotCache, type UiBackend } from "./backend";
import { applyGraphDraft, setNodeDraftFlag } from "./draft";
import { demoSession, demoSessions } from "./fixtures";

const defaultBackend = createDisconnectedBackend();

function NodeCard({ node, selected, onSelect }: { node: Node; selected: boolean; onSelect: () => void }) {
  return <article className={`node-card${selected ? " selected" : ""}`} tabIndex={0} aria-label={`${node.name}, ${node.kind}`} aria-current={selected ? "true" : undefined} onClick={onSelect} onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); onSelect(); } }}><span className="node-kind">{node.kind}</span><h3>{node.name}</h3><p>{node.ports.length} port{node.ports.length === 1 ? "" : "s"} - {node.enabled ? "enabled" : "disabled"}</p><div className="port-list">{node.ports.map((port) => <span key={port.name} className={`port ${port.direction}`}>{port.direction}: {port.name} - {port.channels}ch</span>)}</div></article>;
}

export function App({ backend = defaultBackend }: { backend?: UiBackend } = {}) {
  const [snapshotCache] = useState(() => new SnapshotCache());
  const [snapshotState, setSnapshotState] = useState(snapshotCache.current());
  const [selectedSessionId, setSelectedSessionId] = useState(demoSession.id);
  const [selectedNodeId, setSelectedNodeId] = useState(demoSession.nodes[0].id);
  const [draft, setDraft] = useState(demoSession);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [routeInspection, setRouteInspection] = useState<RouteInspection | null>(null);
  const eventCursor = useRef({ backendEpoch: 0, sequence: 0 });
  useEffect(() => { let mounted = true; void snapshotCache.refresh(backend).then((nextState) => { if (mounted) setSnapshotState(nextState); }); return () => { mounted = false; }; }, [backend, snapshotCache]);
  const refresh = () => { void snapshotCache.refresh(backend).then(setSnapshotState); };
  const snapshot = snapshotState.snapshot;
  const availableSessions = snapshot ? [snapshot.session, ...demoSessions.filter((item) => item.id !== snapshot.session.id)] : demoSessions;
  const session = availableSessions.find((item) => item.id === selectedSessionId) ?? availableSessions[0];
  useEffect(() => { setDraft(session); setSelectedNodeId(session.nodes[0]?.id ?? ""); setActionMessage(null); setRouteInspection(null); }, [session]);
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
  const connectionLabel = backend.connected ? "Backend connected" : "Backend disconnected";
  const statusSummary = snapshot ? `${snapshot.status.audio} audio - ${snapshot.status.storage} storage - ${snapshot.status.sessionCount} session${snapshot.status.sessionCount === 1 ? "" : "s"}` : "Waiting for backend snapshot";
  const changeNodeFlag = (flag: "enabled" | "bypass", value: boolean) => { setDraft((current) => setNodeDraftFlag(current, selectedNode.id, flag, value)); setActionMessage("Draft updated. Review and plan the changes before committing."); };
  const planChanges = async () => { setActionMessage("Planning changes..."); try { const result = await applyGraphDraft(backend, draft, `ui-${Date.now()}`); setActionMessage(`Committed revision ${result.revision}. Reconnect to refresh the authoritative view.`); } catch (error) { setActionMessage(error instanceof Error ? error.message : "Unable to apply graph changes."); } };
  const inspectRoute = async () => { setActionMessage("Inspecting route..."); try { setRouteInspection(await backend.inspectRoute(selectedNode.id)); setActionMessage("Route inspection refreshed from the backend."); } catch (error) { setRouteInspection(null); setActionMessage(error instanceof Error ? error.message : "Unable to inspect route."); } };
  return <div className="app-shell">
    <header className="topbar"><div><p className="eyebrow">AudioRouter</p><h1>Routing workspace</h1></div><div className="status-cluster" aria-live="polite"><span className={`status-dot${backend.connected ? "" : " disconnected"}`} aria-hidden="true" /><span>{connectionLabel}</span><span className="status-detail">{statusSummary}</span><button type="button" onClick={refresh}>Reconnect</button></div></header>
    <div className="workspace-grid"><aside className="sidebar" aria-label="Sessions"><div className="section-heading"><h2>Sessions</h2><button type="button" aria-label="Create session">+</button></div><label className="session-picker">Preview session<select value={session.id} onChange={(event) => setSelectedSessionId(event.target.value)}>{availableSessions.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</select></label><button type="button" className="session-item selected"><span>{session.name}</span><small>Stopped - rev {session.revision}</small></button><button type="button" className="session-item" onClick={() => setSelectedSessionId("processed-microphone")}><span>Processed microphone</span><small>Stopped - rev 2</small></button><button type="button" className="session-item" onClick={() => setSelectedSessionId("desktop-recording")}><span>Desktop recording</span><small>Missing device</small></button><div className="sidebar-note"><strong>Safe startup</strong><p>Monitoring is muted and recording is unarmed until you explicitly start them.</p></div></aside>
      <main className="main-content"><section className="workspace-title"><div><p className="eyebrow">Stopped session</p><h2>{session.name}</h2><p className="muted">Revision {session.revision} - {backend.connected ? "draft changes require plan and commit" : "changes are presentation-only in this preview"}</p></div><div className="actions"><button type="button" className="secondary" onClick={() => { setDraft(session); setActionMessage("Draft discarded."); }} disabled={!backend.connected}>Undo</button><button type="button" className="primary" onClick={() => void planChanges()} disabled={!backend.connected}>Plan changes</button></div></section>
        {actionMessage && <p className="muted" role="status" aria-live="polite">{actionMessage}</p>}{snapshotState.stale && snapshotState.error && <p className="muted" role="status">Last known backend state is stale: {snapshotState.error}</p>}
        <section className="notice" role="status"><strong>{backend.connected ? "Connected editor" : "Read-only preview"}</strong><span>{backend.connected ? "Drafts are validated and committed through the authoritative backend." : "The control backend is disconnected. No route, device, or recording action can be applied."}</span></section>
        <section className="canvas-panel" aria-labelledby="canvas-heading"><div className="section-heading"><div><p className="eyebrow">Signal flow</p><h2 id="canvas-heading">Canvas</h2></div><button type="button" className="secondary">List view</button></div><div className="node-canvas">{draft.nodes.map((node) => <NodeCard key={node.id} node={node} selected={node.id === selectedNode.id} onSelect={() => setSelectedNodeId(node.id)} />)}</div></section>
        <section className="panel inspector" aria-labelledby="inspector-heading"><div className="section-heading"><div><p className="eyebrow">Selected node</p><h2 id="inspector-heading">{selectedNode.name}</h2></div><span className="badge">{selectedNode.kind}</span></div><div className="inspector-grid"><label>Enabled<input type="checkbox" checked={selectedNode.enabled} disabled={!backend.connected} onChange={(event) => changeNodeFlag("enabled", event.target.checked)} /></label><label>Bypass<input type="checkbox" checked={selectedNode.bypass} disabled={!backend.connected} onChange={(event) => changeNodeFlag("bypass", event.target.checked)} /></label><button type="button" disabled title="Mute control will be enabled with the live safety API">Mute</button><p className="muted">{backend.connected ? "Changes are local drafts until Plan changes is committed." : "Controls are disabled while disconnected. Selection is local presentation state only."}</p></div></section>
        <section className="lower-grid"><div className="panel"><div className="section-heading"><h2>Library</h2><button type="button" className="secondary">Search</button></div><div className="library-grid"><button type="button">Physical input<small>Source</small></button><button type="button">Gain<small>Effect</small></button><button type="button">Mixer<small>Routing</small></button><button type="button">Recorder<small>Output</small></button></div></div><div className="panel"><div className="section-heading"><h2>Recordings</h2><span className="badge">0 files</span></div><p className="muted">No recording has been armed. Completed recordings will appear here with path and status.</p></div></section>
        <section className="panel route-inspection" aria-labelledby="route-heading"><div className="section-heading"><h2 id="route-heading">Route inspection</h2><button type="button" className="secondary" onClick={() => void inspectRoute()}>Refresh</button></div>{routeInspection === null ? <p className="muted">No route inspection loaded. The backend is authoritative; no path is inferred.</p> : <p className="muted">{routeInspection.reachable ? `${routeInspection.paths.length} reachable path${routeInspection.paths.length === 1 ? "" : "s"} reported.` : "No reachable route reported by the backend."}</p>}{routeInspection?.paths.map((path, index) => <p key={`${path.nodes.join("-")}-${index}`} className="muted">Path {index + 1}: {path.nodes.join(" -> ") || "empty"} ({path.edges.length} edge{path.edges.length === 1 ? "" : "s"})</p>)}</section>
      </main></div>
  </div>;
}
