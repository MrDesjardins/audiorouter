/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { QuitButton } from "./QuitButton";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
});
const shell = (invoke: (command: string) => Promise<unknown>) =>
  Object.assign(window, { __TAURI_INTERNALS__: { invoke } });

it("is hidden outside the desktop shell", () => {
  render(<QuitButton onMessage={() => {}} />);
  expect(screen.queryByRole("button")).toBeNull();
});

it("asks for a second click, then quits through the shell", async () => {
  const invoke = vi.fn().mockResolvedValue(undefined);
  shell(invoke);
  render(<QuitButton onMessage={() => {}} />);
  fireEvent.click(screen.getByRole("button", { name: "Quit AudioRouter" }));
  expect(invoke).not.toHaveBeenCalled();
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Confirm quit AudioRouter" })));
  expect(invoke).toHaveBeenCalledWith("quit_app", {});
});

it("disarms after five seconds and reports a refused quit", async () => {
  vi.useFakeTimers();
  const invoke = vi.fn().mockRejectedValue("a recording is still finishing");
  shell(invoke);
  const onMessage = vi.fn();
  render(<QuitButton onMessage={onMessage} />);
  fireEvent.click(screen.getByRole("button", { name: "Quit AudioRouter" }));
  act(() => vi.advanceTimersByTime(5000));
  expect(screen.getByRole("button").textContent).toBe("Quit");
  fireEvent.click(screen.getByRole("button"));
  await act(async () => fireEvent.click(screen.getByRole("button")));
  expect(onMessage).toHaveBeenCalledWith(expect.stringContaining("Quit refused: a recording is still finishing"));
  expect(screen.getByRole("button").textContent).toBe("Quit");
});

it("offers Quit anyway only when the backend did not answer", async () => {
  const invoke = vi
    .fn()
    .mockRejectedValueOnce("backendUnresponsive: control pipe request failed: timed out")
    .mockResolvedValueOnce(undefined);
  shell(invoke);
  const onMessage = vi.fn();
  render(<QuitButton onMessage={onMessage} />);
  fireEvent.click(screen.getByRole("button", { name: "Quit AudioRouter" }));
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Confirm quit AudioRouter" })));
  expect(onMessage).toHaveBeenCalledWith(expect.stringContaining("did not answer"));
  // One click: the user already confirmed once.
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Quit AudioRouter anyway" })));
  expect(invoke).toHaveBeenLastCalledWith("quit_app", { force: true });
});

it("never offers Quit anyway after a refusal", async () => {
  const invoke = vi.fn().mockRejectedValue("a recording is still finishing");
  shell(invoke);
  render(<QuitButton onMessage={() => {}} />);
  fireEvent.click(screen.getByRole("button", { name: "Quit AudioRouter" }));
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Confirm quit AudioRouter" })));
  expect(screen.queryByRole("button", { name: "Quit AudioRouter anyway" })).toBeNull();
  expect(invoke).not.toHaveBeenCalledWith("quit_app", { force: true });
});
