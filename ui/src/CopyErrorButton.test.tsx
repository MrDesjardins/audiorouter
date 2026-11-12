// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { CopyErrorButton } from "./CopyErrorButton";
import { PanelMessage } from "./PanelMessage";

const originalClipboard = Object.getOwnPropertyDescriptor(navigator, "clipboard");
afterEach(() => {
  cleanup();
  if (originalClipboard) Object.defineProperty(navigator, "clipboard", originalClipboard);
  else Reflect.deleteProperty(navigator, "clipboard");
});

it("copies the full error, keeping the original message visible", async () => {
  const message = "Unable to save: worker not found. Checked C:\\AudioRouter\\resources.";
  const writeText = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
  render(<PanelMessage message={message} />);
  fireEvent.click(screen.getByRole("button", { name: "Copy error" }));
  await screen.findByRole("button", { name: "Copied" });
  expect(writeText).toHaveBeenCalledWith(message);
  expect(screen.getByText(message)).toBeTruthy();
});

it("offers selectable original text when clipboard access fails", async () => {
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: undefined });
  render(<CopyErrorButton message="Worker unavailable" />);
  fireEvent.click(screen.getByRole("button", { name: "Copy error" }));
  const field = await screen.findByRole("textbox", { name: "Error text for manual copying" });
  expect((field as HTMLTextAreaElement).value).toBe("Worker unavailable");
  fireEvent.focus(field);
  expect((field as HTMLTextAreaElement).selectionEnd).toBe("Worker unavailable".length);
});

it("preserves the error when Windows denies clipboard access", async () => {
  const writeText = vi.fn().mockRejectedValue(new Error("Clipboard access denied"));
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
  render(<PanelMessage message="Unable to save plugin settings" small />);
  fireEvent.click(screen.getByRole("button", { name: "Copy error" }));
  await screen.findByRole("textbox", { name: "Error text for manual copying" });
  expect(screen.getByText("Unable to save plugin settings", { selector: ".error-message-text" })).toBeTruthy();
  expect(screen.queryByText("Clipboard access denied")).toBeNull();
});

it("does not carry asynchronous copy feedback into a replacement error", async () => {
  let finish!: () => void;
  const writeText = vi.fn().mockReturnValue(
    new Promise<void>((resolve) => {
      finish = resolve;
    }),
  );
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
  const view = render(<CopyErrorButton message="First failure" />);
  fireEvent.click(screen.getByRole("button", { name: "Copy error" }));
  view.rerender(<CopyErrorButton message="Second failure" />);
  finish();
  await waitFor(() =>
    expect((screen.getByRole("button", { name: "Copy error" }) as HTMLButtonElement).disabled).toBe(false),
  );
  expect(screen.queryByText("Error copied.")).toBeNull();
});
