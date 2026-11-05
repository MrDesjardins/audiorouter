/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { SessionFilePanel } from "./SessionFilePanel";
import { createDisconnectedBackend } from "./backend";
import { demoSession } from "./fixtures";

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
});

function setup() {
  const exportSessionFile = vi.fn().mockResolvedValue({ path: "C:\\backup.audiorouter" });
  const backend = { ...createDisconnectedBackend(), connected: true, exportSessionFile };
  render(<SessionFilePanel backend={backend} session={demoSession} unsaved onImported={vi.fn()} />);
  fireEvent.change(screen.getByRole("textbox"), { target: { value: "C:\\backup.audiorouter" } });
  return exportSessionFile;
}

it("explicitly exports the saved version without committing the draft", async () => {
  const exportSessionFile = setup();
  expect(screen.getByRole("note").textContent).toContain("Draft changes await Save");
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Export saved version…" })));
  expect(exportSessionFile).toHaveBeenCalledExactlyOnceWith(demoSession.id, "C:\\backup.audiorouter", false);
  expect(screen.getByRole("status").textContent).toContain("Draft changes are not included");
});

it("replaces prior success with the actual export failure", async () => {
  const exportSessionFile = setup();
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Export saved version…" })));
  exportSessionFile.mockRejectedValueOnce(new Error("Disk full"));
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Export saved version…" })));
  expect(screen.getByRole("status").textContent).toContain("Disk full");
  expect(screen.getByRole("status").textContent).not.toContain("Exported");
});

it("clears old success when the next file dialog is cancelled", async () => {
  const invoke = vi.fn().mockResolvedValueOnce("C:\\backup.audiorouter").mockResolvedValueOnce(null);
  Object.assign(window, { __TAURI_INTERNALS__: { invoke } });
  const exportSessionFile = vi.fn().mockResolvedValue({ path: "C:\\backup.audiorouter" });
  render(
    <SessionFilePanel
      backend={{ ...createDisconnectedBackend(), connected: true, exportSessionFile }}
      session={demoSession}
      unsaved
      onImported={vi.fn()}
    />,
  );
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Export saved version…" })));
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Export saved version…" })));
  expect(exportSessionFile).toHaveBeenCalledTimes(1);
  expect(screen.queryByRole("status")).toBeNull();
});
