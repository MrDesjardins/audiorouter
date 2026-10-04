/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AutoplaySetting } from "./AutoplaySetting";

afterEach(cleanup);

it("shows the shell's saved autoplay value and saves a change", async () => {
  let saved = true;
  const invoke = vi.fn(async (command: string, args?: Record<string, unknown>) => {
    if (command === "autoplay_get") return saved;
    saved = args?.enabled === true;
    return saved;
  });
  render(<AutoplaySetting invoke={invoke} />);
  const box = screen.getByLabelText("Play the selected session automatically") as HTMLInputElement;
  await waitFor(() => expect(box.checked).toBe(true));
  fireEvent.click(box);
  await waitFor(() => expect(box.checked).toBe(false));
  expect(invoke).toHaveBeenCalledWith("autoplay_set", { enabled: false });
  expect(screen.getByRole("status").textContent).toContain("start stopped");
});

it("keeps the saved value when saving fails, and is unavailable outside the desktop app", async () => {
  const invoke = vi.fn(async (command: string) => { if (command === "autoplay_get") return false; throw new Error("disk full"); });
  const view = render(<AutoplaySetting invoke={invoke} />);
  const box = screen.getByLabelText("Play the selected session automatically") as HTMLInputElement;
  await waitFor(() => expect(box.disabled).toBe(false));
  fireEvent.click(box);
  expect(await screen.findByText("Autoplay could not be saved. Try again.")).toBeTruthy();
  expect(box.checked).toBe(false);
  view.unmount();
  render(<AutoplaySetting invoke={undefined} />);
  expect((screen.getByLabelText("Play the selected session automatically") as HTMLInputElement).disabled).toBe(true);
  expect(screen.getByText("Available in the AudioRouter desktop app.")).toBeTruthy();
});
