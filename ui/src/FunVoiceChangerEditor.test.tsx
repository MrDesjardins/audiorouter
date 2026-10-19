/** @vitest-environment jsdom */

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { DiscoveryDocument, Node } from "@audiorouter/contracts";
import { FunVoiceChangerEditor, noteName, presetEdited, tunerText } from "./FunVoiceChangerEditor";
import type { ProcessorDescriptor } from "./processorCatalog";

afterEach(cleanup);

type Presets = NonNullable<DiscoveryDocument["presets"]["voiceChanger"]>;

const neutral = { preset: "custom", shiftEnabled: false, pitchSemitones: 0, formantSemitones: 0, mixPercent: 100 };
const presets: Presets = [
  {
    id: "deep",
    version: 1,
    name: "Deep",
    description: "A lower, bigger voice.",
    basicParameters: ["pitchSemitones", "formantSemitones", "mixPercent"],
    parameters: { ...neutral, preset: "deep", shiftEnabled: true, pitchSemitones: -5, formantSemitones: -3 },
  },
  {
    id: "custom",
    version: 1,
    name: "Custom",
    description: "Everything off: build your own voice.",
    basicParameters: [],
    parameters: neutral,
  },
];

const processors = [
  {
    id: "funVoiceChanger",
    version: 1,
    category: "effect",
    availability: { status: "available" },
    latencySamples: 1024,
    parameters: [
      { name: "shiftEnabled", type: "boolean", default: false },
      { name: "pitchSemitones", type: "number", unit: "st", minimum: -12, maximum: 12, default: 0 },
      { name: "formantSemitones", type: "number", unit: "st", minimum: -12, maximum: 12, default: 0 },
      { name: "mixPercent", type: "number", unit: "%", minimum: 0, maximum: 100, default: 100 },
    ],
  },
] as unknown as ProcessorDescriptor[];

const voiceNode = (parameters: Node["parameters"]): Node => ({
  id: "voice",
  kind: "funVoiceChanger",
  typeVersion: 1,
  name: "Fun Voice Changer",
  enabled: true,
  bypass: false,
  parameters,
  ports: [
    { name: "in", direction: "input", channels: 2 },
    { name: "out", direction: "output", channels: 2 },
  ],
});

const renderEditor = (node: Node, extra: Partial<Parameters<typeof FunVoiceChangerEditor>[0]> = {}) => {
  const onChange = vi.fn();
  const onApply = vi.fn();
  render(
    <FunVoiceChangerEditor
      node={node}
      presets={presets}
      telemetry={null}
      running={false}
      processors={processors}
      nodeTypes={null}
      connected
      onChange={onChange}
      onApply={onApply}
      {...extra}
    />,
  );
  return { onChange, onApply };
};

describe("FunVoiceChangerEditor", () => {
  it("applies every value of a chosen preset in one change list", () => {
    const { onApply } = renderEditor(voiceNode({ preset: "custom" }));
    fireEvent.change(screen.getByLabelText("Voice preset"), { target: { value: "deep" } });
    expect(onApply).toHaveBeenCalledTimes(1);
    expect(Object.fromEntries(onApply.mock.calls[0][0])).toEqual(presets[0].parameters);
  });

  it("shows the preset's basic sliders and marks edits, with Reset restoring the recipe", () => {
    const edited = voiceNode({ ...presets[0].parameters, pitchSemitones: -7 });
    const { onApply } = renderEditor(edited);
    expect(screen.getAllByLabelText("Pitch slider").length).toBeGreaterThan(0);
    expect(document.querySelector(".voice-changer-edited")?.classList.contains("is-hidden")).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: "Reset to preset" }));
    expect(Object.fromEntries(onApply.mock.calls[0][0])).toEqual(presets[0].parameters);
  });

  it("keeps the edited badge slot reserved when the node matches its preset", () => {
    renderEditor(voiceNode({ ...presets[0].parameters }));
    const badge = document.querySelector(".voice-changer-edited");
    expect(badge).not.toBeNull();
    expect(badge?.classList.contains("is-hidden")).toBe(true);
    expect(screen.getByRole("button", { name: "Reset to preset" })).toHaveProperty("disabled", true);
  });

  it("treats missing parameters as the neutral defaults", () => {
    expect(presetEdited(voiceNode({ preset: "custom" }), presets[1], presets[1])).toBe(false);
    expect(presetEdited(voiceNode({ preset: "deep", shiftEnabled: true }), presets[0], presets[1])).toBe(true);
  });

  it("explains Custom and opens every section", () => {
    renderEditor(voiceNode({ preset: "custom" }));
    expect(screen.getByText(/Custom starts with every effect off/)).toBeTruthy();
    const sections = Array.from(document.querySelectorAll(".voice-changer-sections details"));
    expect(sections.length).toBe(9);
    expect(sections.every((section) => section.hasAttribute("open"))).toBe(true);
  });
});

describe("tuner strip", () => {
  it("names notes and keeps a placeholder while there is no reading", () => {
    expect(noteName(57)).toBe("A3");
    expect(noteName(61)).toBe("C♯4");
    expect(tunerText(null, true, false)).toBe("— pitch stages off");
    expect(tunerText(null, false, true)).toBe("— press Play to see your note");
    expect(tunerText({ voiced: false, f0Hz: null, note: null, cents: null }, true, true)).toBe("— no voiced sound");
    expect(tunerText({ voiced: true, f0Hz: 221.4, note: 57, cents: 11.2 }, true, true)).toBe("A3 +11 ¢ · 221 Hz");
    expect(tunerText({ voiced: true, f0Hz: 218.1, note: 57, cents: -14.8 }, true, true)).toBe("A3 −15 ¢ · 218 Hz");
  });
});
