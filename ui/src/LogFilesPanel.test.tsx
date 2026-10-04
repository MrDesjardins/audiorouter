/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { LogFilesPanel } from "./LogFilesPanel";

afterEach(() => { cleanup(); delete window.__TAURI_INTERNALS__; vi.restoreAllMocks(); });

function desktop(failOpen = false) {
  const invoke = vi.fn(async (command: string) => {
    if (command === "log_folder_path") return "C:/Users/example/AppData/Local/AudioRouter/logs";
    if (command === "open_logs_folder" && failOpen) throw new Error("private native error");
    return null;
  });
  window.__TAURI_INTERNALS__ = { invoke };
  return invoke;
}

it("opens the fixed logs folder and copies its path", async () => {
  const invoke = desktop();
  const copy = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: copy } });
  render(<LogFilesPanel />);
  await screen.findByDisplayValue("C:/Users/example/AppData/Local/AudioRouter/logs");
  fireEvent.click(screen.getByRole("button", { name: "Open logs folder" }));
  await screen.findByText(/Logs folder opened/);
  expect(invoke).toHaveBeenCalledWith("open_logs_folder");
  fireEvent.click(screen.getByRole("button", { name: "Copy folder path" }));
  await screen.findByText("Folder path copied.");
  expect(copy).toHaveBeenCalledWith("C:/Users/example/AppData/Local/AudioRouter/logs");
  expect(screen.getByText(/attach shell.jsonl/).textContent).toContain("discovery.jsonl");
  expect(screen.getByText(/attach shell.jsonl/).textContent).toContain("network.jsonl");
});

it("gives a useful fallback when Explorer cannot open", async () => {
  desktop(true); render(<LogFilesPanel />);
  fireEvent.click(screen.getByRole("button", { name: "Open logs folder" }));
  await screen.findByText(/Could not open the logs folder/);
  expect(screen.queryByText(/private native error/)).toBeNull();
  await waitFor(() => expect((screen.getByRole("button", { name: "Open logs folder" }) as HTMLButtonElement).disabled).toBe(false));
});

it("handles clipboard failure and browser previews", async () => {
  desktop();
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: vi.fn().mockRejectedValue(new Error("denied")) } });
  const view = render(<LogFilesPanel />);
  await screen.findByDisplayValue("C:/Users/example/AppData/Local/AudioRouter/logs");
  fireEvent.click(screen.getByRole("button", { name: "Copy folder path" }));
  await screen.findByText(/copy it manually/);
  view.unmount(); delete window.__TAURI_INTERNALS__;
  render(<LogFilesPanel />);
  expect((screen.getByRole("button", { name: "Open logs folder" }) as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByText(/installed AudioRouter app/)).toBeTruthy();
});
