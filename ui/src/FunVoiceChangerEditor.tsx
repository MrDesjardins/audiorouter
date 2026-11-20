import { PropertySection } from "./PropertySection";
import type { DiagnosticsSnapshot, DiscoveryDocument, Node } from "@audiorouter/contracts";
import { ProcessorParameterEditor } from "./NodeEditors";
import type { ProcessorDescriptor } from "./processorCatalog";
import { useMessages } from "./i18n/useMessages";
import { tx } from "./i18n";
import type { MessageKey } from "./i18n/generated/messageKeys";

type VoicePreset = NonNullable<DiscoveryDocument["presets"]["voiceChanger"]>[number];
type VoiceTelemetry = NonNullable<DiagnosticsSnapshot["nodeTelemetry"][number]["voiceChanger"]>;
type Value = boolean | number | string;

/** Inspector sections in signal-chain order; each starts with its on switch. */
export const VOICE_CHANGER_SECTIONS: ReadonlyArray<{ id: string; title: MessageKey; parameters: readonly string[] }> = [
  { id: "Noise gate", title: "g5.funGate", parameters: ["gateDb"] },
  {
    id: "Pitch correction",
    title: "g5.funCorrect",
    parameters: ["correctEnabled", "key", "scale", "retuneMs", "humanizePercent"],
  },
  {
    id: "Pitch and formant",
    title: "g5.funShift",
    parameters: ["shiftEnabled", "pitchSemitones", "pitchCents", "formantSemitones"],
  },
  {
    id: "Character",
    title: "g5.funCharacter",
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
  { id: "Whisper", title: "g5.funWhisper", parameters: ["whisperPercent"] },
  { id: "Tone", title: "g5.funTone", parameters: ["toneEnabled", "lowCutHz", "highCutHz", "peakHz", "peakDb"] },
  {
    id: "Modulation",
    title: "g5.funModulation",
    parameters: ["modEnabled", "modType", "modRateHz", "modDepthPercent", "modFeedbackPercent"],
  },
  {
    id: "Space",
    title: "g5.funSpace",
    parameters: ["spaceEnabled", "spaceSizePercent", "spaceDecayMs", "spaceLevelPercent"],
  },
  { id: "Output", title: "g5.funOutput", parameters: ["mixPercent", "outputDb"] },
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
  if (!pitchOn) return tx("g5.funTunerOff");
  if (!running) return tx("g5.funTunerPlay");
  if (!telemetry?.voiced || telemetry.note === null || telemetry.cents === null) return tx("g5.funTunerNone");
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
  const tr = useMessages();
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
    <section className="voice-changer" aria-label={tr("g5.funEditorAria")}>
      <label className="voice-changer-preset">
        {tr("g5.funPreset")}
        <select
          aria-label={tr("g5.funPresetAria")}
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
        <small className="muted">{preset?.description ?? tr("g5.funLoading")}</small>
        {/* Always rendered so the row never changes height (UI-17). */}
        <span className={`badge voice-changer-edited${edited ? "" : " is-hidden"}`} aria-hidden={!edited}>
          {tr("g5.funEdited")}
        </span>
        <button
          type="button"
          className="secondary"
          disabled={!connected || !edited || !preset}
          onClick={() => apply(presetId)}
        >
          {tr("g5.funReset")}
        </button>
      </div>
      <p className="voice-changer-tuner" role="status" aria-label={tr("g5.funDetected")}>
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
            compact
          />
        </div>
      ) : (
        <p className="muted">{tr("g5.funCustom")}</p>
      )}
      <div className="voice-changer-sections">
        {VOICE_CHANGER_SECTIONS.map((section) => (
          <PropertySection key={section.id} kind={node.kind} title={tr(section.title)} sectionId={section.id}>
            <ProcessorParameterEditor
              node={node}
              processors={processors}
              nodeTypes={nodeTypes}
              connected={connected}
              onChange={onChange}
              only={section.parameters}
              compact
            />
          </PropertySection>
        ))}
      </div>
      <small className="muted">{tr("g5.funLatency")}</small>
    </section>
  );
}
