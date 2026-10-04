/** @vitest-environment jsdom */
import { afterEach, expect, it } from "vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import type { DiagnosticsSnapshot } from "@audiorouter/contracts";
import { LiveTelemetry, createTelemetryStore, differsOnlyInTelemetry } from "./liveTelemetry";

afterEach(cleanup);

const diagnostics = (level: number, adapter = "running") => ({
  nativeAdapter: adapter,
  privacyMute: { active: false },
  nodeTelemetry: [{ nodeId: "mic", meter: { peakDb: level } }],
}) as unknown as DiagnosticsSnapshot;
const level = (value: DiagnosticsSnapshot | null) => String((value?.nodeTelemetry[0] as unknown as { meter: { peakDb: number } } | undefined)?.meter.peakDb ?? "none");

it("tells meter-only refreshes from state changes", () => {
  expect(differsOnlyInTelemetry(diagnostics(-20), diagnostics(-12))).toBe(true);
  expect(differsOnlyInTelemetry(diagnostics(-20), diagnostics(-20, "configured-stopped"))).toBe(false);
  expect(differsOnlyInTelemetry(null, diagnostics(-20))).toBe(false);
});

it("updates only the subscribed view, and a newer snapshot wins at once", () => {
  const store = createTelemetryStore();
  const first = diagnostics(-20);
  let renders = 0;
  const view = (current: DiagnosticsSnapshot) => <LiveTelemetry store={store} diagnostics={current}>{(live) => { renders += 1; return <span data-testid="level">{level(live)}</span>; }}</LiveTelemetry>;
  const { rerender } = render(view(first));
  expect(screen.getByTestId("level").textContent).toBe("-20");
  act(() => store.set({ basis: first, nodeTelemetry: diagnostics(-6).nodeTelemetry }));
  expect(screen.getByTestId("level").textContent).toBe("-6");
  expect(renders).toBe(2);
  // Stop (or any state change) replaces the snapshot: stale live values are dropped.
  const stopped = { ...diagnostics(-20, "configured-stopped"), nodeTelemetry: [] } as unknown as DiagnosticsSnapshot;
  rerender(view(stopped));
  expect(screen.getByTestId("level").textContent).toBe("none");
});
