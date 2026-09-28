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
  kind?: Extract<NodeKind, "physicalInput" | "physicalOutput" | "testSignal" | "audioFile" | "mixer" | "gain" | "volume" | "bassTreble" | "dehum" | "declick" | "inputSwitch" | "denoise" | "speechDenoise" | "spectralGate" | "firFilter" | "timeShift" | "mute" | "meter" | "parametricEq" | "compressor" | "gate" | "limiter" | "delay" | "graphicEq" | "pitch" | "recorder" | "networkSend" | "networkReceive">;
  /** Helper text shown as a tooltip and matched by search, even for an
   * available entry. Used to point at the free existing-endpoint path
   * (VoiceMeeter, VB-Cable) instead of the deferred signed-driver one. */
  note?: string;
  unavailableReason?: string;
  virtualKind?: "virtualRenderSource" | "virtualCaptureSink";
};

const INPUT_DEVICE_NOTE = "Sound from a microphone, line input, or virtual cable (for example CABLE Output). Choose the device in Properties.";
// Windows has no way to send audio directly to a specific running
// application by picking it from a list (unlike capture, which can target
// a process); an application must choose its own input device. Routing
// here into a virtual endpoint, then selecting that same endpoint as the
// microphone/input inside the target application (Discord, Zoom, OBS,
// etc.), is the actual mechanism — this note exists specifically to answer
// "how do I send audio to another app" without a misleading picker.
const OUTPUT_DEVICE_NOTE = "Send sound to speakers, headphones, or a virtual cable (for example CABLE Input) that another app uses as its input. Choose the device in Properties.";

export const libraryEntries: LibraryEntry[] = [
  { id: "physical-input", label: "Input device", category: "Source", flow: "input", kind: "physicalInput", note: INPUT_DEVICE_NOTE },
  { id: "test-signal", label: "Test Signal", category: "Source", flow: "input", kind: "testSignal" },
  { id: "audio-file", label: "Audio file", category: "Source", flow: "input", kind: "audioFile", note: "Play a WAV or MP3 through the graph. Select media in the node properties." },
  { id: "endpoint-loopback", label: "Endpoint loopback", category: "Source", flow: "input", unavailableReason: "Select an exact active render endpoint in Endpoint binding" },
  { id: "virtual-render-source", label: "Virtual render source", category: "Virtual bus", flow: "input", unavailableReason: "Not available: AudioRouter does not install its own virtual devices. Use Input device with a virtual cable such as CABLE Output.", virtualKind: "virtualRenderSource" },
  { id: "physical-output", label: "Output device", category: "Destination", flow: "output", kind: "physicalOutput", note: OUTPUT_DEVICE_NOTE },
  { id: "network-receive", label: "Network Receive", category: "Network", flow: "input", kind: "networkReceive", note: "Play audio streamed by AudioRouter on another computer on your network, for example game audio from a gaming PC. Enter that computer's IP address." },
  { id: "network-send", label: "Network Send", category: "Network", flow: "output", kind: "networkSend", note: "Stream audio to AudioRouter on another computer on your network, for example from a gaming PC to a streaming PC. Enter the receiving computer's IP address." },
  { id: "virtual-capture-sink", label: "Virtual capture sink", category: "Virtual bus", flow: "output", unavailableReason: "Not available: AudioRouter does not install its own virtual devices. Use Output device with a virtual cable such as CABLE Input.", virtualKind: "virtualCaptureSink" },
  { id: "volume", label: "Volume", category: "Effect", flow: "tool", kind: "volume", note: "Set one source's level in percent (0–200 %) before it reaches a Mixer." },
  { id: "gain", label: "Gain", category: "Effect", flow: "tool", kind: "gain" },
  { id: "bass-treble", label: "Bass & Treble", category: "Effect", flow: "tool", kind: "bassTreble", note: "Shape voice warmth and brightness by up to 12 dB. Adjustable shelf frequencies let you affect more or less of your voice." },
  { id: "dehum", label: "Dehum", category: "Restoration", flow: "tool", kind: "dehum", note: "Remove low-frequency electrical hum at 50 or 60 Hz and its harmonics." },
  { id: "declick", label: "Declick", category: "Restoration", flow: "tool", kind: "declick", note: "Repair clicks, pops, and crackle. A lower threshold repairs more but may alter sharp sounds. Adds about 1.3 ms of delay." },
  { id: "denoise", label: "Denoise", category: "Restoration", flow: "tool", kind: "denoise", note: "Learn a steady noise (fan, hiss, hum) while nothing else plays, then remove it. Adds about 21 ms of delay." },
  { id: "speech-denoise", label: "Speech Denoise", category: "Restoration", flow: "tool", kind: "speechDenoise", note: "Automatically reduce background noise around speech. Runs locally with a spectral method, not a machine-learning model. Adds about 21 ms of delay." },
  { id: "spectral-gate", label: "FIR Filter Hz", category: "Restoration", flow: "tool", kind: "spectralGate", note: "Learn the noise at every frequency, then block whatever stays below it, like ReaFIR's gate. Shows a live spectrum. Adds about 21 ms of delay." },
  { id: "fir-filter", label: "FIR Filter", category: "Effect", flow: "tool", kind: "firFilter", note: "Give your sound the character of a room, speaker or mic from its impulse response (WAV or MP3, up to 2 s). Wet mix blends it with the original. Adds about 11 ms of delay." },
  { id: "time-shift", label: "Time Shift", category: "Effect", flow: "tool", kind: "timeShift", note: "A DVR for live audio: pause, jump back or forward 10 s, and return to live. Keeps up to 120 s." },
  { id: "mixer", label: "Mixer", category: "Routing", flow: "tool", kind: "mixer" },
  { id: "input-switch", label: "Input Switch", category: "Routing", flow: "tool", kind: "inputSwitch", note: "Pass either input A or input B. Switching crossfades over 0.5 s (Shift-click for 2 s)." },
  { id: "recorder", label: "Recorder", category: "Output", flow: "tool", kind: "recorder" },
  { id: "mute", label: "Mute", category: "Effect", flow: "tool", kind: "mute" },
  { id: "meter", label: "Meter", category: "Monitor", flow: "tool", kind: "meter" },
  { id: "parametric-eq", label: "Advanced EQ", category: "Effect", flow: "tool", kind: "parametricEq", note: "Place up to 16 EQ points with peaking, shelves, low/high/band pass, all pass, or notch filters." },
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

/** Short explanation of each tool, shown in Tools and at the top of Properties. */
export const TOOL_HELP: Record<string, string> = {
  "physical-input": "Sound from a microphone, line input, or virtual cable. Choose the device in Properties.",
  "test-signal": "A steady tone for checking a route without speaking. Press Play on the node to start the tone.",
  "physical-output": "Send the route to speakers, headphones, or a virtual cable. Choose the device in Properties.",
  gain: "Make the sound louder or quieter, in decibels.",
  mixer: "Combine several sources into one, with a volume for each input.",
  recorder: "Record the sound passing through this point to a file on this PC.",
  mute: "Silence this part of the route without disconnecting anything.",
  meter: "Show the level of the sound at this point, to check it is neither too quiet nor clipping.",
  "parametric-eq": "Shape the tone precisely: boost or cut chosen frequencies with up to 16 points.",
  compressor: "Even out loud and quiet moments so a voice sits at a steady level.",
  gate: "Silence quiet background noise between words; opens when you speak.",
  limiter: "Hold peaks under a ceiling so sudden loud sounds never clip or distort.",
  delay: "Hold the sound back by a set time, for example to line audio up with video.",
  "graphic-eq": "Boost or cut ten fixed frequency bands, from deep bass (31.5 Hz) to air (16 kHz).",
  pitch: "Shift the pitch up or down without changing the speed.",
};

/** The explanation of the tool a node was made from. */
export function toolDescription(kind: string): string | undefined {
  const entry = libraryEntries.find((item) => item.kind === kind);
  return entry ? entry.note ?? TOOL_HELP[entry.id] : undefined;
}
