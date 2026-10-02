// @vitest-environment jsdom
import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { ApiPanel } from "./ApiPanel";
afterEach(() => { cleanup(); delete window.__TAURI_INTERNALS__; vi.restoreAllMocks(); });
function fixture() {
  let token = "a".repeat(64); let running = false;
  const invoke = vi.fn(async (_: string, args?: Record<string, unknown>) => {
    if (args?.action === "start") running = true;
    if (args?.action === "stop") running = false;
    if (args?.action === "regenerate") token = "b".repeat(64);
    return { running, port: 17891, url: running ? "http://127.0.0.1:17891" : null, token: args?.action === "reveal" || args?.action === "regenerate" ? token : null };
  });
  window.__TAURI_INTERNALS__ = { invoke }; render(<ApiPanel />); return invoke;
}
test("saved token is visible and copyable while stopped, and remains across start/stop", async () => {
  const copy = vi.fn(async () => undefined);
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: copy } });
  const invoke = fixture();
  const field = await screen.findByLabelText("API bearer token") as HTMLTextAreaElement;
  expect(field.value).toBe("a".repeat(64));
  expect(invoke).toHaveBeenCalledWith("http_api_control", { action: "reveal", port: null });
  fireEvent.focus(field); expect(field.selectionEnd - field.selectionStart).toBe(64);
  fireEvent.click(screen.getByText("Copy API token")); await screen.findByText("Token copied.");
  expect(copy).toHaveBeenCalledWith("a".repeat(64));
  fireEvent.click(screen.getByText("Start API")); await screen.findByText("Stop API");
  expect(field.value).toBe("a".repeat(64));
  fireEvent.click(screen.getByText("Stop API")); await screen.findByText("Start API");
  expect(field.value).toBe("a".repeat(64));
  fireEvent.click(screen.getByText("Hide API token")); expect(screen.queryByLabelText("API bearer token")).toBeNull();
  fireEvent.click(screen.getByText("Reveal API token")); expect((await screen.findByLabelText("API bearer token") as HTMLTextAreaElement).value).toBe("a".repeat(64));
});
test("explicit replacement immediately shows new token without starting API", async () => {
  const invoke = fixture(); await screen.findByLabelText("API bearer token");
  fireEvent.click(screen.getByText("Generate new token"));
  expect(invoke.mock.calls.some(call => call[1]?.action === "regenerate")).toBe(false);
  fireEvent.click(screen.getByText("Cancel")); expect(screen.queryByText("Replace API token")).toBeNull();
  fireEvent.click(screen.getByText("Generate new token")); fireEvent.click(screen.getByText("Replace API token"));
  await screen.findByText(/New token generated/);
  expect((screen.getByLabelText("API bearer token") as HTMLTextAreaElement).value).toBe("b".repeat(64));
  expect(screen.getByText("Start API")).toBeTruthy();
});
test("missing clipboard offers manual copy", async () => {
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: undefined });
  fixture(); await screen.findByLabelText("API bearer token");
  fireEvent.click(screen.getByText("Copy API token"));
  await screen.findByText("Select the token and copy it manually.");
});
