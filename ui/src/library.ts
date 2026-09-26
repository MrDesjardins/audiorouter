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
  kind?: Extract<NodeKind, "physicalInput" | "physicalOutput" | "testSignal" | "audioFile" | "mixer" | "gain" | "volume" | "bassTreble" | "dehum" | "declick" | "inputSwitch" | "denoise" | "speechDenoise" | "firFilter" | "timeShift" | "mute" | "meter" | "parametricEq" | "compressor" | "gate" | "limiter" | "delay" | "graphicEq" | "pitch" | "recorder">;
  /** Helper text shown as a tooltip and matched by search, even for an
   * available entry. Used to point at the free existing-endpoint path
   * (VoiceMeeter, VB-Cable) instead of the deferred signed-driver one. */
  note?: string;
  unavailableReason?: string;
  virtualKind?: "virtualRenderSource" | "virtualCaptureSink";
};

const INPUT_DEVICE_NOTE = "Choose a Windows capture endpoint: a microphone, line input, or an installed virtual capture bus such as Voicemeeter Out B1 or CABLE Output. Voicemeeter Input is a playback endpoint; its B1 route needs the Voicemeeter mixer running. For a virtual cable without that app, send Windows sound to CABLE Input and capture CABLE Output here.";
// Windows has no way to send audio directly to a specific running
// application by picking it from a list (unlike capture, which can target
// a process); an application must choose its own input device. Routing
// here into a virtual endpoint, then selecting that same endpoint as the
// microphone/input inside the target application (Discord, Zoom, OBS,
// etc.), is the actual mechanism — this note exists specifically to answer
// "how do I send audio to another app" without a misleading picker.
const OUTPUT_DEVICE_NOTE = "Choose a Windows playback endpoint: speakers, headphones, or an installed virtual playback device such as Voicemeeter Input or CABLE Input. To send audio to another application, select the matching virtual capture device in that application's input settings.";

export const libraryEntries: LibraryEntry[] = [
  { id: "physical-input", label: "Input device", category: "Source", flow: "input", kind: "physicalInput", note: INPUT_DEVICE_NOTE },
  { id: "test-signal", label: "Test Signal", category: "Source", flow: "input", kind: "testSignal" },
  { id: "audio-file", label: "Audio file", category: "Source", flow: "input", kind: "audioFile", note: "Play a WAV or MP3 through the graph. Select media in the node properties." },
  { id: "endpoint-loopback", label: "Endpoint loopback", category: "Source", flow: "input", unavailableReason: "Select an exact active render endpoint in Endpoint binding" },
  { id: "virtual-render-source", label: "Virtual render source", category: "Virtual bus", flow: "input", unavailableReason: "Requires the deferred AudioRouter-managed signed driver. For an installed Voicemeeter or VB-Cable capture endpoint today, use Input device instead.", virtualKind: "virtualRenderSource" },
  { id: "physical-output", label: "Output device", category: "Destination", flow: "output", kind: "physicalOutput", note: OUTPUT_DEVICE_NOTE },
  { id: "virtual-capture-sink", label: "Virtual capture sink", category: "Virtual bus", flow: "output", unavailableReason: "Requires the deferred AudioRouter-managed signed driver. For an installed Voicemeeter or VB-Cable playback endpoint today, use Output device instead.", virtualKind: "virtualCaptureSink" },
  { id: "volume", label: "Volume", category: "Effect", flow: "tool", kind: "volume", note: "Set one source's level in percent (0–200 %) before it reaches a Mixer." },
  { id: "gain", label: "Gain", category: "Effect", flow: "tool", kind: "gain" },
  { id: "bass-treble", label: "Bass & Treble", category: "Effect", flow: "tool", kind: "bassTreble", note: "Two sliders: boost or cut bass (below ~120 Hz) and treble (above ~6 kHz) by up to 12 dB." },
  { id: "dehum", label: "Dehum", category: "Restoration", flow: "tool", kind: "dehum", note: "Remove low-frequency electrical hum at 50 or 60 Hz and its harmonics." },
  { id: "declick", label: "Declick", category: "Restoration", flow: "tool", kind: "declick", note: "Repair clicks, pops, and crackle. A lower threshold repairs more but may alter sharp sounds. Adds about 1.3 ms of delay." },
  { id: "denoise", label: "Denoise", category: "Restoration", flow: "tool", kind: "denoise", note: "Learn a steady noise (fan, hiss, hum) while nothing else plays, then remove it. Adds about 21 ms of delay." },
  { id: "speech-denoise", label: "Speech Denoise", category: "Restoration", flow: "tool", kind: "speechDenoise", note: "Automatically reduce background noise around speech. Runs locally with a spectral method, not a machine-learning model. Adds about 21 ms of delay." },
  { id: "fir-filter", label: "FIR Filter", category: "Effect", flow: "tool", kind: "firFilter", note: "Convolve audio with an impulse response (WAV or MP3, up to 2 s) to emulate a room, speaker, or device. Adds about 11 ms of delay." },
  { id: "time-shift", label: "Time Shift", category: "Effect", flow: "tool", kind: "timeShift", note: "A DVR for live audio: pause, jump back or forward 10 s, and return to live. Keeps up to 120 s." },
  { id: "mixer", label: "Mixer", category: "Routing", flow: "tool", kind: "mixer" },
  { id: "input-switch", label: "Input Switch", category: "Routing", flow: "tool", kind: "inputSwitch", note: "Pass either input A or input B. Switching crossfades over 0.5 s (Shift-click for 2 s)." },
  { id: "recorder", label: "Recorder", category: "Output", flow: "tool", kind: "recorder" },
  { id: "mute", label: "Mute", category: "Effect", flow: "tool", kind: "mute" },
  { id: "meter", label: "Meter", category: "Monitor", flow: "tool", kind: "meter" },
  { id: "parametric-eq", label: "Advanced EQ", category: "Effect", flow: "tool", kind: "parametricEq", note: "Place up to 16 EQ points and shape each with peaking, shelves, pass filters, or a notch." },
  { id: "compressor", label: "Compressor", category: "Effect", flow: "tool", kind: "compressor" },
  { id: "gate", label: "Gate", category: "Effect", flow: "tool", kind: "gate" },
  { id: "limiter", label: "Limiter", category: "Effect", flow: "tool", kind: "limiter" },
  { id: "delay", label: "Sync (delay)", category: "Effect", flow: "tool", kind: "delay", note: "Delay audio by 0–1000 ms to line it up with video or another source. Chain several for longer delays." },
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
