import { useContext } from "react";
import type { Session } from "@audiorouter/contracts";
import { BackendConnectionContext } from "./backendConnectionContext";
import type { InsertableProcessorKind } from "./draft";
import { useMessages } from "./i18n/useMessages";
import { tx } from "./i18n";
import type { MessageKey } from "./i18n/generated/messageKeys";

const INSERT_MIXER_ACTION = "__audiorouter_insert_mixer__";
const REMOVE_MIXER_ACTION = "__audiorouter_remove_mixer__";
export const PROCESSOR_ACTIONS: Array<{ kind: InsertableProcessorKind; label: string }> = [
  { kind: "meter", label: "Meter" },
  { kind: "gain", label: "Gain" },
  { kind: "mute", label: "Mute" },
  { kind: "parametricEq", label: "Advanced EQ" },
  { kind: "graphicEq", label: "Graphic EQ" },
  { kind: "compressor", label: "Compressor" },
  { kind: "gate", label: "Gate" },
  { kind: "limiter", label: "Limiter" },
  { kind: "duck", label: "Duck" },
  { kind: "volume", label: "Volume" },
  { kind: "bassTreble", label: "Bass & Treble" },
  { kind: "dehum", label: "Dehum" },
  { kind: "declick", label: "Declick" },
  { kind: "denoise", label: "Denoise" },
  { kind: "speechDenoise", label: "Speech Denoise" },
  { kind: "spectralGate", label: "Spectral Gate" },
  { kind: "funVoiceChanger", label: "Fun Voice Changer" },
  { kind: "firFilter", label: "FIR Filter" },
  { kind: "timeShift", label: "Time Shift" },
  { kind: "delay", label: "Sync (delay)" },
  { kind: "pitch", label: "Pitch" },
];

/** The tool's name in the current language, as the tool catalog spells it. */
export function processorLabel(kind: InsertableProcessorKind): string {
  if (kind === "pitch") return tx("g4Panel.processorPitch");
  const id = kind.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);
  return tx(`toolCatalog.${id}.name` as MessageKey);
}

export function insertMixerActionId(edgeId: string): string {
  return `${INSERT_MIXER_ACTION}${edgeId}`;
}

export function removeMixerActionId(nodeId: string): string {
  return `${REMOVE_MIXER_ACTION}${nodeId}`;
}

export function decodeTopologyAction(value: string): { kind: "insertMixer" | "removeMixer"; id: string } | null {
  if (value.startsWith(INSERT_MIXER_ACTION))
    return { kind: "insertMixer", id: value.slice(INSERT_MIXER_ACTION.length) };
  if (value.startsWith(REMOVE_MIXER_ACTION))
    return { kind: "removeMixer", id: value.slice(REMOVE_MIXER_ACTION.length) };
  return null;
}

export function DraftConnectionList({
  session,
  onRemove,
  onToggle,
  onInsertProcessor,
}: {
  session: Session;
  onRemove: (id: string) => void;
  onToggle: (id: string, enabled: boolean) => void;
  onInsertProcessor?: (edgeId: string, kind: InsertableProcessorKind) => void;
}) {
  const tr = useMessages();
  const connected = useContext(BackendConnectionContext);
  const names = new Map(session.nodes.map((node) => [node.id, node.name]));
  const mixers = session.nodes.filter((node) => node.kind === "mixer");
  const requestInsertProcessor = (edgeId: string, kind: InsertableProcessorKind) => {
    if (onInsertProcessor) onInsertProcessor(edgeId, kind);
    else globalThis.dispatchEvent(new CustomEvent("audiorouter:insert-processor", { detail: { edgeId, kind } }));
  };
  return (
    <section className="draft-connections" aria-labelledby="draft-connections-heading">
      <h3 id="draft-connections-heading">{tr("g4Panel.draftHeading")}</h3>
      <p className="muted draft-connections-help">
        {tr("g4Panel.draftHelpBefore")} <strong>{tr("g4Panel.draftHelpSave")}</strong> {tr("g4Panel.draftHelpAfter")}
      </p>
      {session.edges.length === 0 ? (
        <p className="muted">{tr("g4Panel.noConnectionsYet")}</p>
      ) : (
        <ul aria-label={tr("g4Panel.routeConnections")}>
          {session.edges.map((edge) => (
            <li key={edge.id}>
              <span>
                {names.get(edge.sourceNode) ?? edge.sourceNode}:{edge.sourcePort} →{" "}
                {names.get(edge.destinationNode) ?? edge.destinationNode}:{edge.destinationPort}{" "}
                <small>{tr(edge.enabled ? "g4Panel.connEnabled" : "g4Panel.connDisabled")}</small>
              </span>
              <button type="button" className="secondary" onClick={() => onToggle(edge.id, !edge.enabled)}>
                {tr(edge.enabled ? "g4Panel.disable" : "g4Panel.enable")}
              </button>
              <button type="button" className="secondary" onClick={() => onRemove(edge.id)}>
                {tr("g4Panel.remove")}
              </button>
              <button
                type="button"
                className="secondary"
                disabled={!connected}
                aria-label={tr("g4Panel.insertMixerAria", {
                  source: names.get(edge.sourceNode) ?? edge.sourceNode,
                  destination: names.get(edge.destinationNode) ?? edge.destinationNode,
                })}
                onClick={() => onRemove(insertMixerActionId(edge.id))}
              >
                {tr("g4Panel.insertMixer")}
              </button>
              {PROCESSOR_ACTIONS.map((processor) => (
                <button
                  type="button"
                  key={processor.kind}
                  className="secondary"
                  disabled={!connected}
                  onClick={() => requestInsertProcessor(edge.id, processor.kind)}
                >
                  {tr("g4Panel.insertProcessor", { name: processorLabel(processor.kind) })}
                </button>
              ))}
            </li>
          ))}
        </ul>
      )}
      {mixers.length > 0 && (
        <div className="mixer-topology-actions" aria-label={tr("g4Panel.mixerTopology")}>
          {mixers.map((mixer) => (
            <div key={mixer.id}>
              <span>{mixer.name}</span>
              <button
                type="button"
                className="secondary"
                disabled={!connected}
                aria-label={tr("g4Panel.removeReconnectAria", { name: mixer.name })}
                onClick={() => onRemove(removeMixerActionId(mixer.id))}
              >
                {tr("g4Panel.removeReconnect")}
              </button>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
