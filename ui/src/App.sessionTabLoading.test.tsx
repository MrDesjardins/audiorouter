/** @vitest-environment jsdom */
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { App } from "./App";
import { createDisconnectedBackend } from "./backend";

// The session file section loads lazily. Hold its module forever: the
// Session tab's picker and actions must still render (CI's slow first
// import once hid the whole tab behind "Loading panel...").
vi.mock("./SessionFilePanel", () => new Promise(() => {}));

beforeAll(() => {
  Object.defineProperty(window, "DOMMatrixReadOnly", {
    configurable: true,
    value: class {
      m22 = 1;
    },
  });
  Object.defineProperty(globalThis, "ResizeObserver", {
    configurable: true,
    value: class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  });
});
afterEach(() => cleanup());

it("shows the session picker while the session file section is still loading", async () => {
  await act(async () => {
    render(<App backend={createDisconnectedBackend()} />);
  });
  fireEvent.click(await screen.findByRole("tab", { name: "Session" }));
  expect(await screen.findByLabelText("Choose session")).toBeTruthy();
  expect(screen.getByText("Loading session file tools...")).toBeTruthy();
});
