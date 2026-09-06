import type { EntityId, Session } from "@audiorouter/contracts";

export type DraftChange = {
  path: `/nodes/${number}/${"enabled" | "bypass"}`;
  value: boolean;
};

/**
 * Creates a UI candidate without changing the authoritative session revision.
 * Validation and commit remain backend responsibilities.
 */
export function setNodeDraftFlag(
  session: Session,
  nodeId: EntityId,
  flag: "enabled" | "bypass",
  value: boolean,
): Session {
  const nodeIndex = session.nodes.findIndex((node) => node.id === nodeId);
  if (nodeIndex < 0) throw new Error(`Unknown node: ${nodeId}`);
  return {
    ...session,
    nodes: session.nodes.map((node, index) => index === nodeIndex ? { ...node, [flag]: value } : node),
  };
}

/** Produces deterministic plan inputs for changed node boolean flags. */
export function describeDraftChanges(base: Session, candidate: Session): DraftChange[] {
  return candidate.nodes.flatMap((node, index) => {
    const original = base.nodes.find((item) => item.id === node.id);
    if (!original) return [];
    const changes: DraftChange[] = [];
    if (original.enabled !== node.enabled) changes.push({ path: `/nodes/${index}/enabled`, value: node.enabled });
    if (original.bypass !== node.bypass) changes.push({ path: `/nodes/${index}/bypass`, value: node.bypass });
    return changes;
  });
}
