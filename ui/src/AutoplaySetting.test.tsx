/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ApiAutostartSetting, AutoplaySetting } from "./AutoplaySetting";

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

it("shows and saves API auto-start through its own shell command", async () => {
  let saved = false;
  const invoke = vi.fn(async (command: string, args?: Record<string, unknown>) => {
    if (command === "api_autostart_get") return saved;
    if (command === "api_autostart_set") { saved = args?.enabled === true; return saved; }
    throw new Error(`unexpected ${command}`);
  });
  render(<ApiAutostartSetting invoke={invoke} />);
  const box = screen.getByLabelText("Start the local API automatically") as HTMLInputElement;
  await waitFor(() => expect(box.disabled).toBe(false));
  expect(box.checked).toBe(false);
  fireEvent.click(box);
  await waitFor(() => expect(box.checked).toBe(true));
  expect(invoke).toHaveBeenCalledWith("api_autostart_set", { enabled: true });
  expect(screen.getByRole("status").textContent).toContain("will start with AudioRouter");
});
