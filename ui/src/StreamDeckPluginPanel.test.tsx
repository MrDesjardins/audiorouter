/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { StreamDeckPluginPanel } from "./StreamDeckPluginPanel";

afterEach(() => { cleanup(); delete window.__TAURI_INTERNALS__; vi.restoreAllMocks(); });

it("hands the bundled plugin to Stream Deck", async () => {
  const invoke = vi.fn(async () => null);
  window.__TAURI_INTERNALS__ = { invoke };
  render(<StreamDeckPluginPanel />);
  fireEvent.click(screen.getByRole("button", { name: "Install Stream Deck plugin" }));
  await screen.findByText(/Confirm the install there/);
  expect(invoke).toHaveBeenCalledWith("install_streamdeck_plugin");
});

it("shows the shell's guidance when Stream Deck is missing", async () => {
  window.__TAURI_INTERNALS__ = { invoke: vi.fn(async () => { throw "Install the Stream Deck app (version 7.1 or newer) first, then try again."; }) };
  render(<StreamDeckPluginPanel />);
  fireEvent.click(screen.getByRole("button", { name: "Install Stream Deck plugin" }));
  await screen.findByText(/Install the Stream Deck app/);
  await waitFor(() => expect((screen.getByRole("button", { name: "Install Stream Deck plugin" }) as HTMLButtonElement).disabled).toBe(false));
});

it("explains where the action works outside the desktop app", () => {
  render(<StreamDeckPluginPanel />);
  expect((screen.getByRole("button", { name: "Install Stream Deck plugin" }) as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByText(/Open the installed AudioRouter app/)).toBeTruthy();
});
