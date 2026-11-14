/** @vitest-environment jsdom */
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Node } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";
import { InstrumentNodeEditor, keyboardKeys, noteLabel } from "./InstrumentNodeEditor";
import { PluginEditorControls } from "./PluginPanels";

afterEach(cleanup);

const node = (midiInput?: string): Node => ({
  id: "instrument-1",
  kind: "instrument",
  typeVersion: 1,
  name: "Synth 1",
  enabled: true,
  bypass: false,
  parameters: midiInput === undefined ? {} : { midiInput },
  ports: [{ name: "out", direction: "output", channels: 2 }],
});

function setup(options: { running?: boolean; midiInput?: string; devices?: string[] } = {}) {
  const send = vi.fn().mockResolvedValue({ accepted: 1, scheduled: 0, heldNotes: 1 });
  const backend = {
    connected: true,
    midiDevices: vi.fn().mockResolvedValue({ devices: options.devices ?? ["Keystation 61"], instruments: [] }),
    midiSendEvents: send,
  } as unknown as UiBackend;
  const onChange = vi.fn();
  const view = render(
    <InstrumentNodeEditor
      node={node(options.midiInput)}
      backend={backend}
      sessionId="session-1"
      running={options.running ?? true}
      disabled={false}
      onChange={onChange}
    />,
  );
  return { view, send, onChange };
}

describe("Instrument keyboard", () => {
  it("labels notes and lays out two octaves plus the top C", () => {
    expect(noteLabel(60)).toBe("C4");
    expect(noteLabel(61)).toBe("C♯4");
    const keys = keyboardKeys();
    expect(keys.filter((key) => !key.black)).toHaveLength(15);
    expect(keys.filter((key) => key.black)).toHaveLength(10);
    expect(Math.max(...keys.map((key) => key.note))).toBe(84);
  });

  it("sends noteOn on press and the matching noteOff on release, with the exact request", async () => {
    const { view, send } = setup();
    const key = view.getByRole("button", { name: "C4" });
    fireEvent.pointerDown(key);
    fireEvent.pointerUp(key);
    await waitFor(() => expect(send).toHaveBeenCalledTimes(2));
    expect(send.mock.calls).toEqual([
      ["session-1", "instrument-1", [{ type: "noteOn", note: 60, velocity: 100 }]],
      ["session-1", "instrument-1", [{ type: "noteOff", note: 60 }]],
    ]);
  });

  it("silences held notes with Stop all notes and when the panel goes away", async () => {
    const { view, send } = setup();
    fireEvent.click(view.getByRole("button", { name: "Stop all notes" }));
    await waitFor(() => expect(send).toHaveBeenCalledTimes(1));
    expect(send.mock.calls[0][2]).toEqual([{ type: "allNotesOff" }]);
    fireEvent.pointerDown(view.getByRole("button", { name: "E4" }));
    view.unmount();
    await waitFor(() => expect(send).toHaveBeenCalledTimes(3));
    expect(send.mock.calls[2][2]).toEqual([{ type: "allNotesOff" }]);
  });

  it("disables the keys until the route plays", () => {
    const { view } = setup({ running: false });
    expect((view.getByRole("button", { name: "C4" }) as HTMLButtonElement).disabled).toBe(true);
    expect((view.getByRole("button", { name: "Stop all notes" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("offers the connected devices and keeps a saved one that is unplugged", async () => {
    const { view, onChange } = setup({ midiInput: "Old Piano" });
    const select = view.getByRole("combobox");
    await waitFor(() => expect(view.getByRole("option", { name: "Keystation 61" })).toBeTruthy());
    expect((select as HTMLSelectElement).value).toBe("Old Piano");
    expect(view.getByRole("option", { name: "Old Piano (not connected)" })).toBeTruthy();
    fireEvent.change(select, { target: { value: "Keystation 61" } });
    expect(onChange).toHaveBeenCalledWith("midiInput", "Keystation 61");
  });

  it("shows the on-screen keyboard choice when the node has no saved value", () => {
    const { view } = setup();
    expect((view.getByRole("combobox") as HTMLSelectElement).value).toBe("keyboard");
  });

  it("does not promise sliders in the window hint, since an Instrument has none", () => {
    const view = render(
      <PluginEditorControls
        node={node()}
        backend={{ connected: true } as unknown as UiBackend}
        sessionId="session-1"
        running={false}
        onStateSaved={() => undefined}
      />,
    );
    expect(view.container.textContent).toContain("play it with the keyboard below");
    expect(view.container.textContent).not.toContain("sliders");
  });
});
