// @vitest-environment jsdom
import { render, screen, fireEvent, waitFor, cleanup } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { ApiPanel } from "./ApiPanel";

afterEach(() => { cleanup(); delete window.__TAURI_INTERNALS__; });
test("API tokens are revealed deliberately and cleared on stop", async () => {
  const invoke = vi.fn(async (_: string, args?: Record<string, unknown>) => ({ running: args?.action !== "stop" && args?.action !== "status", port: 17891, url: args?.action !== "stop" && args?.action !== "status" ? "http://127.0.0.1:17891" : null, token: args?.action === "reveal" ? "temporary-test-token" : null }));
  window.__TAURI_INTERNALS__ = { invoke };
  render(<ApiPanel />);
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("http_api_control", { action: "status", port: null }));
  fireEvent.click(screen.getByText("Start API"));
  await screen.findByText("Reveal API token");
  expect(screen.queryByLabelText("API bearer token")).toBeNull();
  fireEvent.click(screen.getByText("Reveal API token"));
  expect((await screen.findByLabelText("API bearer token") as HTMLInputElement).value).toBe("temporary-test-token");
  fireEvent.click(screen.getByText("Stop API"));
  await screen.findByText("Start API");
  expect(screen.queryByLabelText("API bearer token")).toBeNull();
});
