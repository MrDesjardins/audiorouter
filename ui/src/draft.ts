import type { EntityId, Session } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";

export type DraftChange = {
  path: `/nodes/${number}/${"enabled" | "bypass"}` | `/nodes/${number}/parameters/${string}`;
  value: boolean | number | string;
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

export function setNodeDraftParameter(
  session: Session,
  nodeId: EntityId,
  parameter: string,
  value: boolean | number | string,
): Session {
  const nodeIndex = session.nodes.findIndex((node) => node.id === nodeId);
  if (nodeIndex < 0) throw new Error(`Unknown node: ${nodeId}`);
  return {
    ...session,
    nodes: session.nodes.map((node, index) => index === nodeIndex
      ? { ...node, parameters: { ...node.parameters, [parameter]: value } }
      : node),
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
    const parameterNames = new Set([...Object.keys(original.parameters), ...Object.keys(node.parameters)].sort());
    for (const parameter of parameterNames) {
      if (original.parameters[parameter] !== node.parameters[parameter] && node.parameters[parameter] !== undefined) {
        changes.push({ path: `/nodes/${index}/parameters/${parameter}`, value: node.parameters[parameter] });
      }
    }
    return changes;
  });
}

/** Plans and commits a draft through the authoritative backend in two phases. */
export async function applyGraphDraft(
  backend: Pick<UiBackend, "planGraph" | "commitGraph">,
  candidate: Session,
  idempotencyKey: string,
) {
  const plan = await backend.planGraph(candidate);
  if (plan.baseRevision !== candidate.revision) {
    throw new Error("Backend returned a plan for a different session revision");
  }
  return backend.commitGraph(plan.planId, plan.baseRevision, idempotencyKey);
}
