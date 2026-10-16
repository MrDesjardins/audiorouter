import type { Session } from "@audiorouter/contracts";
import { sameJsonValue } from "./sessionInventory";

/** Presentation only: mark saved/draft differences, including changed wiring. */
export function unsavedNodeIds(saved: Session, draft: Session): Set<string> {
  const result = new Set<string>();
  const savedNodes = new Map(saved.nodes.map((node) => [node.id, node]));
  for (const node of draft.nodes) {
    if (!sameJsonValue(savedNodes.get(node.id), node)) result.add(node.id);
  }
  const savedEdges = new Map(saved.edges.map((edge) => [edge.id, edge]));
  const draftEdges = new Map(draft.edges.map((edge) => [edge.id, edge]));
  for (const id of new Set([...savedEdges.keys(), ...draftEdges.keys()])) {
    const before = savedEdges.get(id);
    const after = draftEdges.get(id);
    if (sameJsonValue(before, after)) continue;
    for (const edge of [before, after]) {
      if (!edge) continue;
      result.add(edge.sourceNode);
      result.add(edge.destinationNode);
    }
  }
  return result;
}
