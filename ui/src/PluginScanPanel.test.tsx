/** @vitest-environment jsdom */
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { PluginScanPanel, PluginToolsGroup } from "./PluginPanels";
import { createDisconnectedBackend } from "./backend";
import { demoSession } from "./fixtures";

afterEach(cleanup);

it("reports failed standard scans instead of claiming zero plugins were found", async () => {
  const backend = {
    ...createDisconnectedBackend(demoSession),
    connected: true,
    scanPlugins: vi.fn().mockRejectedValue(new Error("tooManyCandidates: scan the vendor folder separately")),
  };
  render(
    <PluginToolsGroup
      backend={backend}
      connected
      search=""
      refreshKey={false}
      onAdd={() => {}}
      onOpenPicker={() => {}}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "Scan standard folders" }));
  await waitFor(() => expect(screen.getByText(/Plugin scan failed for/)).toBeTruthy());
  expect(screen.getByRole("button", { name: "Copy error" })).toBeTruthy();
});

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

it("offers one plugin guide for the whole list, not a misleading copy per plugin", async () => {
  const entry = (name: string) => ({
    path: `C:/VST3/${name}.vst3`,
    identity: {
      path: `C:/VST3/${name}.vst3`,
      binaryPath: `C:/VST3/${name}.vst3`,
      sha256: name,
      format: "vst3",
      compatibility: "supportedVst3X64",
    },
  });
  const pluginInventory = vi.fn(async () => ({
    inventories: [{ directory: "C:/VST3", entries: [entry("alpha"), entry("beta")] }],
  }));
  const backend = { ...createDisconnectedBackend(demoSession), connected: true, pluginInventory } as never;
  render(
    <PluginToolsGroup
      backend={backend}
      connected
      search=""
      refreshKey={false}
      onAdd={() => {}}
      onOpenPicker={() => {}}
    />,
  );
  await waitFor(() => expect(screen.getByText("beta")).toBeTruthy());
  const guides = screen.getAllByRole("link", { name: /guide/i });
  expect(guides.map((link) => link.getAttribute("aria-label"))).toEqual(["Open VST plugin guide"]);
});
