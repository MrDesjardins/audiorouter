/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { PluginScanPanel } from "./PluginPanels";
import { createDisconnectedBackend } from "./backend";
import { demoSession } from "./fixtures";

afterEach(cleanup);

it("marks a pending scan busy, disables competing actions and retains its directory after failure", async () => {
  let reject!: (error: Error) => void;
  const scanPlugins = vi.fn(
    () =>
      new Promise<never>((_, fail) => {
        reject = fail;
      }),
  );
  const backend = { ...createDisconnectedBackend(demoSession), connected: true, scanPlugins };
  render(<PluginScanPanel backend={backend} />);
  fireEvent.change(screen.getByLabelText("Absolute plugin directory"), { target: { value: "C:\\Plugins" } });
  fireEvent.click(screen.getByRole("button", { name: "Scan directory" }));
  for (const name of ["Scan directory", "Load last scan", "Retry scan", "Inspect path"]) {
    expect((screen.getByRole("button", { name }) as HTMLButtonElement).disabled).toBe(true);
  }
  expect(screen.getByRole("region", { name: "Plugin scan" }).getAttribute("aria-busy")).toBe("true");
  reject(new Error("Scan refused"));
  await waitFor(() => expect(screen.getByText(/Scan refused/)).toBeTruthy());
  expect((screen.getByLabelText("Absolute plugin directory") as HTMLInputElement).value).toBe("C:\\Plugins");
  expect((screen.getByRole("button", { name: "Retry scan" }) as HTMLButtonElement).disabled).toBe(false);
  expect(scanPlugins).toHaveBeenCalledTimes(1);
});
