import type { EntityId, NodeKind, Session } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";

export type DraftChange = {
  path: `/nodes/${number}/${"enabled" | "bypass"}` | `/nodes/${number}/parameters/${string}`;
  value: boolean | number | string;
};

const libraryNodeDefinitions: Record<Extract<NodeKind, "mixer" | "gain" | "mute" | "meter">, {
  name: string;
  parameters: Record<string, boolean | number | string>;
  ports: Session["nodes"][number]["ports"];
}> = {
  mixer: {
    name: "Mixer",
    parameters: {},
    ports: [
      { name: "in", direction: "input", channels: 2 },
      { name: "out", direction: "output", channels: 2 },
    ],
  },
  gain: {
    name: "Gain",
    parameters: { gainDb: 0 },
    ports: [
      { name: "in", direction: "input", channels: 1 },
      { name: "out", direction: "output", channels: 1 },
    ],
  },
  mute: {
    name: "Mute",
    parameters: { muted: false },
    ports: [
      { name: "in", direction: "input", channels: 1 },
      { name: "out", direction: "output", channels: 1 },
    ],
  },
  meter: {
    name: "Meter",
    parameters: {},
    ports: [{ name: "in", direction: "input", channels: 1 }],
  },
};

/** Adds one supported built-in processor to a draft without mutating its revision or edges. */
export function appendLibraryNode(
  session: Session,
  kind: keyof typeof libraryNodeDefinitions,
): Session {
  const definition = libraryNodeDefinitions[kind];
  let suffix = 1;
  let id = `${kind}-${suffix}`;
  while (session.nodes.some((node) => node.id === id)) {
    suffix += 1;
    id = `${kind}-${suffix}`;
  }
  return {
    ...session,
    nodes: [
      ...session.nodes,
      {
        id,
        kind,
        name: `${definition.name} ${suffix}`,
        enabled: true,
        bypass: false,
        parameters: { ...definition.parameters },
        ports: definition.ports.map((port) => ({ ...port })),
      },
    ],
  };
}

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
  acknowledgments?: string[],
) {
  const plan = await backend.planGraph(candidate);
  if (plan.baseRevision !== candidate.revision) {
    throw new Error("Backend returned a plan for a different session revision");
  }
  if (plan.warnings.length > 0 && acknowledgments === undefined) {
    throw new Error(`Plan requires acknowledgment: ${plan.warnings.join(", ")}`);
  }
  return backend.commitGraph(plan.planId, plan.baseRevision, idempotencyKey, acknowledgments);
}
