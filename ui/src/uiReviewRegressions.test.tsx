/** @vitest-environment jsdom */
import { act, cleanup, fireEvent, render, renderHook } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { Node, Session } from "@audiorouter/contracts";
import { InstrumentNodeEditor } from "./InstrumentNodeEditor";
import { useAudioFileStatus, type SourceTransportActivity } from "./useAudioFileStatus";
import { createDisconnectedBackend, type UiBackend } from "./backend";
import { demoSession } from "./fixtures";
import { PluginNodeInspector } from "./PluginPanels";
import { AudioFileNodeEditor } from "./AudioFileNodeEditor";
import { generatedOnlyRoute, needsNativePaths } from "./draft";
import { useSoundboardController } from "./soundboardLive";
import { useState } from "react";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});
const instrument = (id: string): Node => ({
  id,
  name: id,
  kind: "instrument",
  typeVersion: 1,
  enabled: true,
  bypass: false,
  parameters: {},
  ports: [{ name: "out", direction: "output", channels: 2 }],
});
const sourceSession = (count = 1): Session => ({
  ...demoSession,
  nodes: Array.from({ length: count }, (_, i) => ({ ...instrument(`a${i}`), kind: "audioFile" })),
});
const deferred = <T,>() => {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
};

it("selects native paths for a standalone instrument, even with remembered capture settings", () => {
  const session: Session = {
    ...demoSession,
    nodes: [
      instrument("synth"),
      {
        ...instrument("out"),
        kind: "physicalOutput",
        parameters: { endpointId: "headphones" },
        ports: [{ name: "in", direction: "input", channels: 2 }],
      },
    ],
    edges: [
      {
        id: "e",
        sourceNode: "synth",
        sourcePort: "out",
        destinationNode: "out",
        destinationPort: "in",
        enabled: true,
        matrix: [1, 0, 0, 1],
      },
    ],
  };
  expect(needsNativePaths(session)).toBe(true);
  expect(generatedOnlyRoute(session)).toBe(true);
});

for (const change of ["node", "session", "disabled", "unmount"] as const) {
  it(`releases the original instrument after ${change}, behind an unacknowledged noteOn`, async () => {
    const pending = deferred<never>();
    const send = vi.fn().mockReturnValueOnce(pending.promise).mockResolvedValue({});
    const backend = { ...createDisconnectedBackend(), connected: true, midiSendEvents: send };
    const view = (id: string, sessionId = "s", disabled = false) => (
      <InstrumentNodeEditor
        node={instrument(id)}
        backend={backend}
        sessionId={sessionId}
        running
        disabled={disabled}
        onChange={() => {}}
      />
    );
    const result = render(view("a"));
    fireEvent.pointerDown(result.getByRole("button", { name: "C4" }));
    if (change === "unmount") result.unmount();
    else result.rerender(view(change === "node" ? "b" : "a", change === "session" ? "t" : "s", change === "disabled"));
    expect(send).toHaveBeenCalledTimes(1);
    await act(async () => {
      pending.resolve({} as never);
    });
    expect(send.mock.calls).toEqual([
      ["s", "a", [{ type: "noteOn", note: 60, velocity: 100 }]],
      ["s", "a", [{ type: "allNotesOff" }]],
    ]);
  });
}

for (const key of ["Enter", " "]) {
  it(`supports ${key} and releases on blur without repeats or a second key-up release`, async () => {
    const send = vi.fn().mockResolvedValue({});
    const backend = { ...createDisconnectedBackend(), connected: true, midiSendEvents: send };
    const result = render(
      <InstrumentNodeEditor
        node={instrument("a")}
        backend={backend}
        sessionId="s"
        running
        disabled={false}
        onChange={() => {}}
      />,
    );
    const button = result.getByRole("button", { name: "C4" });
    await act(async () => {
      fireEvent.keyDown(button, { key });
      fireEvent.keyDown(button, { key, repeat: true });
      fireEvent.blur(button);
      fireEvent.keyUp(button, { key });
    });
    expect(send.mock.calls).toEqual([
      ["s", "a", [{ type: "noteOn", note: 60, velocity: 100 }]],
      ["s", "a", [{ type: "noteOff", note: 60 }]],
    ]);
  });
}

it("discovers externally started sources in rotating batches without one failed source blocking the others", async () => {
  vi.useFakeTimers();
  const transport = vi.fn(async (_session: string, id: string) => {
    if (id === "a0") throw new Error("not prepared");
    return { state: "playing" };
  });
  const backend = {
    ...createDisconnectedBackend(),
    connected: true,
    transportAudioSource: transport,
  } as unknown as UiBackend;
  const session = sourceSession(18);
  const result = renderHook(() => {
    const [states, setStates] = useState<Record<string, "playing" | "paused" | "stopped">>({});
    useAudioFileStatus({
      backend,
      draft: session,
      session,
      sessionRunning: true,
      audioSourceStates: states,
      setAudioSourceStates: setStates,
      recordUiDiagnostic: () => {},
    });
    return states;
  });
  await act(async () => {
    await vi.advanceTimersByTimeAsync(750);
  });
  expect(transport).toHaveBeenCalledTimes(16);
  expect(result.result.current.a1).toBe("playing");
  await act(async () => {
    await vi.advanceTimersByTimeAsync(750);
  });
  expect(result.result.current.a17).toBe("playing");
});

it("does not publish source status after Stop or a newer transport command", async () => {
  vi.useFakeTimers();
  const pending = deferred<{ state: "playing" }>();
  const transport = vi.fn(() => pending.promise);
  const backend = {
    ...createDisconnectedBackend(),
    connected: true,
    transportAudioSource: transport,
  } as unknown as UiBackend;
  const session = sourceSession();
  const activity: { current: SourceTransportActivity } = { current: new Map() };
  const key = JSON.stringify([session.id, "a0"]);
  const result = renderHook(
    ({ running }) => {
      const [states, setStates] = useState<Record<string, "playing" | "paused" | "stopped">>({ a0: "stopped" });
      useAudioFileStatus({
        backend,
        draft: session,
        session,
        sessionRunning: running,
        audioSourceStates: states,
        setAudioSourceStates: setStates,
        recordUiDiagnostic: () => {},
        transportActivity: activity,
      });
      return states;
    },
    { initialProps: { running: true } },
  );
  await act(async () => {
    await vi.advanceTimersByTimeAsync(750);
  });
  activity.current.set(key, { version: 1, pending: 0 });
  await act(async () => {
    pending.resolve({ state: "playing" });
  });
  expect(result.result.current.a0).toBe("stopped");
  const late = deferred<{ state: "playing" }>();
  transport.mockReturnValue(late.promise);
  await act(async () => {
    await vi.advanceTimersByTimeAsync(750);
  });
  result.rerender({ running: false });
  await act(async () => {
    late.resolve({ state: "playing" });
  });
  expect(result.result.current).toEqual({});
});

it("forgets plugin-specific state and late loads on node or session changes", async () => {
  const late = deferred<{ programs: { id: number; name: string }[]; units: [] }>();
  const backend = {
    listPluginPrograms: vi
      .fn()
      .mockResolvedValueOnce({ programs: [{ id: 0, name: "Only A" }], units: [] })
      .mockReturnValueOnce(late.promise),
  };
  const view = (id: string, sessionId = "s") => (
    <PluginNodeInspector
      node={{ ...instrument(id), kind: "plugin", parameters: { format: "vst2", path: `${id}.dll` } }}
      backend={backend}
      sessionId={sessionId}
      disabled={false}
      onUnload={() => {}}
      snapshot={null}
    />
  );
  const result = render(view("a"));
  await act(async () => {
    fireEvent.click(result.getByRole("button", { name: /Load programs/i }));
  });
  expect(result.getByRole("option", { name: "Only A" })).toBeTruthy();
  result.rerender(view("b"));
  expect(result.queryByRole("option", { name: "Only A" })).toBeNull();
  fireEvent.click(result.getByRole("button", { name: /Load programs/i }));
  result.rerender(view("b", "other-session"));
  await act(async () => {
    late.resolve({ programs: [{ id: 3, name: "Late B" }], units: [] });
  });
  expect(result.queryByRole("option", { name: "Late B" })).toBeNull();
});

for (const failure of ["inventory", "stop", "import", "unmount", "automatic-stop"] as const) {
  it(`stops Sound Check independently of inventory and retains recovery after ${failure}`, async () => {
    vi.useFakeTimers();
    vi.spyOn(Date, "now").mockReturnValue(1_000_000);
    const stop = vi.fn().mockResolvedValue({ state: "stopped" });
    if (failure === "stop" || failure === "automatic-stop") stop.mockRejectedValueOnce(new Error("stop unavailable"));
    const importTake = vi.fn().mockResolvedValue({ mediaId: "take", fileName: "take.wav", durationMs: 10000 });
    if (failure === "import") importTake.mockRejectedValueOnce(new Error("import unavailable"));
    const backend = {
      ...createDisconnectedBackend(),
      connected: true,
      createRecorder: vi.fn().mockResolvedValue({}),
      armRecorder: vi.fn().mockResolvedValue({}),
      startRecorder: vi.fn().mockResolvedValue({}),
      listRecorders: vi.fn().mockRejectedValue(new Error("inventory unavailable")),
      stopNodeRecording: stop,
      listRecordings: vi
        .fn()
        .mockResolvedValue([
          { id: "row", recorderId: `audio-file-take-${(1_000_000).toString(36)}`, state: "completed" },
        ]),
      importTemporaryRecording: importTake,
    } as UiBackend;
    const recorder: Node = {
      ...instrument("r"),
      kind: "recorder",
      ports: [{ name: "in", direction: "input", channels: 1 }],
    };
    const result = render(
      <AudioFileNodeEditor
        node={{ ...instrument("a"), kind: "audioFile" }}
        backend={backend}
        disabled={false}
        transportDisabled={false}
        sessionRunning
        state="stopped"
        sessionId="s"
        currentSessionIdRef={{ current: "s" }}
        recorderNodes={[recorder]}
        recorderStatuses={[]}
        onChange={() => {}}
        onTransport={() => {}}
      />,
    );
    await act(async () => {
      fireEvent.click(result.getByRole("button", { name: "Record Sound Check" }));
    });
    if (failure === "unmount" || failure === "automatic-stop") result.unmount();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10000);
    });
    expect(stop).toHaveBeenCalledWith("s", "r", expect.any(String));
    expect(backend.listRecorders).not.toHaveBeenCalled();
    if (failure === "automatic-stop") {
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1000);
      });
      expect(stop).toHaveBeenCalledTimes(2);
      expect(importTake).toHaveBeenCalledOnce();
    }
    if (failure === "stop" || failure === "import") {
      expect(result.queryByRole("button", { name: "Record Sound Check" })).toBeNull();
      await act(async () => {
        fireEvent.click(result.getByRole("button", { name: "Stop and prepare replay" }));
      });
      expect(result.getByRole("button", { name: "Record Sound Check" })).toBeTruthy();
      expect(stop).toHaveBeenCalledTimes(failure === "stop" ? 2 : 1);
    }
  });
}

it("does not repopulate Soundboard state after stop or a newer control action", async () => {
  vi.useFakeTimers();
  const pending = deferred<{ slots: []; ignoredTriggers: number }>();
  const backend = {
    connected: true,
    soundboardStatus: vi.fn(() => pending.promise),
    soundboardStop: vi.fn().mockResolvedValue({}),
  } as unknown as UiBackend;
  const session = { ...demoSession, nodes: [{ ...instrument("board"), kind: "soundboard" as const }] };
  const result = renderHook(
    ({ running }) =>
      useSoundboardController({
        backend,
        session,
        sessionRunning: running,
        startSession: async () => true,
        report: () => {},
      }),
    { initialProps: { running: true } },
  );
  await act(async () => {
    await vi.advanceTimersByTimeAsync(150);
  });
  await act(async () => {
    result.result.current.stop("board");
  });
  await act(async () => {
    pending.resolve({ slots: [], ignoredTriggers: 42 });
  });
  expect(result.result.current.store.get("board")).toBeNull();
  const late = deferred<{ slots: []; ignoredTriggers: number }>();
  vi.mocked(backend.soundboardStatus!).mockReturnValue(late.promise as never);
  await act(async () => {
    await vi.advanceTimersByTimeAsync(150);
  });
  result.rerender({ running: false });
  await act(async () => {
    late.resolve({ slots: [], ignoredTriggers: 99 });
  });
  expect(result.result.current.store.get("board")).toBeNull();
});
