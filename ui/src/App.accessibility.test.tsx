/** @vitest-environment jsdom */

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { createDisconnectedBackend } from "./backend";
import { DraftConnectionList, insertMixerActionId, removeMixerActionId } from "./DraftConnectionList";
import { appendDraftConnection, insertDraftMixer } from "./draft";
import { demoSession } from "./fixtures";
import { BackendConnectionContext } from "./backendConnectionContext";
import { GraphList } from "./GraphList";
import type { ProcessorDescriptor } from "./processorCatalog";

function connectedPreviewBackend() {
  return { ...createDisconnectedBackend(), connected: true };
}

beforeAll(() => {
  Object.defineProperty(globalThis, "ResizeObserver", {
    configurable: true,
    value: class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  });
});

afterEach(() => cleanup());

describe("keyboard connection dialog", () => {
  it("opens with focus, wraps focus, retains validation errors, and restores focus", async () => {
    const backend = connectedPreviewBackend();
    render(<App backend={backend} />);

    const opener = screen.getByRole("button", { name: "Keyboard connection dialog" });
    opener.focus();
    fireEvent.click(opener);

    const dialog = await screen.findByRole("dialog", { name: "Keyboard connection" });
    const source = within(dialog).getByRole("combobox", { name: "Keyboard source output port" });
    expect(document.activeElement).toBe(source);

    const close = within(dialog).getByRole("button", { name: "Close keyboard connection dialog" });
    const cancel = within(dialog).getByRole("button", { name: "Cancel" });
    cancel.focus();
    fireEvent.keyDown(window, { key: "Tab" });
    expect(document.activeElement).toBe(close);
    close.focus();
    fireEvent.keyDown(window, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(cancel);

    const add = within(dialog).getByRole("button", { name: "Add connection to draft" });
    fireEvent.click(add);
    expect(screen.getByRole("dialog", { name: "Keyboard connection" })).toBeTruthy();
    expect(screen.getByText("Choose an output and input port first.")).toBeTruthy();

    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Keyboard connection" })).toBeNull());
    await waitFor(() => expect(document.activeElement).toBe(opener));
  });

  it("closes after a valid draft connection", async () => {
    const backend = connectedPreviewBackend();
    render(<App backend={backend} />);

    fireEvent.click(screen.getByRole("button", { name: "Keyboard connection dialog" }));
    const dialog = await screen.findByRole("dialog", { name: "Keyboard connection" });
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Keyboard source output port" }), { target: { value: "mic::out" } });
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Keyboard destination input port" }), { target: { value: "voice::in" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Add connection to draft" }));

    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Keyboard connection" })).toBeNull());
    expect(screen.getByText("Connection added to the draft. Review and plan the changes before committing.")).toBeTruthy();
  });

  it("renders named canvas handles for connected graph editing", async () => {
    render(<App backend={connectedPreviewBackend()} />);

    expect(await screen.findByLabelText("Microphone out output")).toBeTruthy();
    expect(await screen.findByLabelText("Voice gain in input")).toBeTruthy();
  });

  it("offers bounded slider and precise entry for numeric processor parameters", async () => {
    const processor: ProcessorDescriptor = {
      id: "gain",
      version: 1,
      category: "effect",
      availability: { status: "available" },
      latencySamples: 0,
      parameters: [{ name: "gainDb", type: "number", unit: "dB", minimum: -60, maximum: 24, default: 0 }],
    };
    const backend = { ...createDisconnectedBackend(), connected: true, listProcessors: async () => [processor] };
    render(<App backend={backend} />);
    fireEvent.click(await screen.findByLabelText("Voice gain, gain"));

    const slider = await screen.findByRole("slider", { name: "gainDb slider" });
    const precise = screen.getByRole("spinbutton", { name: "gainDb precise value" });
    expect(slider.getAttribute("min")).toBe("-60");
    expect(slider.getAttribute("max")).toBe("24");
    expect((precise as HTMLInputElement).value).toBe("0");
    fireEvent.change(slider, { target: { value: "-6" } });
    expect((precise as HTMLInputElement).value).toBe("-6");
  });

  it("persists tidy layout positions as presentation state", async () => {
    render(<App backend={connectedPreviewBackend()} />);

    fireEvent.click(await screen.findByRole("button", { name: "Tidy layout" }));
    expect(JSON.parse(window.localStorage.getItem("audiorouter.ui.layout.demo-session") ?? "null")).toMatchObject({
      mic: { x: 0, y: 0 },
      voice: { x: 260, y: 0 },
    });
  });

  it("exposes previewable mixer topology actions from the draft connection list", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const inserted = insertDraftMixer(connected, "edge-1");
    const onRemove = vi.fn();
    const onToggle = vi.fn();
    render(<BackendConnectionContext.Provider value={true}><DraftConnectionList session={inserted} onRemove={onRemove} onToggle={onToggle} /></BackendConnectionContext.Provider>);

    fireEvent.click(screen.getAllByRole("button", { name: /Insert mixer on/ })[0]);
    expect(onRemove).toHaveBeenCalledWith(insertMixerActionId("edge-1"));
    fireEvent.click(screen.getByRole("button", { name: "Remove and reconnect Mixer 1" }));
    expect(onRemove).toHaveBeenCalledWith(removeMixerActionId("mixer-1"));
  });

  it("executes mixer topology previews through the connected App draft boundary", async () => {
    render(<App backend={connectedPreviewBackend()} />);
    fireEvent.click(screen.getByRole("button", { name: "Keyboard connection dialog" }));
    const dialog = await screen.findByRole("dialog", { name: "Keyboard connection" });
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Keyboard source output port" }), { target: { value: "mic::out" } });
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Keyboard destination input port" }), { target: { value: "voice::in" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Add connection to draft" }));

    fireEvent.click(screen.getAllByRole("button", { name: /Insert mixer on/ })[0]);
    expect(screen.getByText("Mixer inserted into the draft. Review and plan the changes before committing.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Remove and reconnect Mixer 1" }));
    expect(screen.getByText("Mixer removed and its single path reconnected in the draft. Review and plan the changes before committing.")).toBeTruthy();
  });

  it("disables topology mutations when no backend connection context exists", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const inserted = insertDraftMixer(connected, "edge-1");
    render(<DraftConnectionList session={inserted} onRemove={vi.fn()} onToggle={vi.fn()} />);

    expect(screen.getAllByRole("button", { name: /Insert mixer on/ })[0]).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "Remove and reconnect Mixer 1" })).toHaveProperty("disabled", true);
  });

  it("keeps topology actions available in the structured list view", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const onRemove = vi.fn();
    const onToggle = vi.fn();
    render(<BackendConnectionContext.Provider value={true}><GraphList session={connected} selectedNodeId="mic" onSelect={vi.fn()} onRemoveConnection={onRemove} onToggleConnection={onToggle} /></BackendConnectionContext.Provider>);

    fireEvent.click(screen.getByRole("button", { name: "Insert mixer on Microphone to Voice gain" }));
    expect(onRemove).toHaveBeenCalledWith(insertMixerActionId("edge-1"));
  });
});
