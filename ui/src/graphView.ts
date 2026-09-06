import type { Session } from "@audiorouter/contracts";

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
