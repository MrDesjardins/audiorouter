// Canvas kind labels and signal-flow families (moved from SessionFlowCanvas).
import type { Node, NodeKind } from "@audiorouter/contracts";
import type { LibraryFlowGroup } from "./library";

const NODE_FLOW_GROUPS: Record<NodeKind, LibraryFlowGroup> = {
  physicalInput: "input",
  testSignal: "input",
  audioFile: "input",
  soundboard: "input",
  applicationCapture: "input",
  endpointLoopback: "input",
  virtualRenderSource: "input",
  networkReceive: "input",
  physicalOutput: "output",
  virtualCaptureSink: "output",
  networkSend: "output",
  mixer: "tool",
  gain: "tool",
  volume: "tool",
  bassTreble: "tool",
  dehum: "tool",
  declick: "tool",
  inputSwitch: "tool",
  outputSwitch: "tool",
  duck: "tool",
  denoise: "tool",
  speechDenoise: "tool",
  spectralGate: "tool",
  funVoiceChanger: "tool",
  firFilter: "tool",
  timeShift: "tool",
  mute: "tool",
  meter: "tool",
  parametricEq: "tool",
  compressor: "tool",
  gate: "tool",
  limiter: "tool",
  delay: "tool",
  graphicEq: "tool",
  pitch: "tool",
  recorder: "tool",
  plugin: "tool",
};

/** Where a node sits in the source -> tool -> destination signal-flow model,
 * used for the node card's accent color and the drag shelf's grouping. */
export function nodeFlowGroup(kind: NodeKind): LibraryFlowGroup {
  return NODE_FLOW_GROUPS[kind] ?? "tool";
}

export const FLOW_GROUPS: LibraryFlowGroup[] = ["input", "tool", "output"];
export const FLOW_GROUP_LABELS: Record<LibraryFlowGroup, string> = {
  input: "Inputs",
  tool: "Tools",
  output: "Outputs",
};

type NodeKindFamily = "input" | "native" | "vst" | "output";
export const NODE_KIND_FAMILY_LABELS: Record<NodeKindFamily, string> = {
  input: "Input",
  native: "Native",
  vst: "VST",
  output: "Output",
};

/** Beyond input/tool/output signal-flow position, a "tool" is either a
 * built-in native processor or an isolated-worker VST plugin; those are
 * visually and operationally very different (a VST loads user-supplied code
 * from disk in its own process), so this is called out as its own family
 * rather than folded into the generic "tool" label. */
export function nodeKindFamily(kind: NodeKind): NodeKindFamily {
  const group = nodeFlowGroup(kind);
  if (group === "input") return "input";
  if (group === "output") return "output";
  return kind === "plugin" ? "vst" : "native";
}

const NODE_KIND_LABELS: Partial<Record<NodeKind, string>> = {
  physicalInput: "Physical Input",
  physicalOutput: "Physical Output",
  applicationCapture: "App Capture",
  endpointLoopback: "Endpoint Loopback",
  virtualRenderSource: "Cable Input",
  virtualCaptureSink: "Cable Output",
  testSignal: "Test Signal",
  audioFile: "Audio File",
  soundboard: "Soundboard",
  parametricEq: "Advanced EQ",
  graphicEq: "Graphic EQ",
  delay: "Sync",
  bassTreble: "Bass & Treble",
  inputSwitch: "Input Switch",
  outputSwitch: "Output Switch",
  duck: "Duck",
  speechDenoise: "Speech Denoise",
  spectralGate: "Spectral Gate",
  funVoiceChanger: "Fun Voice Changer",
  firFilter: "FIR Filter",
  timeShift: "Time Shift",
  networkSend: "Network Send",
  networkReceive: "Network Receive",
};

/** A readable node-kind label for the canvas card. A plugin's label names
 * its exact loaded format (VST2/VST3) instead of the generic "Plugin", since
 * that is the single most useful fact distinguishing it from a native tool. */
export function humanizeNodeKind(node: Node): string {
  if (node.kind === "plugin") {
    const format = node.parameters.format;
    return format === "vst3" ? "VST3 Plugin" : format === "vst2" ? "VST2 Plugin" : "VST Plugin";
  }
  const known = NODE_KIND_LABELS[node.kind];
  if (known) return known;
  return node.kind.replace(/([a-z0-9])([A-Z])/g, "$1 $2").replace(/^./, (char) => char.toUpperCase());
}
