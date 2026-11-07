/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ApiAutostartSetting, AutoplaySetting } from "./AutoplaySetting";

afterEach(cleanup);

it("distinguishes loading from a confirmed value and an unavailable setting", async () => {
  let answer: ((value: unknown) => void) | undefined;
  const invoke = vi.fn(
    () =>
      new Promise<unknown>((resolve) => {
        answer = resolve;
      }),
  );
  const view = render(<AutoplaySetting invoke={invoke} />);
  expect(screen.getByTestId("autoplay-state").textContent).toBe("Loading…");
  await waitFor(() => expect(answer).toBeDefined());
  answer?.(true);
  await waitFor(() => expect(screen.getByTestId("autoplay-state").textContent).toBe("On"));
  view.unmount();
  render(
    <AutoplaySetting
      invoke={vi.fn(async () => {
        throw new Error("unavailable");
      })}
    />,
  );
  await waitFor(() => expect(screen.getByTestId("autoplay-state").textContent).toBe("Unavailable"));
});

it("keeps the confirmed state during a save and reports the acknowledged value", async () => {
  let answer: ((value: unknown) => void) | undefined;
  const invoke = vi.fn((command: string) =>
    command === "autoplay_get"
      ? Promise.resolve(false)
      : new Promise<unknown>((resolve) => {
          answer = resolve;
        }),
  );
  render(<AutoplaySetting invoke={invoke} />);
  const box = screen.getByLabelText("Play the selected session automatically") as HTMLInputElement;
  await waitFor(() => expect(box.disabled).toBe(false));
  fireEvent.click(box);
  expect(screen.getByTestId("autoplay-state").textContent).toBe("Off");
  expect(box.disabled).toBe(true);
  expect(screen.getByRole("status").textContent).toContain("Saving");
  answer?.(false);
  await waitFor(() => expect(box.disabled).toBe(false));
  expect(screen.getByTestId("autoplay-state").textContent).toBe("Off");
  expect(screen.getByRole("status").textContent).toContain("start stopped");
});

it("reports a synchronous setting-read failure as unavailable", async () => {
  render(
    <AutoplaySetting
      invoke={() => {
        throw new Error("bridge unavailable");
      }}
    />,
  );
  await waitFor(() => expect(screen.getByTestId("autoplay-state").textContent).toBe("Unavailable"));
});

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
  const invoke = vi.fn(async (command: string) => {
    if (command === "autoplay_get") return false;
    throw new Error("disk full");
  });
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
    if (command === "api_autostart_set") {
      saved = args?.enabled === true;
      return saved;
    }
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
