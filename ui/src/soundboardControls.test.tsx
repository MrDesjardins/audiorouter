/** @vitest-environment jsdom */
import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Node } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";
import { demoSession } from "./fixtures";
import { SoundboardNodeControls } from "./SoundboardNodeControls";
import { SoundboardNodeEditor } from "./SoundboardNodeEditor";
import { SoundboardContext, useSoundboardController } from "./soundboardLive";

afterEach(cleanup);

describe("Soundboard Play entry points", () => {
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
