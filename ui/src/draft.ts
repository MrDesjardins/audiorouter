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

/** Adds a topology edge to a local draft; backend validation still gates commit. */
export function appendDraftConnection(
  session: Session,
  sourceNodeId: EntityId,
  sourcePortName: string,
  destinationNodeId: EntityId,
  destinationPortName: string,
): Session {
  if (sourceNodeId === destinationNodeId) throw new Error("A node cannot connect to itself");
  const sourceNode = session.nodes.find((node) => node.id === sourceNodeId);
  const destinationNode = session.nodes.find((node) => node.id === destinationNodeId);
  if (!sourceNode || !destinationNode) throw new Error("Both connection nodes are required");
  const sourcePort = sourceNode.ports.find((port) => port.name === sourcePortName);
  const destinationPort = destinationNode.ports.find((port) => port.name === destinationPortName);
  if (!sourcePort || sourcePort.direction !== "output") throw new Error("Choose an output source port");
  if (!destinationPort || destinationPort.direction !== "input") throw new Error("Choose an input destination port");
  if (session.edges.some((edge) => edge.sourceNode === sourceNodeId && edge.sourcePort === sourcePortName && edge.destinationNode === destinationNodeId && edge.destinationPort === destinationPortName)) {
    throw new Error("That connection is already in the draft");
  }
  if (destinationNode.kind !== "mixer" && session.edges.some((edge) => edge.destinationNode === destinationNodeId && edge.destinationPort === destinationPortName)) {
    throw new Error("That input already has a connection");
  }
  const matrix = Array.from({ length: destinationPort.channels * sourcePort.channels }, () => 0);
  for (let destinationChannel = 0; destinationChannel < destinationPort.channels; destinationChannel += 1) {
    const sourceChannel = Math.min(destinationChannel, sourcePort.channels - 1);
    matrix[destinationChannel * sourcePort.channels + sourceChannel] = 1;
  }
  let suffix = 1;
  let id = `edge-${suffix}`;
  while (session.edges.some((edge) => edge.id === id)) {
    suffix += 1;
    id = `edge-${suffix}`;
  }
  return {
    ...session,
    edges: [...session.edges, {
      id,
      sourceNode: sourceNodeId,
      sourcePort: sourcePortName,
      destinationNode: destinationNodeId,
      destinationPort: destinationPortName,
      matrix,
      enabled: true,
    }],
  };
}

/** Removes one draft edge while leaving the authoritative graph untouched. */
export function removeDraftConnection(session: Session, edgeId: EntityId): Session {
  if (!session.edges.some((edge) => edge.id === edgeId)) throw new Error(`Unknown draft connection: ${edgeId}`);
  return { ...session, edges: session.edges.filter((edge) => edge.id !== edgeId) };
}

/** Removes a draft node and its incident edges without changing the session revision. */
export function removeDraftNode(session: Session, nodeId: EntityId): Session {
  if (!session.nodes.some((node) => node.id === nodeId)) throw new Error(`Unknown draft node: ${nodeId}`);
  return {
    ...session,
    nodes: session.nodes.filter((node) => node.id !== nodeId),
    edges: session.edges.filter((edge) => edge.sourceNode !== nodeId && edge.destinationNode !== nodeId),
  };
}

/** Duplicates one node as an unconnected draft node with a deterministic ID. */
export function duplicateDraftNode(session: Session, nodeId: EntityId): Session {
  const original = session.nodes.find((node) => node.id === nodeId);
  if (!original) throw new Error(`Unknown draft node: ${nodeId}`);
  let suffix = 1;
  let id = `${nodeId}-copy-${suffix}`;
  while (session.nodes.some((node) => node.id === id)) {
    suffix += 1;
    id = `${nodeId}-copy-${suffix}`;
  }
  return {
    ...session,
    nodes: [...session.nodes, {
      ...original,
      id,
      name: `${original.name} copy ${suffix}`,
      parameters: { ...original.parameters },
      ports: original.ports.map((port) => ({ ...port })),
    }],
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

/** Renames a node in the local candidate while preserving its identity and topology. */
export function setNodeDraftName(session: Session, nodeId: EntityId, name: string): Session {
  const trimmed = name.trim();
  if (trimmed.length === 0) throw new Error("Node name cannot be empty");
  if (trimmed.length > 120) throw new Error("Node name cannot exceed 120 characters");
  if (!session.nodes.some((node) => node.id === nodeId)) throw new Error(`Unknown node: ${nodeId}`);
  return { ...session, nodes: session.nodes.map((node) => node.id === nodeId ? { ...node, name: trimmed } : node) };
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
