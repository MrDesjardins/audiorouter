import type { Session } from "@audiorouter/contracts";

const INSERT_MIXER_ACTION = "__audiorouter_insert_mixer__";
const REMOVE_MIXER_ACTION = "__audiorouter_remove_mixer__";

export function insertMixerActionId(edgeId: string): string {
  return `${INSERT_MIXER_ACTION}${edgeId}`;
}

export function removeMixerActionId(nodeId: string): string {
  return `${REMOVE_MIXER_ACTION}${nodeId}`;
}

export function decodeTopologyAction(value: string): { kind: "insertMixer" | "removeMixer"; id: string } | null {
  if (value.startsWith(INSERT_MIXER_ACTION)) return { kind: "insertMixer", id: value.slice(INSERT_MIXER_ACTION.length) };
  if (value.startsWith(REMOVE_MIXER_ACTION)) return { kind: "removeMixer", id: value.slice(REMOVE_MIXER_ACTION.length) };
  return null;
}

export function DraftConnectionList({ session, onRemove, onToggle }: { session: Session; onRemove: (id: string) => void; onToggle: (id: string, enabled: boolean) => void }) {
  const names = new Map(session.nodes.map((node) => [node.id, node.name]));
  const mixers = session.nodes.filter((node) => node.kind === "mixer");
  return <section className="draft-connections" aria-labelledby="draft-connections-heading">
    <h3 id="draft-connections-heading">Draft connections</h3>
    {session.edges.length === 0 ? <p className="muted">No draft connections.</p> : <ul aria-label="Draft connections">{session.edges.map((edge) => <li key={edge.id}>
      <span>{names.get(edge.sourceNode) ?? edge.sourceNode}:{edge.sourcePort} → {names.get(edge.destinationNode) ?? edge.destinationNode}:{edge.destinationPort} <small>{edge.enabled ? "enabled" : "disabled"}</small></span>
      <button type="button" className="secondary" onClick={() => onToggle(edge.id, !edge.enabled)}>{edge.enabled ? "Disable" : "Enable"}</button>
      <button type="button" className="secondary" onClick={() => onRemove(edge.id)}>Remove</button>
      <button type="button" className="secondary" aria-label={`Insert mixer on ${names.get(edge.sourceNode) ?? edge.sourceNode} to ${names.get(edge.destinationNode) ?? edge.destinationNode}`} onClick={() => onRemove(insertMixerActionId(edge.id))}>Insert mixer</button>
    </li>)}</ul>}
    {mixers.length > 0 && <div className="mixer-topology-actions" aria-label="Mixer topology actions">{mixers.map((mixer) => <div key={mixer.id}>
      <span>{mixer.name}</span>
      <button type="button" className="secondary" aria-label={`Remove and reconnect ${mixer.name}`} onClick={() => onRemove(removeMixerActionId(mixer.id))}>Remove and reconnect</button>
    </div>)}</div>}
  </section>;
}
