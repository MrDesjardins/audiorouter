// Canvas kind labels and signal-flow families (moved from SessionFlowCanvas).
import type { Node, NodeKind } from "@audiorouter/contracts";
import type { LibraryFlowGroup } from "./library";
import { tx } from "./i18n";
import type { MessageKey } from "./i18n/generated/messageKeys";

const NODE_FLOW_GROUPS: Record<NodeKind, LibraryFlowGroup> = {
  physicalInput: "input",
  testSignal: "input",
  audioFile: "input",
  soundboard: "input",
  instrument: "input",
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
const FLOW_GROUP_LABEL_KEYS = {
  input: "g4Canvas.flowInputs",
  tool: "g4Canvas.flowTools",
  output: "g4Canvas.flowOutputs",
} as const satisfies Record<LibraryFlowGroup, MessageKey>;
export function flowGroupLabel(group: LibraryFlowGroup): string {
  return tx(FLOW_GROUP_LABEL_KEYS[group]);
}

type NodeKindFamily = "input" | "native" | "vst" | "output";
const NODE_KIND_FAMILY_LABEL_KEYS = {
  input: "g4Canvas.familyInput",
  native: "g4Canvas.familyNative",
  vst: "g4Canvas.familyVst",
  output: "g4Canvas.familyOutput",
} as const satisfies Record<NodeKindFamily, MessageKey>;
export function nodeKindFamilyLabel(family: NodeKindFamily): string {
  return tx(NODE_KIND_FAMILY_LABEL_KEYS[family]);
}

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

const NODE_KIND_LABEL_KEYS = {
  physicalInput: "g4Canvas.kindPhysicalInput",
  physicalOutput: "g4Canvas.kindPhysicalOutput",
  applicationCapture: "g4Canvas.kindApplicationCapture",
  endpointLoopback: "g4Canvas.kindEndpointLoopback",
  virtualRenderSource: "g4Canvas.kindVirtualRenderSource",
  virtualCaptureSink: "g4Canvas.kindVirtualCaptureSink",
  testSignal: "g4Canvas.kindTestSignal",
  audioFile: "g4Canvas.kindAudioFile",
  soundboard: "g4Canvas.kindSoundboard",
  instrument: "g4Canvas.kindInstrument",
  parametricEq: "g4Canvas.kindParametricEq",
  graphicEq: "g4Canvas.kindGraphicEq",
  delay: "g4Canvas.kindDelay",
  bassTreble: "g4Canvas.kindBassTreble",
  inputSwitch: "g4Canvas.kindInputSwitch",
  outputSwitch: "g4Canvas.kindOutputSwitch",
  duck: "g4Canvas.kindDuck",
  speechDenoise: "g4Canvas.kindSpeechDenoise",
  spectralGate: "g4Canvas.kindSpectralGate",
  funVoiceChanger: "g4Canvas.kindFunVoiceChanger",
  firFilter: "g4Canvas.kindFirFilter",
  timeShift: "g4Canvas.kindTimeShift",
  networkSend: "g4Canvas.kindNetworkSend",
  networkReceive: "g4Canvas.kindNetworkReceive",
  mixer: "g4Canvas.kindMixer",
  gain: "g4Canvas.kindGain",
  volume: "g4Canvas.kindVolume",
  dehum: "g4Canvas.kindDehum",
  declick: "g4Canvas.kindDeclick",
  denoise: "g4Canvas.kindDenoise",
  mute: "g4Canvas.kindMute",
  meter: "g4Canvas.kindMeter",
  compressor: "g4Canvas.kindCompressor",
  gate: "g4Canvas.kindGate",
  limiter: "g4Canvas.kindLimiter",
  pitch: "g4Canvas.kindPitch",
  recorder: "g4Canvas.kindRecorder",
  plugin: "g4Canvas.kindPlugin",
} as const satisfies Record<NodeKind, MessageKey>;

/** A readable node-kind label for the canvas card. A plugin's label names
 * its exact loaded format (VST2/VST3) instead of the generic "Plugin", since
 * that is the single most useful fact distinguishing it from a native tool. */
export function humanizeNodeKind(node: Node): string {
  if (node.kind === "plugin") {
    const format = node.parameters.format;
    return tx(format === "vst3" ? "g4Canvas.kindVst3" : format === "vst2" ? "g4Canvas.kindVst2" : "g4Canvas.kindVst");
  }
  const key = NODE_KIND_LABEL_KEYS[node.kind];
  if (key) return tx(key);
  return node.kind.replace(/([a-z0-9])([A-Z])/g, "$1 $2").replace(/^./, (char) => char.toUpperCase());
}
