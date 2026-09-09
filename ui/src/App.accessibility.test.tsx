/** @vitest-environment jsdom */

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { createDisconnectedBackend } from "./backend";
import { DraftConnectionList, insertMixerActionId, removeMixerActionId } from "./DraftConnectionList";
import { appendDraftConnection, insertDraftMixer } from "./draft";
import { demoSession } from "./fixtures";

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
    render(<DraftConnectionList session={inserted} onRemove={onRemove} onToggle={onToggle} />);

    fireEvent.click(screen.getAllByRole("button", { name: "Insert mixer" })[0]);
    expect(onRemove).toHaveBeenCalledWith(insertMixerActionId("edge-1"));
    fireEvent.click(screen.getByRole("button", { name: "Remove and reconnect" }));
    expect(onRemove).toHaveBeenCalledWith(removeMixerActionId("mixer-1"));
  });
});
