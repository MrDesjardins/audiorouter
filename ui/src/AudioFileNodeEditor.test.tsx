/** @vitest-environment jsdom */
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Node } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";
import { AudioFileNodeEditor } from "./AudioFileNodeEditor";
import { demoSession } from "./fixtures";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

function soundCheckFixture() {
  const recorderId = `audio-file-take-${(1_000_000).toString(36)}`;
  const audioNode = {
    id: "audio-file",
    kind: "audioFile",
    typeVersion: 1,
    name: "Sound Check replay",
    enabled: true,
    bypass: false,
    parameters: { fileName: "No file selected", loop: false },
    ports: [{ name: "out", direction: "output", channels: 1 }],
  } as Node;
  const recorder = {
    id: "voice-recorder",
    kind: "recorder",
    typeVersion: 1,
    name: "Voice Recorder",
    enabled: true,
    bypass: false,
    parameters: {},
    ports: [
      { name: "in", direction: "input", channels: 1 },
      { name: "out", direction: "output", channels: 1 },
    ],
  } as Node;
  const stopRecorder = vi.fn(async () => undefined);
  const backend = {
    connected: true,
    createRecorder: vi.fn(async () => undefined),
    armRecorder: vi.fn(async () => undefined),
    startRecorder: vi.fn(async () => undefined),
    listRecorders: vi.fn(async () => [{ sessionId: demoSession.id, nodeId: recorder.id, lastFrame: 128 }]),
    stopRecorder,
    listRecordings: vi.fn(async () => [{ id: "take-row", recorderId, state: "completed" }]),
    importTemporaryRecording: vi.fn(async () => ({
      mediaId: "temporary-media",
      fileName: "voice.wav",
      durationMs: 10_000,
    })),
  } as unknown as UiBackend;
  return { audioNode, recorder, backend, stopRecorder };
}

describe("Audio File Sound Check", () => {
  it("automatically stops and prepares the take after 10 seconds", async () => {
    vi.useFakeTimers();
    vi.spyOn(Date, "now").mockReturnValue(1_000_000);
    const { audioNode, recorder, backend, stopRecorder } = soundCheckFixture();
    const onChange = vi.fn();
    render(
      <AudioFileNodeEditor
        node={audioNode}
        backend={backend}
        disabled={false}
        transportDisabled={false}
        sessionRunning
        state="stopped"
        sessionId={demoSession.id}
        currentSessionIdRef={{ current: demoSession.id }}
        recorderNodes={[recorder]}
        recorderStatuses={[]}
        onChange={onChange}
        onTransport={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Record Sound Check" }));
    await act(async () => {
      for (let index = 0; index < 8; index += 1) await Promise.resolve();
    });
    expect(screen.getByText(/stops automatically after 10 seconds/)).toBeTruthy();
    expect(stopRecorder).not.toHaveBeenCalled();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(9_999);
    });
    expect(stopRecorder).not.toHaveBeenCalled();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
      for (let index = 0; index < 12; index += 1) await Promise.resolve();
    });
    expect(stopRecorder).toHaveBeenCalledOnce();
    expect(backend.importTemporaryRecording).toHaveBeenCalledWith("take-row");
    expect(onChange).toHaveBeenCalledWith("mediaId", "temporary-media");
    expect(onChange).toHaveBeenCalledWith("fileName", "voice.wav");
    expect(
      screen.getByText(/Plan and commit this Audio File source before replaying it through the effects/),
    ).toBeTruthy();
  });
});
