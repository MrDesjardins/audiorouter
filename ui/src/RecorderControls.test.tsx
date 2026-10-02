/** @vitest-environment jsdom */

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { Node } from "@audiorouter/contracts";
import { RecordButton, RecorderControls, formatElapsed, isNodeRecording } from "./RecorderControls";

afterEach(cleanup);

const recorder = (parameters: Node["parameters"] = {}): Node => ({
  id: "rec", kind: "recorder", typeVersion: 1, name: "Podcast", enabled: true, bypass: false, parameters,
  ports: [{ name: "in", direction: "input", channels: 2 }],
});
const status = (state: "recording" | "completed") => ({ sessionId: "s", nodeId: "rec", state, lastFrame: 0 }) as never;

describe("Recorder controls", () => {
  it("records only while playing and stops at any time", () => {
    const onToggle = vi.fn();
    const view = render(<RecordButton node={recorder()} status={null} running={false} connected busy={false} onToggle={onToggle} />);
    expect((screen.getByRole("button", { name: "Record Podcast" }) as HTMLButtonElement).disabled).toBe(true);
    view.rerender(<RecordButton node={recorder()} status={null} running connected busy={false} onToggle={onToggle} />);
    fireEvent.click(screen.getByRole("button", { name: "Record Podcast" }));
    expect(onToggle).toHaveBeenLastCalledWith("rec", true);
    // A recording can always be stopped, even after playback stopped.
    view.rerender(<RecordButton node={recorder()} status={status("recording")} running={false} connected busy={false} onToggle={onToggle} />);
    const stop = screen.getByRole("button", { name: "Stop recording Podcast" });
    expect(stop.getAttribute("aria-pressed")).toBe("true");
    expect(stop.textContent).toContain("Stop · 0:00");
    fireEvent.click(stop);
    expect(onToggle).toHaveBeenLastCalledWith("rec", false);
  });

  it("summarises format, splitting and automatic start, and shows the last file", () => {
    render(<RecorderControls node={recorder({ format: "mp3", splitMinutes: 10, autoRecord: true })} status={null} running connected busy={false} lastPath={"C:\\Rec\\podcast-1.mp3"} message={null} onToggle={() => undefined} />);
    expect(screen.getByText("MP3 · new file every 10 min · starts with Play")).toBeTruthy();
    expect(screen.getByRole("status").textContent).toBe("Ready");
    expect(screen.getByText("podcast-1.mp3")).toBeTruthy();
  });

  it("formats elapsed time and recognises active states", () => {
    expect(formatElapsed(65)).toBe("1:05");
    expect(formatElapsed(3725)).toBe("1:02:05");
    expect(isNodeRecording(status("recording"))).toBe(true);
    expect(isNodeRecording(status("completed"))).toBe(false);
  });
});
