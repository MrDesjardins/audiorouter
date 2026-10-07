/** @vitest-environment jsdom */

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { Node } from "@audiorouter/contracts";
import { DuckEditor, duckSettings, duckSketch, duckTriggerChoices, roundFeedStatus } from "./DuckEditor";

afterEach(cleanup);

const node = (
  id: string,
  kind: Node["kind"],
  name: string,
  ins: boolean,
  outs: boolean,
  parameters: Node["parameters"] = {},
): Node => ({
  id,
  kind,
  typeVersion: 1,
  name,
  enabled: true,
  bypass: false,
  parameters,
  ports: [
    ...(ins ? [{ name: "in", direction: "input" as const, channels: 2 as const }] : []),
    ...(outs ? [{ name: "out", direction: "output" as const, channels: 2 as const }] : []),
  ],
});
const mic = node("mic", "physicalInput", "Microphone", false, true);
const gate = node("gate", "gate", "Voice Gate", true, true);
const out = node("out", "physicalOutput", "Headphones", true, false);
const duck = (parameters: Node["parameters"] = {}) =>
  node("duck-1", "duck", "Duck 1", true, true, {
    keyNodeId: "",
    thresholdDb: -35,
    amountDb: 6,
    attackMs: 20,
    holdMs: 300,
    releaseMs: 500,
    ...parameters,
  });
const telemetry = (keyDb: number, reduction: number, ducking: boolean) => ({
  gainReductionDb: [reduction, reduction],
  gateOpen: [ducking, ducking],
  inputLevelDb: [keyDb, keyDb],
  outputLevelDb: [-20, -20],
});

describe("Duck editor", () => {
  it("offers sources and tools as triggers, never itself or an output", () => {
    const choices = duckTriggerChoices({ nodes: [mic, gate, out, duck()] }, "duck-1");
    expect(choices.sources.map((choice) => choice.id)).toEqual(["mic"]);
    expect(choices.tools.map((choice) => choice.id)).toEqual(["gate"]);
  });

  it("sketches attack down to the amount, hold, then release back to unity", () => {
    const sketch = duckSketch({ ...duckSettings(duck()), amountDb: 10, attackMs: 5, holdMs: 200, releaseMs: 50 });
    const at = (ms: number) => sketch.points.find((point) => point.ms >= ms)!.gainDb;
    expect(at(100)).toBeCloseTo(0, 3);
    expect(at(600)).toBeCloseTo(-10, 1);
    expect(at(850)).toBeCloseTo(-10, 1); // hold ends at 900 ms
    expect(at(1500)).toBeGreaterThan(-0.1);
  });

  it("chooses a trigger, amount and trigger level, and asks for a trigger first", () => {
    const onChange = vi.fn();
    render(
      <DuckEditor
        node={duck()}
        session={{ nodes: [mic, gate, out, duck()] }}
        telemetry={null}
        running
        disabled={false}
        onChange={onChange}
      />,
    );
    expect(screen.getByRole("status").textContent).toBe("No trigger");
    fireEvent.change(screen.getByLabelText("Triggered by"), { target: { value: "mic" } });
    expect(onChange).toHaveBeenLastCalledWith("keyNodeId", "mic");
    fireEvent.change(screen.getByRole("slider", { name: "Duck amount" }), { target: { value: "10" } });
    expect(onChange).toHaveBeenLastCalledWith("amountDb", 10);
    fireEvent.keyDown(screen.getByRole("slider", { name: "Trigger level line" }), { key: "ArrowUp" });
    expect(onChange).toHaveBeenLastCalledWith("thresholdDb", -34);
  });

  it("shows ducking live and suggests a trigger level from voice and room noise", () => {
    const onChange = vi.fn();
    const ducked = duck({ keyNodeId: "mic" });
    const session = { nodes: [mic, out, ducked] };
    const view = render(
      <DuckEditor
        node={ducked}
        session={session}
        telemetry={telemetry(-60, 0, false)}
        running
        disabled={false}
        onChange={onChange}
      />,
    );
    expect(screen.getByRole("status").textContent).toBe("Listening");
    for (let index = 0; index < 60; index += 1) {
      const talking = index % 3 === 0;
      view.rerender(
        <DuckEditor
          node={ducked}
          session={session}
          telemetry={talking ? telemetry(-14, 6, true) : telemetry(-60, 0, false)}
          running
          disabled={false}
          onChange={onChange}
        />,
      );
    }
    view.rerender(
      <DuckEditor
        node={ducked}
        session={session}
        telemetry={telemetry(-14, 6, true)}
        running
        disabled={false}
        onChange={onChange}
      />,
    );
    expect(screen.getByRole("status").textContent).toBe("Ducking −6.0 dB");
    expect(screen.getByText(/Microphone is active: this audio is 6.0 dB down/)).toBeTruthy();
    expect(screen.getByRole("meter", { name: "Turned down" }).getAttribute("aria-valuetext")).toBe("−6.0 dB");
    fireEvent.click(screen.getByRole("button", { name: /^Use −\d+ dB$/ }));
    expect(onChange).toHaveBeenLastCalledWith("thresholdDb", -45);
  });

  it("explains Off and Bypass without live readings", () => {
    for (const flags of [{ enabled: false }, { bypass: true }]) {
      render(
        <DuckEditor
          node={{ ...duck({ keyNodeId: "mic" }), ...flags }}
          session={{ nodes: [mic] }}
          telemetry={telemetry(-14, 6, true)}
          running
          disabled={false}
          onChange={() => undefined}
        />,
      );
      expect(screen.getByRole("status").textContent).toBe(flags.enabled === false ? "Off" : "Bypassed");
      cleanup();
    }
  });

  it("switches to the Siege round trigger with phase choices and Stats.cc status", () => {
    const onChange = vi.fn();
    const round = {
      source: "statsCc" as const,
      state: "connected" as const,
      phase: "prep" as const,
      feedConfigured: true,
    };
    const view = render(
      <DuckEditor
        node={duck({ trigger: "siegeRound", duckBetweenRounds: false, amountDb: 20 })}
        session={{ nodes: [mic, gate, out] }}
        telemetry={telemetry(-120, 20, true)}
        running
        disabled={false}
        gameRound={round}
        onChange={onChange}
      />,
    );
    expect((screen.getByLabelText("Duck trigger") as HTMLSelectElement).value).toBe("siegeRound");
    expect(screen.queryByLabelText("Triggered by")).toBeNull();
    expect(screen.queryByRole("meter", { name: "Trigger" })).toBeNull();
    expect((screen.getByLabelText("Menu and matchmaking") as HTMLInputElement).checked).toBe(true);
    expect((screen.getByLabelText("Between rounds and results") as HTMLInputElement).checked).toBe(false);
    expect(screen.getByRole("status").textContent).toBe("Ducking −20.0 dB");
    expect(screen.getByText("Stats.cc: preparation.")).toBeTruthy();
    fireEvent.click(screen.getByLabelText("Between rounds and results"));
    expect(onChange).toHaveBeenCalledWith("duckBetweenRounds", true);
    fireEvent.change(screen.getByLabelText("Duck trigger"), { target: { value: "level" } });
    expect(onChange).toHaveBeenCalledWith("trigger", "level");
    view.rerender(
      <DuckEditor
        node={duck({ trigger: "siegeRound" })}
        session={{ nodes: [mic] }}
        telemetry={telemetry(-120, 0, false)}
        running
        disabled={false}
        gameRound={{ ...round, phase: "action" }}
        onChange={onChange}
      />,
    );
    expect(screen.getByRole("status").textContent).toBe("Full volume");
  });

  it("explains every Stats.cc feed state in plain language", () => {
    const round = (
      state: "off" | "connecting" | "waitingForUpdate" | "connected" | "unavailable",
      feedConfigured: boolean | null = true,
    ) => ({ source: "statsCc" as const, state, phase: "unknown" as const, feedConfigured });
    expect(roundFeedStatus(null, false)).toContain("Press Play");
    expect(roundFeedStatus(round("connecting"), true)).toBe("Connecting to Stats.cc…");
    expect(roundFeedStatus(round("unavailable", false), true)).toContain("examples\\integrations\\stats-cc-siege");
    expect(roundFeedStatus(round("unavailable"), true)).toContain("not running");
    expect(roundFeedStatus(round("waitingForUpdate"), true)).toContain("Full volume until");
  });
});
