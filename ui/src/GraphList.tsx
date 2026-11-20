import { useContext } from "react";
import type { Session } from "@audiorouter/contracts";
import { BackendConnectionContext } from "./backendConnectionContext";
import { insertMixerActionId, PROCESSOR_ACTIONS, processorLabel, removeMixerActionId } from "./DraftConnectionList";
import { humanizeNodeKind } from "./canvasNodeKinds";
import { nodePortLabelsText, nodeStateText } from "./graphView";
import { useMessages } from "./i18n/useMessages";

export function GraphList({
  session,
  selectedNodeId,
  onSelect,
  onRemoveConnection,
  onToggleConnection,
  onInsertProcessor,
  onOpenPluginPicker,
}: {
  session: Session;
  selectedNodeId: string;
  onSelect: (id: string) => void;
  onRemoveConnection: (id: string) => void;
  onToggleConnection: (id: string, enabled: boolean) => void;
  onInsertProcessor?: (edgeId: string, kind: (typeof PROCESSOR_ACTIONS)[number]["kind"]) => void;
  onOpenPluginPicker?: (edgeId: string) => void;
}) {
  const tr = useMessages();
  const connected = useContext(BackendConnectionContext);
  const names = new Map(session.nodes.map((node) => [node.id, node.name]));
  const mixers = session.nodes.filter((node) => node.kind === "mixer");
  const insertProcessor = (edgeId: string, kind: (typeof PROCESSOR_ACTIONS)[number]["kind"]) => {
    if (onInsertProcessor) onInsertProcessor(edgeId, kind);
    else globalThis.dispatchEvent(new CustomEvent("audiorouter:insert-processor", { detail: { edgeId, kind } }));
  };
  return (
    <div className="graph-list" aria-label={tr("g4Panel.toolsAndConnections")}>
      <ol aria-label={tr("g4Panel.toolsList")}>
        {session.nodes.map((node) => (
          <li key={node.id}>
            <button
              type="button"
              className={node.id === selectedNodeId ? "selected" : ""}
              aria-current={node.id === selectedNodeId ? "true" : undefined}
              onClick={() => onSelect(node.id)}
            >
              {node.name}{" "}
              <small>
                {humanizeNodeKind(node)}, {nodeStateText(node)}
              </small>
              <span className="list-port-summary">{nodePortLabelsText(node).join(" · ")}</span>
            </button>
          </li>
        ))}
      </ol>
      <h3>{tr("g4Panel.connections")}</h3>
      {session.edges.length === 0 ? (
        <p className="muted">{tr("g4Panel.noCommitted")}</p>
      ) : (
        <ul aria-label={tr("g4Panel.connections")}>
          {session.edges.map((edge) => (
            <li key={edge.id}>
              <span>
                {names.get(edge.sourceNode) ?? edge.sourceNode}:{edge.sourcePort} →{" "}
                {names.get(edge.destinationNode) ?? edge.destinationNode}:{edge.destinationPort}{" "}
                <small>{tr(edge.enabled ? "g4Panel.connEnabled" : "g4Panel.connDisabled")}</small>
              </span>
              <button
                type="button"
                className="secondary"
                disabled={!connected}
                onClick={() => onToggleConnection(edge.id, !edge.enabled)}
              >
                {tr(edge.enabled ? "g4Panel.disable" : "g4Panel.enable")}
              </button>
              <button
                type="button"
                className="secondary"
                disabled={!connected}
                onClick={() => onRemoveConnection(edge.id)}
              >
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
                onClick={() => onRemoveConnection(insertMixerActionId(edge.id))}
              >
                {tr("g4Panel.insertMixer")}
              </button>
              {PROCESSOR_ACTIONS.map((processor) => (
                <button
                  type="button"
                  key={processor.kind}
                  className="secondary"
                  disabled={!connected}
                  onClick={() => insertProcessor(edge.id, processor.kind)}
                >
                  {tr("g4Panel.insertProcessor", { name: processorLabel(processor.kind) })}
                </button>
              ))}
              {onOpenPluginPicker && (
                <button
                  type="button"
                  className="secondary"
                  disabled={!connected}
                  onClick={() => onOpenPluginPicker(edge.id)}
                >
                  {tr("g4Panel.insertVst")}
                </button>
              )}
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
                onClick={() => onRemoveConnection(removeMixerActionId(mixer.id))}
              >
                {tr("g4Panel.removeReconnect")}
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
