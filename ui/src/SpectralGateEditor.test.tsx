/** @vitest-environment jsdom */

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { Node } from "@audiorouter/contracts";
import { BAND_FREQUENCIES_HZ, SpectralGateEditor, decodeProfileDb } from "./SpectralGateEditor";

afterEach(cleanup);

const node = (parameters: Node["parameters"]): Node => ({
  id: "gate-hz", kind: "spectralGate", typeVersion: 1, name: "FIR Filter Hz", enabled: true, bypass: false, parameters,
  ports: [{ name: "in", direction: "input", channels: 2 }, { name: "out", direction: "output", channels: 2 }],
});
const profile = "64".repeat(64); // 100 − 160 = −60 dB in every band

describe("FIR Filter Hz editor", () => {
  it("decodes stored profiles and spans the audible range", () => {
    expect(decodeProfileDb(profile)).toEqual(Array(64).fill(-60));
    expect(decodeProfileDb("xyz")).toBeNull();
    expect(BAND_FREQUENCIES_HZ).toHaveLength(64);
    expect(BAND_FREQUENCIES_HZ.every((hz, index) => index === 0 || hz > BAND_FREQUENCIES_HZ[index - 1])).toBe(true);
    expect(BAND_FREQUENCIES_HZ.at(-1)!).toBeGreaterThan(18_000);
  });

  it("draws the live sound only while the route plays", () => {
    const levels = Array(64).fill(-20);
    const { container, rerender } = render(<SpectralGateEditor node={node({})} running levelsDb={levels} liveProfile={null} disabled={false} onChange={() => undefined} />);
    expect(container.querySelector(".spectral-gate-live")).not.toBeNull();
    // After Stop the last telemetry may still be present; it is not live sound.
    rerender(<SpectralGateEditor node={node({})} running={false} levelsDb={levels} liveProfile={null} disabled={false} onChange={() => undefined} />);
    expect(container.querySelector(".spectral-gate-live")).toBeNull();
  });

  it("starts learning only while playing", () => {
    const onChange = vi.fn();
    const { rerender } = render(<SpectralGateEditor node={node({})} running={false} levelsDb={null} liveProfile={null} disabled={false} onChange={onChange} />);
    expect((screen.getByRole("button", { name: "Learn noise" }) as HTMLButtonElement).disabled).toBe(true);
    rerender(<SpectralGateEditor node={node({})} running levelsDb={Array(64).fill(-20)} liveProfile={null} disabled={false} onChange={onChange} />);
    fireEvent.click(screen.getByRole("button", { name: "Learn noise" }));
    expect(onChange).toHaveBeenCalledWith([["learning", true]]);
  });

  it("keeps the learned profile when learning stops and draws the threshold", () => {
    const onChange = vi.fn();
    const { container, rerender } = render(<SpectralGateEditor node={node({ learning: true })} running levelsDb={Array(64).fill(-20)} liveProfile={profile} disabled={false} onChange={onChange} />);
    fireEvent.click(screen.getByRole("button", { name: "Stop and keep" }));
    expect(onChange).toHaveBeenCalledWith([["noiseProfile", profile], ["learning", false]]);
    rerender(<SpectralGateEditor node={node({ noiseProfile: profile, thresholdDb: 6 })} running levelsDb={Array(64).fill(-20)} liveProfile={null} disabled={false} onChange={onChange} />);
    expect(container.querySelector(".spectral-gate-threshold")).not.toBeNull();
    expect(screen.getByRole("button", { name: "Learn again" })).toBeTruthy();
  });
});
