import type { Node, NodeKind, Session } from "@audiorouter/contracts";

/** Tools that can be left out for a comparison; mirrors `can_be_compared` in the backend, which has the final say. */
const COMPARABLE: Record<NodeKind, boolean> = {
  gain: true,
  volume: true,
  parametricEq: true,
  graphicEq: true,
  bassTreble: true,
  compressor: true,
  gate: true,
  limiter: true,
  delay: true,
  pitch: true,
  dehum: true,
  declick: true,
  denoise: true,
  speechDenoise: true,
  spectralGate: true,
  firFilter: true,
  funVoiceChanger: true,
  duck: true,
  plugin: true,
  physicalInput: false,
  physicalOutput: false,
  applicationCapture: false,
  endpointLoopback: false,
  virtualRenderSource: false,
  virtualCaptureSink: false,
  testSignal: false,
  soundboard: false,
  audioFile: false,
  networkSend: false,
  networkReceive: false,
  mixer: false,
  inputSwitch: false,
  outputSwitch: false,
  mute: false,
  meter: false,
  recorder: false,
  timeShift: false,
};

export const canCompare = (node: Pick<Node, "kind">) => COMPARABLE[node.kind];

/**
 * The comparable tools in line with this one: walks up and down the route
 * through single connections and stops where it splits, joins or ends.
 */
export function chainOf(session: Session, nodeId: string): string[] {
  const chain = new Set<string>();
  const byId = new Map(session.nodes.map((node) => [node.id, node]));
  const enabled = session.edges.filter((edge) => edge.enabled);
  const upstream = (id: string) => enabled.filter((edge) => edge.destinationNode === id).map((edge) => edge.sourceNode);
  const downstream = (id: string) =>
    enabled.filter((edge) => edge.sourceNode === id).map((edge) => edge.destinationNode);
  // A neighbour is in line only when this is its only connection that way too:
  // leaving out a tool that also feeds another branch would change that branch.
  const walk = (start: string, next: (id: string) => string[], back: (id: string) => string[]) => {
    let current = start;
    for (;;) {
      const ways = next(current);
      if (ways.length !== 1 || back(ways[0]).length !== 1) return;
      const node = byId.get(ways[0]);
      if (!node || chain.has(node.id)) return;
      if (node.kind !== "meter" && node.kind !== "recorder" && !canCompare(node)) return;
      if (canCompare(node) && node.enabled) chain.add(node.id);
      current = node.id;
    }
  };
  const self = byId.get(nodeId);
  if (self && canCompare(self)) chain.add(nodeId);
  walk(nodeId, upstream, downstream);
  walk(nodeId, downstream, upstream);
  return [...chain];
}

/** Kinds with settings worth naming; mirrors `takes_presets` in the backend, which has the final say. */
const TAKES_PRESETS: Record<NodeKind, boolean> = {
  gain: true,
  volume: true,
  parametricEq: true,
  graphicEq: true,
  bassTreble: true,
  compressor: true,
  gate: true,
  limiter: true,
  delay: true,
  pitch: true,
  dehum: true,
  declick: true,
  denoise: true,
  speechDenoise: true,
  spectralGate: true,
  funVoiceChanger: true,
  firFilter: false,
  duck: false,
  plugin: false,
  physicalInput: false,
  physicalOutput: false,
  applicationCapture: false,
  endpointLoopback: false,
  virtualRenderSource: false,
  virtualCaptureSink: false,
  testSignal: false,
  soundboard: false,
  audioFile: false,
  networkSend: false,
  networkReceive: false,
  mixer: false,
  inputSwitch: false,
  outputSwitch: false,
  mute: false,
  meter: false,
  recorder: false,
  timeShift: false,
};

export const takesPresets = (node: Pick<Node, "kind">) => TAKES_PRESETS[node.kind];
