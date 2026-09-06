import { useEffect, useState } from "react";
import type { Node } from "@audiorouter/contracts";
import { createDisconnectedBackend, SnapshotCache } from "./backend";
import { demoSession, demoSessions } from "./fixtures";

const backend = createDisconnectedBackend();
const snapshotCache = new SnapshotCache();

function NodeCard({ node, selected, onSelect }: { node: Node; selected: boolean; onSelect: () => void }) {
  return (
    <article className={`node-card${selected ? " selected" : ""}`} tabIndex={0} aria-label={`${node.name}, ${node.kind}`} aria-current={selected ? "true" : undefined} onClick={onSelect} onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); onSelect(); } }}>
      <span className="node-kind">{node.kind}</span>
      <h3>{node.name}</h3>
      <p>{node.ports.length} port{node.ports.length === 1 ? "" : "s"} · {node.enabled ? "enabled" : "disabled"}</p>
      <div className="port-list">
        {node.ports.map((port) => <span key={port.name} className={`port ${port.direction}`}>{port.direction}: {port.name} · {port.channels}ch</span>)}
      </div>
    </article>
  );
}

export function App() {
  const [snapshotState, setSnapshotState] = useState(snapshotCache.current());
  const [selectedSessionId, setSelectedSessionId] = useState(demoSession.id);
  const [selectedNodeId, setSelectedNodeId] = useState(demoSession.nodes[0].id);
  useEffect(() => {
    let mounted = true;
    void snapshotCache.refresh(backend).then((nextState) => {
      if (mounted) setSnapshotState(nextState);
    });
    return () => { mounted = false; };
  }, []);
  const snapshot = snapshotState.snapshot;
  const availableSessions = snapshot ? [snapshot.session, ...demoSessions.filter((item) => item.id !== snapshot.session.id)] : demoSessions;
  const session = availableSessions.find((item) => item.id === selectedSessionId) ?? availableSessions[0];
  const selectedNode = session.nodes.find((node) => node.id === selectedNodeId) ?? session.nodes[0];
  const connectionLabel = backend.connected ? "Backend connected" : "Backend disconnected";
  return (
    <div className="app-shell">
      <header className="topbar">
        <div><p className="eyebrow">AudioRouter</p><h1>Routing workspace</h1></div>
        <div className="status-cluster" aria-live="polite">
          <span className="status-dot disconnected" aria-hidden="true" />
          <span>{connectionLabel}</span>
          <button type="button">Reconnect</button>
        </div>
      </header>

      <div className="workspace-grid">
        <aside className="sidebar" aria-label="Sessions">
          <div className="section-heading"><h2>Sessions</h2><button type="button" aria-label="Create session">+</button></div>
          <label className="session-picker">Preview session<select value={session.id} onChange={(event) => setSelectedSessionId(event.target.value)}>{availableSessions.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</select></label>
          <button type="button" className="session-item selected"><span>{demoSession.name}</span><small>Stopped · rev {demoSession.revision}</small></button>
          <button type="button" className="session-item"><span>Processed microphone</span><small>Stopped · rev 2</small></button>
          <button type="button" className="session-item"><span>Desktop recording</span><small>Missing device</small></button>
          <div className="sidebar-note"><strong>Safe startup</strong><p>Monitoring is muted and recording is unarmed until you explicitly start them.</p></div>
        </aside>

        <main className="main-content">
          <section className="workspace-title"><div><p className="eyebrow">Stopped session</p><h2>{demoSession.name}</h2><p className="muted">Revision {demoSession.revision} · changes are presentation-only in this preview</p></div><div className="actions"><button type="button" className="secondary">Undo</button><button type="button" className="primary">Plan changes</button></div></section>
          {snapshotState.stale && snapshotState.error && <p className="muted" role="status">Last known backend state is stale: {snapshotState.error}</p>}
          <section className="notice" role="status"><strong>Read-only preview</strong><span>The control backend is disconnected. No route, device, or recording action can be applied.</span></section>
          <section className="canvas-panel" aria-labelledby="canvas-heading"><div className="section-heading"><div><p className="eyebrow">Signal flow</p><h2 id="canvas-heading">Canvas</h2></div><button type="button" className="secondary">List view</button></div><div className="node-canvas">{demoSession.nodes.map((node) => <NodeCard key={node.id} node={node} selected={node.id === selectedNode.id} onSelect={() => setSelectedNodeId(node.id)} />)}</div></section>
          <section className="panel inspector" aria-labelledby="inspector-heading"><div className="section-heading"><div><p className="eyebrow">Selected node</p><h2 id="inspector-heading">{selectedNode.name}</h2></div><span className="badge">{selectedNode.kind}</span></div><div className="inspector-grid"><label>Enabled<input type="checkbox" checked={selectedNode.enabled} readOnly disabled /></label><label>Bypass<input type="checkbox" checked={selectedNode.bypass} readOnly disabled /></label><button type="button" disabled title="Connect the backend to apply node changes">Mute</button><p className="muted">Controls are disabled while disconnected. Selection is local presentation state only.</p></div></section>
          <section className="lower-grid"><div className="panel"><div className="section-heading"><h2>Library</h2><button type="button" className="secondary">Search</button></div><div className="library-grid"><button type="button">Physical input<small>Source</small></button><button type="button">Gain<small>Effect</small></button><button type="button">Mixer<small>Routing</small></button><button type="button">Recorder<small>Output</small></button></div></div><div className="panel"><div className="section-heading"><h2>Recordings</h2><span className="badge">0 files</span></div><p className="muted">No recording has been armed. Completed recordings will appear here with path and status.</p></div></section>
        </main>
      </div>
    </div>
  );
}
