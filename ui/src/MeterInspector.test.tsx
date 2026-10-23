/** @vitest-environment jsdom */
import { afterEach, describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, waitFor, cleanup } from "@testing-library/react";
import { MeterInspector, VOICE_ZONE, levelVerdict } from "./MeterInspector";
import { createDisconnectedBackend } from "./backend";
import { demoSession } from "./fixtures";
afterEach(cleanup);
it("offers an explicit output upgrade for saved input-only meters", () => {
  const onUpgrade = vi.fn();
  const node = {
    ...demoSession.nodes[0],
    id: "legacy-meter",
    kind: "meter" as const,
    bypass: true,
    ports: [{ name: "in", direction: "input" as const, channels: 1 as const }],
  };
  render(
    <MeterInspector
      node={node}
      sessionId="session"
      snapshot={null}
      running
      backend={{ ...createDisconnectedBackend(), connected: true }}
      onUpgrade={onUpgrade}
    />,
  );
  expect(screen.getByText("Bypass: metering is inactive; sound passes through.")).toBeTruthy();
  expect(screen.getByRole("meter").getAttribute("aria-valuenow")).toBe("-60");
  expect((screen.getByRole("button", { name: "Reset peak & clipping" }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Add pass-through output" }));
  expect(onUpgrade).toHaveBeenCalledOnce();
});
it("shows exact per-channel hold, clip duration and reset without saving", async () => {
  const backend = createDisconnectedBackend();
  const initial = await backend.snapshot();
  const resetMeter = vi.fn(async () => ({ sessionId: "session", nodeId: "meter", reset: true }));
  const node = {
    ...demoSession.nodes[0],
    id: "meter",
    kind: "meter" as const,
    ports: [
      { name: "in", direction: "input" as const, channels: 2 as const },
      { name: "out", direction: "output" as const, channels: 2 as const },
    ],
  };
  render(
    <MeterInspector
      node={node}
      sessionId="session"
      snapshot={{
        ...initial.diagnostics,
        nodeTelemetry: [
          {
            nodeId: "meter",
            kind: "meter",
            processor: null,
            plugin: null,
            meter: {
              peakDb: 3,
              rmsDb: -18,
              currentPeakDb: -6,
              channelCurrentPeakDb: [-6, -9],
              channelPeakDb: [3, -1],
              channelRmsDb: [-18, -21],
              clippedSamples: 480,
              channelClippedSamples: [480, 0],
              observedFrames: 48000,
              sampleRateHz: 48000,
            },
          },
        ],
      }}
      running
      backend={{ ...backend, connected: true, resetMeter }}
      onUpgrade={() => {}}
    />,
  );
  expect(screen.getAllByRole("meter")).toHaveLength(2);
  expect(screen.getByText("0.010 s")).toBeTruthy();
  expect(screen.getByText("1.00%")).toBeTruthy();
  expect(screen.getByText("3.0 dBFS")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Reset peak & clipping" }));
  await waitFor(() => expect(resetMeter).toHaveBeenCalledWith("session", "meter"));
});
it("holds the recent-peak line when the reading drops, while the readout stays exact", async () => {
  const backend = createDisconnectedBackend();
  const initial = await backend.snapshot();
  const node = {
    ...demoSession.nodes[0],
    id: "meter",
    kind: "meter" as const,
    bypass: false,
    enabled: true,
    ports: [
      { name: "in", direction: "input" as const, channels: 1 as const },
      { name: "out", direction: "output" as const, channels: 1 as const },
    ],
  };
  const snapshot = (peak: number) => ({
    ...initial.diagnostics,
    nodeTelemetry: [
      {
        nodeId: "meter",
        kind: "meter",
        processor: null,
        plugin: null,
        meter: {
          peakDb: -3,
          rmsDb: peak - 10,
          currentPeakDb: peak,
          channelCurrentPeakDb: [peak],
          channelPeakDb: [-3],
          channelRmsDb: [peak - 10],
          clippedSamples: 0,
          channelClippedSamples: [0],
          observedFrames: 4800,
          sampleRateHz: 48000,
        },
      },
    ],
  });
  const view = render(
    <MeterInspector
      node={node}
      sessionId="session"
      snapshot={snapshot(-6)}
      running
      backend={{ ...backend, connected: true }}
      onUpgrade={() => {}}
    />,
  );
  const marker = () => (view.container.querySelector(".meter-current-marker") as HTMLElement).style.bottom;
  const loud = marker();
  view.rerender(
    <MeterInspector
      node={node}
      sessionId="session"
      snapshot={snapshot(-40)}
      running
      backend={{ ...backend, connected: true }}
      onUpgrade={() => {}}
    />,
  );
  expect(marker()).toBe(loud);
  expect(screen.getByText("-40.0 dBFS")).toBeTruthy();
});
it("judges voice peaks against the target zone (UI-21)", () => {
  expect(levelVerdict(undefined)).toBe("waiting");
  expect(levelVerdict(Number.NaN)).toBe("waiting");
  expect(levelVerdict(-Infinity)).toBe("waiting");
  expect(levelVerdict(-120)).toBe("waiting");
  expect(levelVerdict(-50.1)).toBe("waiting");
  expect(levelVerdict(-50)).toBe("quiet");
  expect(levelVerdict(-18.1)).toBe("quiet");
  expect(levelVerdict(VOICE_ZONE.lowDb)).toBe("zone");
  expect(levelVerdict(-12)).toBe("zone");
  expect(levelVerdict(VOICE_ZONE.highDb)).toBe("zone");
  expect(levelVerdict(-5.9)).toBe("loud");
  expect(levelVerdict(-1.1)).toBe("loud");
  expect(levelVerdict(-1)).toBe("tooLoud");
  expect(levelVerdict(3)).toBe("tooLoud");
  expect(levelVerdict(Infinity)).toBe("tooLoud");
});
describe("voice target zone in the inspector", () => {
  const node = {
    ...demoSession.nodes[0],
    id: "meter",
    kind: "meter" as const,
    bypass: false,
    enabled: true,
    ports: [
      { name: "in", direction: "input" as const, channels: 2 as const },
      { name: "out", direction: "output" as const, channels: 2 as const },
    ],
  };
  const view = async (peaks: [number, number] | null, props: { running?: boolean; bypass?: boolean } = {}) => {
    const backend = createDisconnectedBackend();
    const initial = await backend.snapshot();
    const snapshot = {
      ...initial.diagnostics,
      nodeTelemetry: peaks
        ? [
            {
              nodeId: "meter",
              kind: "meter",
              processor: null,
              plugin: null,
              meter: {
                peakDb: Math.max(...peaks),
                rmsDb: -30,
                currentPeakDb: Math.max(...peaks),
                channelCurrentPeakDb: peaks,
                channelPeakDb: peaks,
                channelRmsDb: [-30, -30],
                clippedSamples: 0,
                channelClippedSamples: [0, 0],
                observedFrames: 4800,
                sampleRateHz: 48000,
              },
            },
          ]
        : [],
    };
    return render(
      <MeterInspector
        node={{ ...node, bypass: props.bypass ?? false }}
        sessionId="session"
        snapshot={snapshot}
        running={props.running ?? true}
        backend={{ ...backend, connected: true }}
        onUpgrade={() => {}}
      />,
    );
  };
  const verdict = (container: HTMLElement) => container.querySelector(".meter-level-verdict") as HTMLElement;

  it("uses the loudest channel", async () => {
    const { container } = await view([-30, -12]);
    expect(verdict(container).dataset.verdict).toBe("zone");
    expect(verdict(container).textContent).toBe("Voice guide: in the zone");
  });
  it("says too quiet and too loud", async () => {
    expect(verdict((await view([-30, -25])).container).dataset.verdict).toBe("quiet");
    cleanup();
    expect(verdict((await view([-0.5, -40])).container).dataset.verdict).toBe("tooLoud");
  });
  it("waits while stopped, bypassed or without readings, keeping the line in place", async () => {
    for (const [peaks, props] of [
      [[-12, -12], { running: false }],
      [[-12, -12], { bypass: true }],
      [null, {}],
    ] as const) {
      const { container } = await view(peaks as [number, number] | null, props);
      expect(verdict(container).dataset.verdict).toBe("waiting");
      expect(verdict(container).textContent).toBe("Voice guide: waiting for sound");
      cleanup();
    }
  });
  it("draws the zone band at the same place on every channel", async () => {
    const { container } = await view([-12, -12]);
    const bands = [...container.querySelectorAll(".meter-target-zone")] as HTMLElement[];
    expect(bands).toHaveLength(2);
    expect(new Set(bands.map((band) => band.style.bottom + "/" + band.style.height)).size).toBe(1);
    expect(parseFloat(bands[0].style.bottom)).toBeCloseTo(((-18 + 60) / 66) * 100, 5);
    expect(parseFloat(bands[0].style.height)).toBeCloseTo((12 / 66) * 100, 5);
  });
});
