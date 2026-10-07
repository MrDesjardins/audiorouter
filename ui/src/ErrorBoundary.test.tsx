/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { ErrorBoundary, RootRecoveryPanel, renderErrorDiagnostic } from "./ErrorBoundary";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

let shouldThrow = true;
function Fragile() {
  if (shouldThrow) throw new TypeError("node.parameters is undefined");
  return <p>Canvas is back</p>;
}

it("contains a render failure to its area, records a diagnostic and recovers on Try again", () => {
  vi.spyOn(console, "error").mockImplementation(() => {});
  shouldThrow = true;
  const onError = vi.fn();
  render(
    <>
      <button type="button">Stop</button>
      <ErrorBoundary area="Signal flow" onError={onError}>
        <Fragile />
      </ErrorBoundary>
    </>,
  );

  expect(screen.getByRole("alert", { name: "Signal flow stopped" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Stop" })).toBeTruthy();
  expect(onError).toHaveBeenCalledWith("UI render error in Signal flow (TypeError)");
  expect(screen.getByRole("button", { name: "Reload window" })).toBeTruthy();

  shouldThrow = false;
  fireEvent.click(screen.getByRole("button", { name: "Try again" }));
  expect(screen.getByText("Canvas is back")).toBeTruthy();
});

it("keeps sibling areas working when one area fails", () => {
  vi.spyOn(console, "error").mockImplementation(() => {});
  shouldThrow = true;
  function Counter() {
    const [count, setCount] = useState(0);
    return (
      <button type="button" onClick={() => setCount(count + 1)}>
        Clicked {count}
      </button>
    );
  }
  render(
    <>
      <ErrorBoundary area="Properties">
        <Fragile />
      </ErrorBoundary>
      <ErrorBoundary area="Side panel">
        <Counter />
      </ErrorBoundary>
    </>,
  );
  fireEvent.click(screen.getByRole("button", { name: "Clicked 0" }));
  expect(screen.getByRole("button", { name: "Clicked 1" })).toBeTruthy();
  expect(screen.queryByRole("alert", { name: "Side panel stopped" })).toBeNull();
});

it("never puts error text in the diagnostic, only a safe name", () => {
  const hostile = Object.assign(new Error("C:\\Users\\me\\secret.wav"), { name: "<script>" });
  expect(renderErrorDiagnostic("Properties", hostile)).toBe("UI render error in Properties (Error)");
  expect(renderErrorDiagnostic("Properties", "plain string")).toBe("UI render error in Properties (string)");
});

it("root recovery offers reload and privacy mute, and reports the mute result in a reserved slot", async () => {
  const mute = vi.fn().mockResolvedValue({ muted: true });
  render(<RootRecoveryPanel onPrivacyMute={mute} />);
  const status = screen.getByRole("status");
  expect(status.textContent).toBe("\u00a0");
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Turn on privacy mute" })));
  expect(mute).toHaveBeenCalledTimes(1);
  expect(status.textContent).toBe("Privacy mute is on.");
  expect((screen.getByRole("button", { name: "Turn on privacy mute" }) as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByRole("button", { name: "Reload window" })).toBeTruthy();
});

it("root recovery without a backend shows only reload", () => {
  render(<RootRecoveryPanel />);
  expect(screen.queryByRole("button", { name: "Turn on privacy mute" })).toBeNull();
  expect(screen.getByRole("button", { name: "Reload window" })).toBeTruthy();
});
