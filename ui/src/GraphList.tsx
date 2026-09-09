import { useContext } from "react";
import type { Session } from "@audiorouter/contracts";
import { BackendConnectionContext } from "./backendConnectionContext";
import { insertMixerActionId, removeMixerActionId } from "./DraftConnectionList";
import { nodePortLabels } from "./graphView";

export function GraphList({ session, selectedNodeId, onSelect, onRemoveConnection, onToggleConnection }: { session: Session; selectedNodeId: string; onSelect: (id: string) => void; onRemoveConnection: (id: string) => void; onToggleConnection: (id: string, enabled: boolean) => void }) {
  const connected = useContext(BackendConnectionContext);
  const names = new Map(session.nodes.map((node) => [node.id, node.name]));
  const mixers = session.nodes.filter((node) => node.kind === "mixer");
  return <div className="graph-list" aria-label="Graph nodes and connections">
    <ol aria-label="Nodes">{session.nodes.map((node) => <li key={node.id}><button type="button" className={node.id === selectedNodeId ? "selected" : ""} aria-current={node.id === selectedNodeId ? "true" : undefined} onClick={() => onSelect(node.id)}>{node.name} <small>{node.kind}, {node.enabled ? "enabled" : "disabled"}</small><span className="list-port-summary">{nodePortLabels(node).join(" · ")}</span></button></li>)}</ol>
    <h3>Connections</h3>
    {session.edges.length === 0 ? <p className="muted">No committed connections.</p> : <ul aria-label="Connections">{session.edges.map((edge) => <li key={edge.id}><span>{names.get(edge.sourceNode) ?? edge.sourceNode}:{edge.sourcePort} → {names.get(edge.destinationNode) ?? edge.destinationNode}:{edge.destinationPort} <small>{edge.enabled ? "enabled" : "disabled"}</small></span><button type="button" className="secondary" disabled={!connected} onClick={() => onToggleConnection(edge.id, !edge.enabled)}>{edge.enabled ? "Disable" : "Enable"}</button><button type="button" className="secondary" disabled={!connected} onClick={() => onRemoveConnection(edge.id)}>Remove</button><button type="button" className="secondary" disabled={!connected} aria-label={`Insert mixer on ${names.get(edge.sourceNode) ?? edge.sourceNode} to ${names.get(edge.destinationNode) ?? edge.destinationNode}`} onClick={() => onRemoveConnection(insertMixerActionId(edge.id))}>Insert mixer</button></li>)}</ul>}
    {mixers.length > 0 && <div className="mixer-topology-actions" aria-label="Mixer topology actions">{mixers.map((mixer) => <div key={mixer.id}><span>{mixer.name}</span><button type="button" className="secondary" disabled={!connected} aria-label={`Remove and reconnect ${mixer.name}`} onClick={() => onRemoveConnection(removeMixerActionId(mixer.id))}>Remove and reconnect</button></div>)}</div>}
  </div>;
}
