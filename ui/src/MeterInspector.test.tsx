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
