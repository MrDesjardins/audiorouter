/** @vitest-environment jsdom */

import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TestSignalPlaybackControls } from "./TestSignalPlaybackControls";

afterEach(cleanup);

describe("Test Signal playback controls", () => {
  it("explains a disabled Play while the route runs instead of only saying the tone is off", () => {
    const base = {
      durationMs: 1_000,
      routed: false,
      sessionRunning: true,
      playing: false,
      actionBusy: false,
      playbackReady: true,
      endpointPrepared: true,
      nodeEnabled: true,
      onStart: vi.fn(),
      onStop: vi.fn(),
    };
    const view = render(<TestSignalPlaybackControls {...base} />);
    expect((view.getByRole("button", { name: "Play tone" }) as HTMLButtonElement).disabled).toBe(true);
    expect(view.getByText("Connect to an enabled physical output.")).toBeTruthy();
    view.rerender(<TestSignalPlaybackControls {...base} routed />);
    expect(view.getByText("Tone off · 1.0 s")).toBeTruthy();
  });

  it("plays and stops only its tone while leaving the route running", () => {
    const onStart = vi.fn();
    const onStop = vi.fn();
    const props = {
      durationMs: 10_000,
      routed: true,
      sessionRunning: false,
      playing: false,
      actionBusy: false,
      playbackReady: true,
      endpointPrepared: true,
      nodeEnabled: true,
      onStart,
      onStop,
    };
    const view = render(<TestSignalPlaybackControls {...props} />);
    const play = view.getByRole("button", { name: "Play tone" });
    const stop = view.getByRole("button", { name: "Stop tone" });
    expect((play as HTMLButtonElement).disabled).toBe(false);
    expect((stop as HTMLButtonElement).disabled).toBe(true);
    expect(view.getByText(/Ready · 10 s/)).toBeTruthy();
    fireEvent.click(play);
    expect(onStart).toHaveBeenCalledTimes(1);

    view.rerender(<TestSignalPlaybackControls {...props} sessionRunning playing actionBusy={false} />);
    expect((play as HTMLButtonElement).disabled).toBe(true);
    expect((stop as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(stop);
    expect(onStop).toHaveBeenCalledTimes(1);
  });

  it.each([
    ["is not connected to an output", { routed: false, playbackReady: true, nodeEnabled: true }, true],
    ["has uncommitted changes", { routed: true, playbackReady: false, nodeEnabled: true }, false],
    ["is disabled", { routed: true, playbackReady: true, nodeEnabled: false }, true],
  ])("handles Play availability when the signal %s", (_reason, state, disabled) => {
    const onStart = vi.fn();
    const { getByRole, getByText } = render(
      <TestSignalPlaybackControls
        durationMs={1000}
        sessionRunning={false}
        playing={false}
        actionBusy={false}
        endpointPrepared
        onStart={onStart}
        onStop={vi.fn()}
        {...state}
      />,
    );
    const play = getByRole("button", { name: "Play tone" }) as HTMLButtonElement;
    expect(play.disabled).toBe(disabled);
    if (!state.routed) expect(getByText(/Connect to an enabled physical output/)).toBeTruthy();
    if (state.routed && !state.playbackReady) {
      fireEvent.click(play);
      expect(onStart).toHaveBeenCalledTimes(1);
    }
  });

  it("lets Play report output readiness when the route is committed", () => {
    const onStart = vi.fn();
    const { getByRole, getByText } = render(
      <TestSignalPlaybackControls
        durationMs={10_000}
        routed
        playbackReady
        endpointPrepared={false}
        nodeEnabled
        sessionRunning={false}
        playing={false}
        actionBusy={false}
        onStart={onStart}
        onStop={vi.fn()}
      />,
    );
    const play = getByRole("button", { name: "Play tone" });
    expect((play as HTMLButtonElement).disabled).toBe(false);
    expect(getByText(/Prepare endpoints in Devices/)).toBeTruthy();
    fireEvent.click(play);
    expect(onStart).toHaveBeenCalledTimes(1);
  });

  it("blocks duplicate lifecycle requests while starting or stopping", () => {
    const onStart = vi.fn();
    const onStop = vi.fn();
    const { getByRole, rerender } = render(
      <TestSignalPlaybackControls
        durationMs={1000}
        routed
        playbackReady
        endpointPrepared
        nodeEnabled
        sessionRunning={false}
        playing={false}
        actionBusy
        onStart={onStart}
        onStop={onStop}
      />,
    );
    const play = getByRole("button", { name: "Play tone" });
    expect((play as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(play);
    expect(onStart).not.toHaveBeenCalled();

    rerender(
      <TestSignalPlaybackControls
        durationMs={1000}
        routed
        playbackReady
        endpointPrepared
        nodeEnabled
        sessionRunning
        playing
        actionBusy
        onStart={onStart}
        onStop={onStop}
      />,
    );
    const stop = getByRole("button", { name: "Stop tone" });
    expect((stop as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(stop);
    expect(onStop).not.toHaveBeenCalled();
  });
});
