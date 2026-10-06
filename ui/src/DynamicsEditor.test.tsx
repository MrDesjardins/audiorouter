/** @vitest-environment jsdom */

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { Node } from "@audiorouter/contracts";
import { DynamicsEditor } from "./DynamicsEditor";

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

const node = (kind: "compressor" | "gate" | "limiter", parameters: Node["parameters"] = {}, flags: Partial<Node> = {}): Node => ({
  id: `${kind}-1`, kind, typeVersion: 1, name: kind, enabled: true, bypass: false, parameters, ...flags,
  ports: [{ name: "in", direction: "input", channels: 1 }, { name: "out", direction: "output", channels: 1 }],
});
const telemetry = (inputDb: number, outputDb: number, reductionDb: number, open = true) => ({ gainReductionDb: [reductionDb], gateOpen: [open], inputLevelDb: [inputDb], outputLevelDb: [outputDb] });

describe("Dynamics editor", () => {
  it("adjusts compressor threshold and ratio from keyboard-operable handles", () => {
    const onChange = vi.fn();
    render(<DynamicsEditor kind="compressor" node={node("compressor", { thresholdDb: -20, ratio: 4 })} telemetry={null} channels={1} running={false} disabled={false} onChange={onChange} />);
    const [threshold] = screen.getAllByRole("slider", { name: "Threshold" });
    expect(threshold.getAttribute("aria-valuenow")).toBe("-20");
    fireEvent.keyDown(threshold, { key: "ArrowUp" });
    expect(onChange).toHaveBeenLastCalledWith("thresholdDb", -19.5);
    fireEvent.keyDown(threshold, { key: "ArrowDown", shiftKey: true });
    expect(onChange).toHaveBeenLastCalledWith("thresholdDb", -25);
    fireEvent.keyDown(screen.getByRole("slider", { name: "Ratio" }), { key: "End" });
    expect(onChange).toHaveBeenLastCalledWith("ratio", 20);
    expect(screen.getByText(/Press Play, then talk/)).toBeTruthy();
    // The history threshold line is also draggable and shares the value.
    expect(screen.getByRole("slider", { name: "Threshold line" }).getAttribute("aria-valuenow")).toBe("-20");
  });

  it("clamps handle changes to the parameter range", () => {
    const onChange = vi.fn();
    render(<DynamicsEditor kind="gate" node={node("gate", { thresholdDb: -80, hysteresisDb: 12 })} telemetry={null} channels={1} running={false} disabled={false} onChange={onChange} />);
    fireEvent.keyDown(screen.getByRole("slider", { name: "Threshold (opens)" }), { key: "ArrowDown" });
    expect(onChange).not.toHaveBeenCalled();
    fireEvent.keyDown(screen.getByRole("slider", { name: "Hysteresis (closes this far below)" }), { key: "ArrowUp" });
    expect(onChange).not.toHaveBeenCalled();
    fireEvent.keyDown(screen.getByRole("slider", { name: "Hysteresis (closes this far below)" }), { key: "ArrowDown" });
    expect(onChange).toHaveBeenLastCalledWith("hysteresisDb", 11.5);
  });

  it("shows live gate state, levels and a threshold suggestion from speech and room noise", () => {
    const onChange = vi.fn();
    const gate = node("gate", { thresholdDb: -30 });
    const view = render(<DynamicsEditor kind="gate" node={gate} telemetry={telemetry(-62, -122, 60, false)} channels={1} running disabled={false} onChange={onChange} />);
    expect(screen.getByRole("status").textContent).toBe("Closed −60 dB");
    expect(screen.getByText(/Gate closed/)).toBeTruthy();
    for (let index = 0; index < 60; index += 1) {
      const speaking = index % 3 === 0;
      view.rerender(<DynamicsEditor kind="gate" node={gate} telemetry={speaking ? telemetry(-18, -18, 0, true) : telemetry(-62, -122, 60, false)} channels={1} running disabled={false} onChange={onChange} />);
    }
    expect(screen.getByRole("status").textContent).toBe("Closed −60 dB");
    view.rerender(<DynamicsEditor kind="gate" node={gate} telemetry={telemetry(-18, -18, 0, true)} channels={1} running disabled={false} onChange={onChange} />);
    expect(screen.getByRole("status").textContent).toBe("Open");
    expect(screen.getByText(/room noise ≈ −62 dB, voice ≈ −18 dB/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Use −47 dB" }));
    expect(onChange).toHaveBeenLastCalledWith("thresholdDb", -47);
    expect(screen.getByRole("meter", { name: "Level entering the tool" }).getAttribute("aria-valuetext")).toBe("−18.0 dB");
  });

  it("reserves the compressor suggestion, keeps it through a lapse and flags a quiet voice", () => {
    // Telemetry arrives every 50 ms in the app; advancing a fake clock keeps
    // the level history to its real 8-second window instead of letting it
    // grow with every frame.
    let clock = 0;
    vi.spyOn(performance, "now").mockImplementation(() => clock);
    const onChange = vi.fn();
    const compressor = node("compressor", { thresholdDb: -18, ratio: 3 });
    const view = render(<DynamicsEditor kind="compressor" node={compressor} telemetry={telemetry(-62, -62, 0)} channels={1} running disabled={false} onChange={onChange} />);
    const box = view.container.querySelector(".dynamics-suggestion");
    expect(box?.textContent).toMatch(/talk normally for a few seconds/);
    expect((screen.getByRole("button", { name: "Use suggestion" }) as HTMLButtonElement).disabled).toBe(true);
    for (let index = 0; index < 60; index += 1) {
      clock += 50;
      view.rerender(<DynamicsEditor kind="compressor" node={compressor} telemetry={index % 3 === 0 ? telemetry(-36, -36, 0) : telemetry(-62, -62, 0)} channels={1} running disabled={false} onChange={onChange} />);
    }
    expect(view.container.querySelector(".dynamics-suggestion")).toBe(box);
    expect(box?.textContent).toMatch(/voice ≈ −36 dB\. Suggested threshold −45 dB \(about 6 dB off your loud words at 3\.0:1\)/);
    expect(box?.textContent).toMatch(/Your voice is quiet/);
    // 8.5 s of steady level with no pauses leaves only steady samples in the
    // 8-second window, so the live estimate is unusable; the last suggestion
    // stays. Sparser frames cover that time without re-rendering 170 times.
    for (let index = 0; index < 34; index += 1) {
      clock += 250;
      view.rerender(<DynamicsEditor kind="compressor" node={compressor} telemetry={telemetry(-36, -36, 0)} channels={1} running disabled={false} onChange={onChange} />);
    }
    fireEvent.click(screen.getByRole("button", { name: "Use −45 dB" }));
    expect(onChange).toHaveBeenLastCalledWith("thresholdDb", -45);
  });

  it("reports compressor gain reduction and explains bypass without stale levels", () => {
    const compressor = node("compressor", { thresholdDb: -24, ratio: 3 });
    const view = render(<DynamicsEditor kind="compressor" node={compressor} telemetry={telemetry(-12, -20, 8)} channels={1} running disabled={false} onChange={() => undefined} />);
    expect(screen.getByRole("status").textContent).toBe("−8.0 dB");
    expect(view.container.querySelector(".dynamics-live-dot")).not.toBeNull();
    view.rerender(<DynamicsEditor kind="compressor" node={{ ...compressor, bypass: true }} telemetry={telemetry(-12, -20, 8)} channels={1} running disabled={false} onChange={() => undefined} />);
    expect(screen.getByText(/Bypassed: sound passes unchanged/)).toBeTruthy();
    expect(view.container.querySelector(".dynamics-live-dot")).toBeNull();
  });

  it("asks for a rebuild when the backend reports gain reduction only", () => {
    render(<DynamicsEditor kind="limiter" node={node("limiter")} telemetry={{ gainReductionDb: [2], gateOpen: [] }} channels={1} running disabled={false} onChange={() => undefined} />);
    expect(screen.getByText(/reports gain reduction only/)).toBeTruthy();
    expect(screen.getByRole("slider", { name: "Ceiling" }).getAttribute("aria-valuenow")).toBe("-1");
  });

  it("disables every handle while disconnected", () => {
    const onChange = vi.fn();
    render(<DynamicsEditor kind="compressor" node={node("compressor")} telemetry={null} channels={1} running={false} disabled onChange={onChange} />);
    for (const slider of screen.getAllByRole("slider")) {
      expect(slider.getAttribute("aria-disabled")).toBe("true");
      fireEvent.keyDown(slider, { key: "ArrowUp" });
    }
    expect(onChange).not.toHaveBeenCalled();
  });
});
