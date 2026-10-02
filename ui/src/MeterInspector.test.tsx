/** @vitest-environment jsdom */
import { afterEach, it, expect, vi } from "vitest";
import { render, screen, fireEvent, waitFor, cleanup } from "@testing-library/react";
import { MeterInspector } from "./MeterInspector";
import { createDisconnectedBackend } from "./backend";
import { demoSession } from "./fixtures";
afterEach(cleanup);
it("offers an explicit output upgrade for saved input-only meters", () => {
  const onUpgrade = vi.fn();
  const node = { ...demoSession.nodes[0], id: "legacy-meter", kind: "meter" as const, bypass: true,
    ports: [{ name: "in", direction: "input" as const, channels: 1 as const }] };
  render(<MeterInspector node={node} sessionId="session" snapshot={null} running backend={{ ...createDisconnectedBackend(), connected: true }} onUpgrade={onUpgrade} />);
  expect(screen.getByText("Bypass: metering is inactive; sound passes through.")).toBeTruthy();
  expect(screen.getByRole("meter").getAttribute("aria-valuenow")).toBe("-60");
  expect((screen.getByRole("button", { name: "Reset peak & clipping" }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Add pass-through output" }));
  expect(onUpgrade).toHaveBeenCalledOnce();
});
it("shows exact per-channel hold, clip duration and reset without saving", async () => {
  const backend = createDisconnectedBackend(); const initial = await backend.snapshot();
  const resetMeter = vi.fn(async () => ({ sessionId: "session", nodeId: "meter", reset: true }));
  const node = { ...demoSession.nodes[0], id: "meter", kind: "meter" as const, ports: [{ name: "in", direction: "input" as const, channels: 2 as const }, { name: "out", direction: "output" as const, channels: 2 as const }] };
  render(<MeterInspector node={node} sessionId="session" snapshot={{ ...initial.diagnostics, nodeTelemetry: [{ nodeId: "meter", kind: "meter", processor: null, plugin: null, meter: { peakDb: 3, rmsDb: -18, currentPeakDb: -6, channelCurrentPeakDb: [-6, -9], channelPeakDb: [3, -1], channelRmsDb: [-18, -21], clippedSamples: 480, channelClippedSamples: [480, 0], observedFrames: 48000, sampleRateHz: 48000 } }] }} running backend={{ ...backend, connected: true, resetMeter }} onUpgrade={() => {}} />);
  expect(screen.getAllByRole("meter")).toHaveLength(2);
  expect(screen.getByText("0.010 s")).toBeTruthy();
  expect(screen.getByText("1.00%")).toBeTruthy();
  expect(screen.getByText("3.0 dBFS")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Reset peak & clipping" }));
  await waitFor(() => expect(resetMeter).toHaveBeenCalledWith("session", "meter"));
});
it("holds the recent-peak line when the reading drops, while the readout stays exact", async () => {
  const backend = createDisconnectedBackend(); const initial = await backend.snapshot();
  const node = { ...demoSession.nodes[0], id: "meter", kind: "meter" as const, bypass: false, enabled: true, ports: [{ name: "in", direction: "input" as const, channels: 1 as const }, { name: "out", direction: "output" as const, channels: 1 as const }] };
  const snapshot = (peak: number) => ({ ...initial.diagnostics, nodeTelemetry: [{ nodeId: "meter", kind: "meter", processor: null, plugin: null, meter: { peakDb: -3, rmsDb: peak - 10, currentPeakDb: peak, channelCurrentPeakDb: [peak], channelPeakDb: [-3], channelRmsDb: [peak - 10], clippedSamples: 0, channelClippedSamples: [0], observedFrames: 4800, sampleRateHz: 48000 } }] });
  const view = render(<MeterInspector node={node} sessionId="session" snapshot={snapshot(-6)} running backend={{ ...backend, connected: true }} onUpgrade={() => {}} />);
  const marker = () => (view.container.querySelector(".meter-current-marker") as HTMLElement).style.bottom;
  const loud = marker();
  view.rerender(<MeterInspector node={node} sessionId="session" snapshot={snapshot(-40)} running backend={{ ...backend, connected: true }} onUpgrade={() => {}} />);
  expect(marker()).toBe(loud);
  expect(screen.getByText("-40.0 dBFS")).toBeTruthy();
});
