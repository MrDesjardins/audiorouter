import type { Node, Session } from "@audiorouter/contracts";
import { tx } from "./i18n";
import type { MessageKey } from "./i18n/generated/messageKeys";

export function nodePortLabels(node: Node): string[] {
  return node.ports.map((port) => `${port.direction}: ${port.name} (${port.channels}ch)`);
}

/** The same port list in the current language, for what the person reads. */
export function nodePortLabelsText(node: Node): string[] {
  return node.ports.map((port) =>
    tx("g4Panel.portLabel", {
      direction: tx(port.direction === "input" ? "g4Panel.portInput" : "g4Panel.portOutput"),
      name: port.name,
      channels: port.channels,
    }),
  );
}

const NODE_STATE_KEYS = {
  muted: "g4Panel.nodeStateMuted",
  bypassed: "g4Panel.nodeStateBypassed",
  enabled: "g4Panel.nodeStateEnabled",
  disabled: "g4Panel.nodeStateDisabled",
} as const satisfies Record<ReturnType<typeof nodeStateLabel>, MessageKey>;

/** The node's state word in the current language. */
export function nodeStateText(node: Node): string {
  return tx(NODE_STATE_KEYS[nodeStateLabel(node)]);
}

export function nodeStateLabel(node: Node): "muted" | "bypassed" | "enabled" | "disabled" {
  return node.parameters.muted === true ? "muted" : node.bypass ? "bypassed" : node.enabled ? "enabled" : "disabled";
}

export function routeNodeLabels(session: Session, nodeIds: string[]): string[] {
  const nodes = new Map(session.nodes.map((node) => [node.id, node]));
  return nodeIds.map((id) => {
    const node = nodes.get(id);
    if (!node) return id;
    return `${node.name} [${nodeStateLabel(node)}]`;
  });
}

/** Returns the enabled upstream/downstream component for presentation highlighting. */
export function relatedNodeIds(session: Session, selectedNodeId: string): Set<string> {
  const neighbors = new Map<string, Set<string>>();
  for (const node of session.nodes) neighbors.set(node.id, new Set());
  for (const edge of session.edges) {
    if (!edge.enabled) continue;
    neighbors.get(edge.sourceNode)?.add(edge.destinationNode);
    neighbors.get(edge.destinationNode)?.add(edge.sourceNode);
  }
  const result = new Set<string>();
  if (!neighbors.has(selectedNodeId)) return result;
  const queue = [selectedNodeId];
  result.add(selectedNodeId);
  while (queue.length > 0) {
    const current = queue.shift();
    if (!current) continue;
    for (const neighbor of neighbors.get(current) ?? []) {
      if (result.has(neighbor)) continue;
      result.add(neighbor);
      queue.push(neighbor);
    }
  }
  return result;
}
