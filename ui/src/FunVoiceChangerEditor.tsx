import type { DiagnosticsSnapshot, DiscoveryDocument, Node } from "@audiorouter/contracts";
import { ProcessorParameterEditor } from "./NodeEditors";
import type { ProcessorDescriptor } from "./processorCatalog";

type VoicePreset = NonNullable<DiscoveryDocument["presets"]["voiceChanger"]>[number];
type VoiceTelemetry = NonNullable<DiagnosticsSnapshot["nodeTelemetry"][number]["voiceChanger"]>;
type Value = boolean | number | string;

/** Inspector sections in signal-chain order; each starts with its on switch. */
export const VOICE_CHANGER_SECTIONS: ReadonlyArray<{ title: string; parameters: readonly string[] }> = [
  { title: "Noise gate", parameters: ["gateDb"] },
  { title: "Pitch correction", parameters: ["correctEnabled", "key", "scale", "retuneMs", "humanizePercent"] },
  { title: "Pitch and formant", parameters: ["shiftEnabled", "pitchSemitones", "pitchCents", "formantSemitones"] },
  {
    title: "Character",
    parameters: [
      "ringEnabled",
      "ringHz",
      "ringDepthPercent",
      "crushEnabled",
      "crushBits",
      "crushDownsample",
      "driveEnabled",
      "drivePercent",
    ],
  },
  { title: "Whisper", parameters: ["whisperPercent"] },
  { title: "Tone", parameters: ["toneEnabled", "lowCutHz", "highCutHz", "peakHz", "peakDb"] },
  {
    title: "Modulation",
    parameters: ["modEnabled", "modType", "modRateHz", "modDepthPercent", "modFeedbackPercent"],
  },
  { title: "Space", parameters: ["spaceEnabled", "spaceSizePercent", "spaceDecayMs", "spaceLevelPercent"] },
  { title: "Output", parameters: ["mixPercent", "outputDb"] },
];

const NOTE_NAMES = ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"];

/** "A3", from a MIDI note number (69 = A4). */
export function noteName(midi: number): string {
  return `${NOTE_NAMES[((midi % 12) + 12) % 12]}${Math.floor(midi / 12) - 1}`;
}

/** The value a node effectively uses: its own, or the neutral default. */
function effective(node: Node, neutral: VoicePreset | undefined, name: string): Value | undefined {
  const own = node.parameters[name];
  return own === undefined ? neutral?.parameters[name] : (own as Value);
}

/** Whether the node's settings differ from its chosen preset's values. */
export function presetEdited(node: Node, preset: VoicePreset | undefined, neutral: VoicePreset | undefined): boolean {
  if (!preset) return false;
  return Object.entries(preset.parameters).some(([name, value]) => {
    const current = effective(node, neutral, name);
    return typeof value === "number" && typeof current === "number"
      ? Math.abs(value - current) > 1e-6
      : current !== value;
  });
}

/** Tuner strip text; fixed-size slot, "—" while there is no reading. */
export function tunerText(telemetry: VoiceTelemetry | null, running: boolean, pitchOn: boolean): string {
  if (!pitchOn) return "— pitch stages off";
  if (!running) return "— press Play to see your note";
  if (!telemetry?.voiced || telemetry.note === null || telemetry.cents === null) return "— no voiced sound";
  const cents = Math.round(telemetry.cents);
  return `${noteName(telemetry.note)} ${cents >= 0 ? "+" : "−"}${Math.abs(cents)} ¢ · ${Math.round(telemetry.f0Hz ?? 0)} Hz`;
}

/**
 * Fun Voice Changer (DSP-20): pick a preset, then tweak it. Preset values come
 * from the backend; choosing one writes all of them in one edit, and the
 * node's own saved values are always what is shown.
 */
export function FunVoiceChangerEditor({
  node,
  presets,
  telemetry,
  running,
  processors,
  nodeTypes,
  connected,
  onChange,
  onApply,
}: {
  node: Node;
  presets: VoicePreset[] | null;
  telemetry: VoiceTelemetry | null;
  running: boolean;
  processors: ProcessorDescriptor[] | null;
  nodeTypes: DiscoveryDocument["nodeTypes"] | null;
  connected: boolean;
  onChange: (name: string, value: Value) => void;
  onApply: (changes: Array<[string, Value]>) => void;
}) {
  const presetId = typeof node.parameters.preset === "string" ? node.parameters.preset : "custom";
  const preset = presets?.find((candidate) => candidate.id === presetId);
  const neutral = presets?.find((candidate) => candidate.id === "custom");
  const edited = presetEdited(node, preset, neutral);
  const pitchOn =
    effective(node, neutral, "correctEnabled") === true || effective(node, neutral, "shiftEnabled") === true;
  const apply = (id: string) => {
    const chosen = presets?.find((candidate) => candidate.id === id);
    if (chosen) onApply(Object.entries(chosen.parameters));
  };
  const basic = preset?.basicParameters ?? [];
  return (
    <section className="voice-changer" aria-label="Fun Voice Changer editor">
      <label className="voice-changer-preset">
        Preset
        <select
          aria-label="Voice preset"
          value={presetId}
          disabled={!connected || !presets}
          onChange={(event) => apply(event.target.value)}
        >
          {(presets ?? []).map((candidate) => (
            <option key={candidate.id} value={candidate.id}>
              {candidate.name}
            </option>
          ))}
        </select>
      </label>
      <div className="voice-changer-preset-status">
        <small className="muted">{preset?.description ?? "Loading presets…"}</small>
        {/* Always rendered so the row never changes height (UI-17). */}
        <span className={`badge voice-changer-edited${edited ? "" : " is-hidden"}`} aria-hidden={!edited}>
          edited
        </span>
        <button
          type="button"
          className="secondary"
          disabled={!connected || !edited || !preset}
          onClick={() => apply(presetId)}
        >
          Reset to preset
        </button>
      </div>
      <p className="voice-changer-tuner" role="status" aria-label="Detected note">
        {tunerText(telemetry, running, pitchOn)}
      </p>
      {basic.length > 0 ? (
        <div className="voice-changer-basic">
          <ProcessorParameterEditor
            node={node}
            processors={processors}
            nodeTypes={nodeTypes}
            connected={connected}
            onChange={onChange}
            only={basic}
          />
        </div>
      ) : (
        <p className="muted">Custom starts with every effect off. Turn on the sections below to build a voice.</p>
      )}
      <div className="voice-changer-sections">
        {VOICE_CHANGER_SECTIONS.map((section) => (
          <details key={section.title} open={presetId === "custom"}>
            <summary>{section.title}</summary>
            <ProcessorParameterEditor
              node={node}
              processors={processors}
              nodeTypes={nodeTypes}
              connected={connected}
              onChange={onChange}
              only={section.parameters}
            />
          </details>
        ))}
      </div>
      <small className="muted">
        Output is mono and 21 ms late. Bypass skips that delay, so the timing jumps; compare with Mix at 0 % instead.
      </small>
    </section>
  );
}
