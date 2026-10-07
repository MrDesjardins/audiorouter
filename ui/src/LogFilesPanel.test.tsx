/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { LogFilesPanel } from "./LogFilesPanel";

afterEach(() => {
  cleanup();
  delete window.__TAURI_INTERNALS__;
  vi.restoreAllMocks();
});

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
  desktop(true);
  render(<LogFilesPanel />);
  fireEvent.click(screen.getByRole("button", { name: "Open logs folder" }));
  await screen.findByText(/Could not open the logs folder/);
  expect(screen.queryByText(/private native error/)).toBeNull();
  await waitFor(() =>
    expect((screen.getByRole("button", { name: "Open logs folder" }) as HTMLButtonElement).disabled).toBe(false),
  );
});

it("saves a support bundle with the client diagnostics through the shell", async () => {
  const invoke = vi.fn(async (command: string) => {
    if (command === "log_folder_path") return "C:/Users/example/AppData/Local/AudioRouter/logs";
    if (command === "export_support_bundle") return "audiorouter-support-1234.zip";
    return null;
  });
  window.__TAURI_INTERNALS__ = { invoke };
  render(<LogFilesPanel clientDiagnostics={["10:00 RPC failed: sessions.play (permissionDenied) [req K7Q2M9XD]"]} />);
  fireEvent.click(screen.getByRole("button", { name: "Copy support bundle" }));
  await screen.findByText(/Saved audiorouter-support-1234.zip in the logs folder/);
  expect(invoke).toHaveBeenCalledWith("export_support_bundle", {
    clientDiagnostics: ["10:00 RPC failed: sessions.play (permissionDenied) [req K7Q2M9XD]"],
  });
  invoke.mockImplementation(async (command: string) => {
    if (command === "export_support_bundle") throw "C:\\private\\path failure";
    return null;
  });
  fireEvent.click(screen.getByRole("button", { name: "Copy support bundle" }));
  await screen.findByText(/Could not create the support bundle/);
  expect(screen.queryByText(/private/)).toBeNull();
});

it("switches verbose logging and counts down the hour in a fixed-size status", async () => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  try {
    desktop();
    const get = vi
      .fn()
      .mockResolvedValue({ enabled: false, expiresAtUnixMs: null, remainingSeconds: 0, maxSeconds: 3600 });
    const set = vi.fn(async (enabled: boolean) =>
      enabled
        ? { enabled: true, expiresAtUnixMs: Date.now() + 3_600_000, remainingSeconds: 3600, maxSeconds: 3600 }
        : { enabled: false, expiresAtUnixMs: null, remainingSeconds: 0, maxSeconds: 3600 },
    );
    render(<LogFilesPanel verbose={{ get, set }} />);
    const status = screen.getByRole("status", { name: "Verbose logging status" });
    await waitFor(() => expect(status.textContent).toBe("Off"));
    const toggle = screen.getByRole("checkbox", { name: "Verbose logging" }) as HTMLInputElement;
    fireEvent.click(toggle);
    await waitFor(() => expect(status.textContent).toBe("On · 60:00 left"));
    expect(set).toHaveBeenCalledWith(true);
    expect(toggle.checked).toBe(true);
    await vi.advanceTimersByTimeAsync(61_000);
    expect(status.textContent).toMatch(/^On · (58:59|59:00) left$/);
    // The same element stays in place; only its text changes.
    expect(screen.getByRole("status", { name: "Verbose logging status" })).toBe(status);
    await vi.advanceTimersByTimeAsync(3_600_000);
    await waitFor(() => expect(status.textContent).toBe("Off"));
    expect(toggle.checked).toBe(false);
  } finally {
    vi.useRealTimers();
  }
});

it("shows verbose logging as unavailable without a backend", () => {
  render(<LogFilesPanel />);
  expect(screen.getByRole("status", { name: "Verbose logging status" }).textContent).toBe("Unavailable");
  expect((screen.getByRole("checkbox", { name: "Verbose logging" }) as HTMLInputElement).disabled).toBe(true);
});

it("handles clipboard failure and browser previews", async () => {
  desktop();
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText: vi.fn().mockRejectedValue(new Error("denied")) },
  });
  const view = render(<LogFilesPanel />);
  await screen.findByDisplayValue("C:/Users/example/AppData/Local/AudioRouter/logs");
  fireEvent.click(screen.getByRole("button", { name: "Copy folder path" }));
  await screen.findByText(/copy it manually/);
  view.unmount();
  delete window.__TAURI_INTERNALS__;
  render(<LogFilesPanel />);
  expect((screen.getByRole("button", { name: "Open logs folder" }) as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByText(/installed AudioRouter app/)).toBeTruthy();
});
