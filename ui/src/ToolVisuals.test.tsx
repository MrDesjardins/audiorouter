/** @vitest-environment jsdom */

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { Node } from "@audiorouter/contracts";
import { createDisconnectedBackend } from "./backend";
import { BassTrebleEditor, DehumEditor, DelayEditor, GraphicEqEditor, InputSwitchEditor, LevelEditor, PitchEditor, StrengthEditor, dehumMagnitudeDb } from "./ToolVisuals";
import { formatParameterValue } from "./parameterText";

afterEach(cleanup);

const node = (kind: Node["kind"], parameters: Node["parameters"] = {}): Node => ({
  id: `${kind}-1`, kind, typeVersion: 1, name: `${kind} 1`, enabled: true, bypass: false, parameters,
  ports: [{ name: "in", direction: "input", channels: 2 }, { name: "out", direction: "output", channels: 2 }],
});

describe("tool visual editors", () => {
  it("Graphic EQ: faders, exact band entry and presets change only differing bands", async () => {
    const onChange = vi.fn();
    const processorResponse = vi.fn(async () => ({ frequenciesHz: [20, 20000], magnitudeDb: [0, 0] }));
    render(<GraphicEqEditor node={node("graphicEq", { band5Db: 3 })} backend={{ ...createDisconnectedBackend(), connected: true, processorResponse } as never} disabled={false} onChange={onChange} />);
    expect(screen.getAllByRole("slider")).toHaveLength(10);
    fireEvent.change(screen.getByRole("slider", { name: "4 kHz band" }), { target: { value: "-4.5" } });
    expect(onChange).toHaveBeenLastCalledWith("band7Db", -4.5);
    fireEvent.doubleClick(screen.getByRole("slider", { name: "1 kHz band" }));
    expect(onChange).toHaveBeenLastCalledWith("band5Db", 0);
    onChange.mockClear();
    fireEvent.click(screen.getByRole("button", { name: "Cut rumble" }));
    expect(onChange.mock.calls).toEqual([["band0Db", -12], ["band1Db", -8], ["band2Db", -3], ["band5Db", 0]]);
    await waitFor(() => expect(processorResponse).toHaveBeenCalled());
    const request = (processorResponse.mock.calls[0] as unknown as [{ bands: Array<{ type: string; q: number; frequencyHz: number }> }])[0];
    expect(request.bands[5]).toMatchObject({ type: "peaking", q: 1.4, frequencyHz: 1000 });
  });

  it("Bass & Treble: keyboard handles move level and frequency within range", () => {
    const onChange = vi.fn();
    render(<BassTrebleEditor node={node("bassTreble", { bassDb: 0, trebleDb: 11.5, bassFrequencyHz: 500, trebleFrequencyHz: 1500 })} backend={null} disabled={false} onChange={onChange} />);
    fireEvent.keyDown(screen.getByRole("slider", { name: "Bass shelf" }), { key: "ArrowUp" });
    expect(onChange).toHaveBeenLastCalledWith("bassDb", 0.5);
    fireEvent.keyDown(screen.getByRole("slider", { name: "Bass shelf" }), { key: "ArrowRight", shiftKey: true });
    expect(onChange).toHaveBeenLastCalledWith("bassFrequencyHz", 630);
    onChange.mockClear();
    fireEvent.keyDown(screen.getByRole("slider", { name: "Treble shelf" }), { key: "ArrowUp", shiftKey: true });
    expect(onChange).toHaveBeenLastCalledWith("trebleDb", 12);
    fireEvent.click(screen.getByRole("button", { name: "Radio voice" }));
    expect(onChange).toHaveBeenCalledWith("bassDb", -6);
    expect(onChange).toHaveBeenCalledWith("trebleDb", 4);
  });

  it("Dehum: draws the DSP harmonic cuts and switches mains frequency", () => {
    expect(dehumMagnitudeDb(60, 60, 100, 4)).toBeCloseTo(-36, 1);
    expect(dehumMagnitudeDb(180, 60, 100, 4)).toBeCloseTo(-36, 1);
    expect(Math.abs(dehumMagnitudeDb(90, 60, 100, 4))).toBeLessThan(0.5);
    expect(dehumMagnitudeDb(300, 60, 100, 4)).toBeGreaterThan(-0.5); // 5th harmonic not cut
    expect(dehumMagnitudeDb(60, 60, 0, 4)).toBe(0);
    const onChange = vi.fn();
    render(<DehumEditor node={node("dehum", { frequencyHz: 60, amountPercent: 50, harmonics: 4 })} disabled={false} onChange={onChange} />);
    expect(screen.getByText("18 dB")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "50 Hz" }));
    expect(onChange).toHaveBeenLastCalledWith("frequencyHz", 50);
  });

  it("Pitch: choices reset fine tune and the readout shows the frequency ratio", () => {
    const onChange = vi.fn();
    render(<PitchEditor node={node("pitch", { semitones: 0, cents: 20 })} disabled={false} onChange={onChange} />);
    expect(screen.getByText(/×1\.012/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "−12" }));
    expect(onChange.mock.calls).toEqual([["semitones", -12], ["cents", 0]]);
  });

  it("Delay, Volume and Gain explain the value in other units", () => {
    const onChange = vi.fn();
    render(<DelayEditor node={node("delay", { delayMs: 100 })} disabled={false} onChange={onChange} />);
    expect(screen.getByText("4800")).toBeTruthy();
    expect(screen.getByText("3.0")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "33 ms" }));
    expect(onChange).toHaveBeenLastCalledWith("delayMs", 33);
    cleanup();
    render(<LevelEditor node={node("volume", { percent: 200 })} disabled={false} onChange={onChange} />);
    expect(screen.getByText("= +6.0 dB")).toBeTruthy();
    cleanup();
    render(<LevelEditor node={node("gain", { gainDb: 24 })} disabled={false} onChange={onChange} />);
    expect(screen.getByText(/6\.0 dBFS — clips/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "−6 dB" }));
    expect(onChange).toHaveBeenLastCalledWith("gainDb", -6);
  });

  it("Input Switch names its sources and Strength editors offer named strengths", () => {
    const onChange = vi.fn();
    const switchNode = { ...node("inputSwitch", { selected: "a", fade: "normal" }), ports: [{ name: "a", direction: "input" as const, channels: 2 as const }, { name: "b", direction: "input" as const, channels: 2 as const }, { name: "out", direction: "output" as const, channels: 2 as const }] };
    const game = { ...node("physicalInput"), id: "game", name: "Game capture" };
    render(<InputSwitchEditor node={switchNode} session={{ nodes: [switchNode, game], edges: [{ sourceNode: "game", destinationNode: switchNode.id, destinationPort: "b", enabled: true }] }} disabled={false} onChange={onChange} />);
    const b = screen.getByRole("radio", { name: /B\s*Game capture/ });
    expect(b.getAttribute("aria-checked")).toBe("false");
    fireEvent.click(b);
    expect(onChange).toHaveBeenLastCalledWith("selected", "b");
    fireEvent.click(screen.getByRole("button", { name: "Slow fade (2 s)" }));
    expect(onChange).toHaveBeenLastCalledWith("fade", "slow");
    cleanup();
    render(<StrengthEditor node={node("speechDenoise", {})} disabled={false} onChange={onChange} />);
    expect(screen.getByRole("button", { name: "Medium · 70 %" }).getAttribute("aria-pressed")).toBe("true");
    fireEvent.click(screen.getByRole("button", { name: "Strong · 90 %" }));
    expect(onChange).toHaveBeenLastCalledWith("strengthPercent", 90);
  });

  it("formats caption values with units", () => {
    expect(formatParameterValue(3, "dB")).toBe("+3.0 dB");
    expect(formatParameterValue(-18, "dBFS")).toBe("−18.0 dBFS");
    expect(formatParameterValue(1500, "Hz", 1)).toBe("1.5 kHz");
    expect(formatParameterValue(12000, "Hz", 1)).toBe("12 kHz");
    expect(formatParameterValue(1250, "ms", 1)).toBe("1.25 s");
    expect(formatParameterValue(70, "%", 1)).toBe("70 %");
  });
});
