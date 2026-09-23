import type { NodeKind } from "@audiorouter/contracts";

/** Where an entry sits in the source -> tool -> destination signal-flow
 * mental model, independent of its finer-grained `category` label. Drives
 * the three-group drag shelf and the matching node-card accent color. */
export type LibraryFlowGroup = "input" | "tool" | "output";

export type LibraryEntry = {
  id: string;
  label: string;
  category: string;
  flow: LibraryFlowGroup;
  kind?: Extract<NodeKind, "physicalInput" | "physicalOutput" | "testSignal" | "mixer" | "gain" | "mute" | "meter" | "parametricEq" | "compressor" | "gate" | "limiter" | "delay" | "graphicEq" | "pitch" | "recorder">;
  /** Helper text shown as a tooltip and matched by search, even for an
   * available entry. Used to point at the free existing-endpoint path
   * (VoiceMeeter, VB-Cable) instead of the deferred signed-driver one. */
  note?: string;
  unavailableReason?: string;
  virtualKind?: "virtualRenderSource" | "virtualCaptureSink";
};

const NO_DRIVER_NOTE = "Binds to an already-installed virtual endpoint such as VoiceMeeter or VB-Cable, the same as any physical device — no AudioRouter driver required.";
const NO_DRIVER_INPUT_NOTE = `${NO_DRIVER_NOTE} To capture audio that another application sent to this same virtual endpoint, use this node.`;
// Windows has no way to send audio directly to a specific running
// application by picking it from a list (unlike capture, which can target
// a process); an application must choose its own input device. Routing
// here into a virtual endpoint, then selecting that same endpoint as the
// microphone/input inside the target application (Discord, Zoom, OBS,
// etc.), is the actual mechanism — this note exists specifically to answer
// "how do I send audio to another app" without a misleading picker.
const NO_DRIVER_OUTPUT_NOTE = `${NO_DRIVER_NOTE} To send this audio into another application, select the SAME virtual endpoint here and as that application's own microphone/input device in its own settings — Windows has no way to target an application directly.`;

export const libraryEntries: LibraryEntry[] = [
  { id: "physical-input", label: "Physical input", category: "Source", flow: "input", kind: "physicalInput" },
  { id: "test-signal", label: "Test Signal", category: "Source", flow: "input", kind: "testSignal" },
  { id: "application-capture", label: "Application capture", category: "Source", flow: "input", unavailableReason: "Select a verified running application in Audio sources" },
  { id: "endpoint-loopback", label: "Endpoint loopback", category: "Source", flow: "input", unavailableReason: "Select an exact active render endpoint in Endpoint binding" },
  { id: "existing-virtual-input", label: "Existing virtual input", category: "Virtual endpoint", flow: "input", kind: "physicalInput", note: NO_DRIVER_INPUT_NOTE },
  { id: "virtual-render-source", label: "Virtual render source", category: "Virtual bus", flow: "input", unavailableReason: "Requires the deferred AudioRouter-managed signed driver. For VoiceMeeter or VB-Cable today, use Existing virtual input instead.", virtualKind: "virtualRenderSource" },
  { id: "physical-output", label: "Physical output", category: "Destination", flow: "output", kind: "physicalOutput" },
  { id: "existing-virtual-output", label: "Existing virtual output", category: "Virtual endpoint", flow: "output", kind: "physicalOutput", note: NO_DRIVER_OUTPUT_NOTE },
  { id: "virtual-capture-sink", label: "Virtual capture sink", category: "Virtual bus", flow: "output", unavailableReason: "Requires the deferred AudioRouter-managed signed driver. For VoiceMeeter or VB-Cable today, use Existing virtual output instead.", virtualKind: "virtualCaptureSink" },
  { id: "gain", label: "Gain", category: "Effect", flow: "tool", kind: "gain" },
  { id: "mixer", label: "Mixer", category: "Routing", flow: "tool", kind: "mixer" },
  { id: "recorder", label: "Recorder", category: "Output", flow: "tool", kind: "recorder" },
  { id: "mute", label: "Mute", category: "Effect", flow: "tool", kind: "mute" },
  { id: "meter", label: "Meter", category: "Monitor", flow: "tool", kind: "meter" },
  { id: "parametric-eq", label: "Parametric EQ", category: "Effect", flow: "tool", kind: "parametricEq" },
  { id: "compressor", label: "Compressor", category: "Effect", flow: "tool", kind: "compressor" },
  { id: "gate", label: "Gate", category: "Effect", flow: "tool", kind: "gate" },
  { id: "limiter", label: "Limiter", category: "Effect", flow: "tool", kind: "limiter" },
  { id: "delay", label: "Delay", category: "Effect", flow: "tool", kind: "delay" },
  { id: "graphic-eq", label: "Graphic EQ", category: "Effect", flow: "tool", kind: "graphicEq" },
  { id: "pitch", label: "Pitch shift", category: "Effect", flow: "tool", kind: "pitch" },
];

export function filterLibraryEntries(entries: LibraryEntry[], query: string): LibraryEntry[] {
  const normalized = query.trim().toLocaleLowerCase();
  if (!normalized) return entries;
  return entries.filter((entry) => `${entry.label} ${entry.category} ${entry.note ?? ""} ${entry.unavailableReason ?? ""}`.toLocaleLowerCase().includes(normalized));
}

export function libraryEntryAccessibleLabel(entry: LibraryEntry): string {
  if (entry.unavailableReason) return `${entry.label}, ${entry.category}, unavailable: ${entry.unavailableReason}`;
  if (entry.note) return `${entry.label}, ${entry.category}, ${entry.note}`;
  return `${entry.label}, ${entry.category}`;
}
