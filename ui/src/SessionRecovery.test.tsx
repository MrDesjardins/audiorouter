/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { SessionTransferPanel } from "./AdvancedPanels";
import { createDisconnectedBackend } from "./backend";
import { demoSession } from "./fixtures";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function setup() {
  const exportRawSession = vi.fn().mockResolvedValue({ sessionId: "broken", document: "not json\n" });
  const deleteSession = vi.fn().mockResolvedValue({ sessionId: "broken", deleted: true });
  const onRecovered = vi.fn();
  render(
    <SessionTransferPanel
      backend={{ ...createDisconnectedBackend(), connected: true, exportRawSession, deleteSession }}
      session={demoSession}
      onImported={vi.fn()}
      onRecovered={onRecovered}
      quarantinedSessions={[{ sessionId: "broken", reason: "The saved data could not be read." }]}
    />,
  );
  return { exportRawSession, deleteSession, onRecovered };
}

it("exports the original text without parsing or changing it", async () => {
  const createObjectURL = vi.fn((blob: Blob) => {
    expect(blob).toBeInstanceOf(Blob);
    return "blob:recovery";
  });
  const revokeObjectURL = vi.fn();
  vi.stubGlobal("URL", { createObjectURL, revokeObjectURL });
  vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});
  const { exportRawSession, deleteSession } = setup();
  fireEvent.click(screen.getByRole("button", { name: "Export original file" }));
  await waitFor(() => expect(revokeObjectURL).toHaveBeenCalledWith("blob:recovery"));
  expect(exportRawSession).toHaveBeenCalledExactlyOnceWith("broken");
  const blob = createObjectURL.mock.calls[0]![0] as Blob;
  const text = await new Promise<string>((resolve) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.readAsText(blob);
  });
  expect(text).toBe("not json\n");
  expect(deleteSession).not.toHaveBeenCalled();
});

it("requires deliberate deletion and refreshes only after success", async () => {
  const { deleteSession, onRecovered } = setup();
  fireEvent.click(screen.getByRole("button", { name: "Delete unavailable session" }));
  expect(deleteSession).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Keep session" }));
  expect(deleteSession).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Delete unavailable session" }));
  fireEvent.click(screen.getByRole("button", { name: "Confirm deletion" }));
  await waitFor(() => expect(onRecovered).toHaveBeenCalledOnce());
  expect(deleteSession).toHaveBeenCalledExactlyOnceWith("broken", expect.any(String));
});
