import { useContext } from "react";
import type { Session } from "@audiorouter/contracts";
import { BackendConnectionContext } from "./backendConnectionContext";
import type { InsertableProcessorKind } from "./draft";

const INSERT_MIXER_ACTION = "__audiorouter_insert_mixer__";
const REMOVE_MIXER_ACTION = "__audiorouter_remove_mixer__";
export const PROCESSOR_ACTIONS: Array<{ kind: InsertableProcessorKind; label: string }> = [
  { kind: "gain", label: "Gain" },
  { kind: "mute", label: "Mute" },
  { kind: "parametricEq", label: "Advanced EQ" },
  { kind: "graphicEq", label: "Graphic EQ" },
  { kind: "compressor", label: "Compressor" },
  { kind: "gate", label: "Gate" },
  { kind: "limiter", label: "Limiter" },
  { kind: "delay", label: "Delay" },
  { kind: "pitch", label: "Pitch" },
];

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

export function DraftConnectionList({ session, onRemove, onToggle, onInsertProcessor }: { session: Session; onRemove: (id: string) => void; onToggle: (id: string, enabled: boolean) => void; onInsertProcessor?: (edgeId: string, kind: InsertableProcessorKind) => void }) {
  const connected = useContext(BackendConnectionContext);
  const names = new Map(session.nodes.map((node) => [node.id, node.name]));
  const mixers = session.nodes.filter((node) => node.kind === "mixer");
  const requestInsertProcessor = (edgeId: string, kind: InsertableProcessorKind) => {
    if (onInsertProcessor) onInsertProcessor(edgeId, kind);
    else globalThis.dispatchEvent(new CustomEvent("audiorouter:insert-processor", { detail: { edgeId, kind } }));
  };
  return <section className="draft-connections" aria-labelledby="draft-connections-heading">
    <h3 id="draft-connections-heading">Connections in this draft</h3>
    <p className="muted draft-connections-help">These are the audio links you are editing locally. They are not active until you click <strong>Plan changes</strong> and commit the validated plan. You can keep VoiceMeeter Banana open while exploring; close it only when it owns the exact endpoint AudioRouter needs to claim.</p>
    {session.edges.length === 0 ? <p className="muted">No draft connections.</p> : <ul aria-label="Draft connections">{session.edges.map((edge) => <li key={edge.id}>
      <span>{names.get(edge.sourceNode) ?? edge.sourceNode}:{edge.sourcePort} → {names.get(edge.destinationNode) ?? edge.destinationNode}:{edge.destinationPort} <small>{edge.enabled ? "enabled" : "disabled"}</small></span>
      <button type="button" className="secondary" onClick={() => onToggle(edge.id, !edge.enabled)}>{edge.enabled ? "Disable" : "Enable"}</button>
      <button type="button" className="secondary" onClick={() => onRemove(edge.id)}>Remove</button>
      <button type="button" className="secondary" disabled={!connected} aria-label={`Insert mixer on ${names.get(edge.sourceNode) ?? edge.sourceNode} to ${names.get(edge.destinationNode) ?? edge.destinationNode}`} onClick={() => onRemove(insertMixerActionId(edge.id))}>Insert mixer</button>
      {PROCESSOR_ACTIONS.map((processor) => <button type="button" key={processor.kind} className="secondary" disabled={!connected} onClick={() => requestInsertProcessor(edge.id, processor.kind)}>Insert {processor.label}</button>)}
    </li>)}</ul>}
    {mixers.length > 0 && <div className="mixer-topology-actions" aria-label="Mixer topology actions">{mixers.map((mixer) => <div key={mixer.id}>
      <span>{mixer.name}</span>
      <button type="button" className="secondary" disabled={!connected} aria-label={`Remove and reconnect ${mixer.name}`} onClick={() => onRemove(removeMixerActionId(mixer.id))}>Remove and reconnect</button>
    </div>)}</div>}
  </section>;
}
