/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { NodeIdentity, SessionIdentity } from "./NodeIdentity";

const originalClipboard = Object.getOwnPropertyDescriptor(navigator, "clipboard");
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  if (originalClipboard) Object.defineProperty(navigator, "clipboard", originalClipboard);
  else Reflect.deleteProperty(navigator, "clipboard");
});
function clipboard(value: unknown) {
  Object.defineProperty(navigator, "clipboard", { configurable: true, value });
}

it("copies the exact node ID and briefly announces success", async () => {
  vi.useFakeTimers();
  const copy = vi.fn().mockResolvedValue(undefined);
  clipboard({ writeText: copy });
  render(<NodeIdentity nodeId="siege-eq" />);
  expect(screen.getByLabelText("Node ID").textContent).toBe("siege-eq");
  expect(screen.getByRole("button", { name: "Copy node ID" }).title).toBe("Copy node ID for API integrations");
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Copy node ID" })));
  expect(copy).toHaveBeenCalledWith("siege-eq");
  expect(screen.getByRole("status").textContent).toBe("Copied.");
  act(() => vi.advanceTimersByTime(2000));
  expect(screen.getByRole("status").textContent).toBe("");
});
it("uses the same copy behavior for the selected session", async () => {
  const copy = vi.fn().mockResolvedValue(undefined);
  clipboard({ writeText: copy });
  const view = render(<SessionIdentity sessionId="gaming-session" />);
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Copy session ID" })));
  expect(copy).toHaveBeenCalledWith("gaming-session");
  view.rerender(<SessionIdentity sessionId="other-session" />);
  expect(screen.getByLabelText("Session ID").textContent).toBe("other-session");
  expect(screen.getByRole("status").textContent).toBe("");
});
it("keeps the full ID selectable and gives manual-copy guidance when clipboard is unavailable", async () => {
  clipboard(undefined);
  const id = "node-" + "a".repeat(250);
  render(<NodeIdentity nodeId={id} />);
  expect(screen.getByLabelText("Node ID").textContent).toBe(id);
  expect(screen.getByLabelText("Node ID").tabIndex).toBe(0);
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Copy node ID" })));
  expect(screen.getByRole("status").textContent).toContain("copy it manually");
});
it("does not transfer late copy feedback to a newly selected node", async () => {
  let resolve!: () => void;
  clipboard({
    writeText: () =>
      new Promise<void>((r) => {
        resolve = r;
      }),
  });
  const view = render(<NodeIdentity nodeId="first" />);
  fireEvent.click(screen.getByRole("button", { name: "Copy node ID" }));
  view.rerender(<NodeIdentity nodeId="second" />);
  await act(async () => resolve());
  expect(screen.getByRole("status").textContent).toBe("");
});
it("handles denied clipboard access without exposing internal errors", async () => {
  clipboard({ writeText: vi.fn().mockRejectedValue(new Error("private clipboard detail")) });
  render(<SessionIdentity sessionId="session" />);
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Copy session ID" })));
  expect(screen.getByRole("status").textContent).toContain("copy it manually");
  expect(screen.getByRole("status").textContent).not.toContain("private");
});
