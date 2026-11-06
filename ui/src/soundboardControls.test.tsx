/** @vitest-environment jsdom */
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Node } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";
import { demoSession } from "./fixtures";
import { SoundboardNodeControls } from "./SoundboardNodeControls";
import { SoundboardNodeEditor, SoundPlayButton } from "./SoundboardNodeEditor";
import { SoundboardContext, createSoundboardStore, useSoundboardController } from "./soundboardLive";

afterEach(cleanup);

describe("Soundboard Play entry points", () => {
  it("releases a held sound when its control is removed", () => {
    const trigger = vi.fn();
    const view = render(
      <SoundboardContext.Provider
        value={{
          store: createSoundboardStore(),
          ready: true,
          trigger,
          stop: vi.fn(),
          importing: false,
          importFile: vi.fn(),
          announce: vi.fn(),
          assign: vi.fn(),
        }}
      >
        <SoundPlayButton nodeId="board" slot={1} label="Hold sound" mode="hold">
          Hold
        </SoundPlayButton>
      </SoundboardContext.Provider>,
    );
    fireEvent.keyDown(view.getByRole("button", { name: "Hold sound" }), { key: "Enter" });
    view.unmount();
    expect(trigger.mock.calls).toEqual([
      ["board", 1, "press"],
      ["board", 1, "release"],
    ]);
  });

  it("orders release after a press still awaiting backend acknowledgement", async () => {
    let finishPress!: (value: { accepted: boolean }) => void;
    const pressing = new Promise<{ accepted: boolean }>((resolve) => {
      finishPress = resolve;
    });
    const send = vi.fn((_session: string, _node: string, _slot: number, action: string) =>
      action === "press" ? pressing : Promise.resolve({ accepted: true }),
    );
    const backend = { connected: true, soundboardTrigger: send } as unknown as UiBackend;
    function Harness() {
      const controller = useSoundboardController({
        backend,
        session: demoSession,
        sessionRunning: true,
        startSession: vi.fn(),
        report: vi.fn(),
      });
      return (
        <SoundboardContext.Provider value={controller}>
          <SoundPlayButton nodeId="board" slot={1} label="Hold sound" mode="hold">
            Hold
          </SoundPlayButton>
        </SoundboardContext.Provider>
      );
    }
    const view = render(<Harness />);
    const button = view.getByRole("button", { name: "Hold sound" });
    fireEvent.keyDown(button, { key: " " });
    fireEvent.keyUp(button, { key: " " });
    expect(send.mock.calls).toEqual([[demoSession.id, "board", 1, "press"]]);
    await act(async () => finishPress({ accepted: true }));
    expect(send.mock.calls).toEqual([
      [demoSession.id, "board", 1, "press"],
      [demoSession.id, "board", 1, "release"],
    ]);
  });

  it("releasing a held sound during route startup cancels the pending press", async () => {
    let finishStart!: (result: boolean) => void;
    const starting = new Promise<boolean>((resolve) => {
      finishStart = resolve;
    });
    const send = vi.fn().mockResolvedValue({ accepted: true });
    const backend = { connected: true, soundboardTrigger: send } as unknown as UiBackend;
    function Harness() {
      const controller = useSoundboardController({
        backend,
        session: demoSession,
        sessionRunning: false,
        startSession: () => starting,
        report: vi.fn(),
      });
      return (
        <SoundboardContext.Provider value={controller}>
          <SoundPlayButton nodeId="board" slot={1} label="Hold sound" mode="hold">
            Hold
          </SoundPlayButton>
        </SoundboardContext.Provider>
      );
    }
    const view = render(<Harness />);
    const button = view.getByRole("button", { name: "Hold sound" });
    fireEvent.keyDown(button, { key: " " });
    fireEvent.keyUp(button, { key: " " });
    expect(send).not.toHaveBeenCalled();
    await act(async () => finishStart(true));
    expect(send).not.toHaveBeenCalled();
  });

  for (const key of [" ", "Enter"]) {
    it(`hold playback supports ${key === " " ? "Space" : key} without repeating or sticking on focus loss`, () => {
      const trigger = vi.fn();
      const view = render(
        <SoundboardContext.Provider
          value={{
            store: createSoundboardStore(),
            ready: true,
            trigger,
            stop: vi.fn(),
            importing: false,
            importFile: vi.fn(),
            announce: vi.fn(),
            assign: vi.fn(),
          }}
        >
          <SoundPlayButton nodeId="board" slot={1} label="Hold sound" mode="hold">
            Hold
          </SoundPlayButton>
        </SoundboardContext.Provider>,
      );
      const button = view.getByRole("button", { name: "Hold sound" });
      fireEvent.keyDown(button, { key });
      fireEvent.keyDown(button, { key, repeat: true });
      fireEvent.keyUp(button, { key });
      expect(trigger.mock.calls).toEqual([
        ["board", 1, "press"],
        ["board", 1, "release"],
      ]);
      trigger.mockClear();
      fireEvent.keyDown(button, { key });
      fireEvent.blur(button);
      fireEvent.keyUp(button, { key });
      expect(trigger.mock.calls).toEqual([
        ["board", 1, "press"],
        ["board", 1, "release"],
      ]);
    });
  }

  it("hold playback ignores secondary buttons and releases the primary press", () => {
    const trigger = vi.fn();
    const view = render(
      <SoundboardContext.Provider
        value={{
          store: createSoundboardStore(),
          ready: true,
          trigger,
          stop: vi.fn(),
          importing: false,
          importFile: vi.fn(),
          announce: vi.fn(),
          assign: vi.fn(),
        }}
      >
        <SoundPlayButton nodeId="board" slot={1} label="Hold sound" mode="hold">
          Hold
        </SoundPlayButton>
      </SoundboardContext.Provider>,
    );
    const button = view.getByRole("button", { name: "Hold sound" });
    for (const other of [1, 2]) {
      fireEvent.pointerDown(button, { button: other });
      fireEvent.pointerUp(button, { button: other });
    }
    expect(trigger).not.toHaveBeenCalled();
    fireEvent.pointerDown(button, { button: 0 });
    fireEvent.pointerUp(button, { button: 0 });
    expect(trigger.mock.calls).toEqual([
      ["board", 1, "press"],
      ["board", 1, "release"],
    ]);
  });

  for (const entry of ["canvas", "properties"] as const) {
    for (const running of [false, true]) {
      it(`${entry} sends the saved board and slot after starting when running=${running}`, async () => {
        const node: Node = {
          id: "board",
          kind: "soundboard",
          typeVersion: 1,
          name: "Soundboard 1",
          enabled: true,
          bypass: false,
          parameters: { slot1MediaId: "lib:burp-01", slot1FileName: "Burp 01" },
          ports: [{ name: "out", direction: "output", channels: 2 }],
        };
        const send = vi.fn().mockResolvedValue({ accepted: true });
        const start = vi.fn().mockResolvedValue(true);
        const report = vi.fn();
        const backend = { connected: true, soundboardTrigger: send } as unknown as UiBackend;
        function Harness() {
          const controller = useSoundboardController({
            backend,
            session: { ...demoSession, nodes: [node] },
            sessionRunning: running,
            startSession: start,
            report,
          });
          return (
            <SoundboardContext.Provider value={controller}>
              <section aria-label="canvas">
                <SoundboardNodeControls node={node} />
              </section>
              <section aria-label="properties">
                <SoundboardNodeEditor
                  node={node}
                  backend={backend}
                  disabled={false}
                  onChange={() => {}}
                  onRemove={() => {}}
                />
              </section>
            </SoundboardContext.Provider>
          );
        }
        const view = render(<Harness />);
        fireEvent.click(
          within(view.getByRole("region", { name: entry })).getByRole("button", { name: "Play Burp 01" }),
        );
        await waitFor(() => expect(send).toHaveBeenCalledExactlyOnceWith(demoSession.id, "board", 1, "press"));
        expect(start).toHaveBeenCalledTimes(running ? 0 : 1);
        if (!running) expect(start.mock.invocationCallOrder[0]).toBeLessThan(send.mock.invocationCallOrder[0]);
        expect(report).not.toHaveBeenCalled();
      });
    }
  }
});
