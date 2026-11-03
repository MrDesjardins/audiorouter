/** @vitest-environment jsdom */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { SetupCheckResult } from "@audiorouter/contracts";
import { SetupCheck, setupCheckSummary } from "./SetupCheck";
import { setupCheckRefreshKey } from "./SetupWorkbench";
import { demoSession } from "./fixtures";
import type { DeviceListItem } from "@audiorouter/contracts";

afterEach(cleanup);

const result = (findings: SetupCheckResult["findings"]): SetupCheckResult => ({
  findings,
  truncated: 0,
  sessionsChecked: 2,
  devicesChecked: 8,
  notChecked: ["Settings inside other apps."],
});

const surround = result([
  {
    code: "surroundDevice",
    severity: "problem",
    sessionId: "main",
    sessionName: "Main",
    nodeId: "siege-in",
    nodeName: "Siege",
    deviceId: "cable-b-in",
    deviceName: "CABLE-B Input",
    message: '"Siege" uses Surround, which needs a 5.1 or 7.1 device, but CABLE-B Input is set to 2 channel(s).',
    fix: "In Windows Sound settings, set CABLE-B Input to 7.1 Surround (Configure) at 48 kHz.",
  },
  {
    code: "cableRate",
    severity: "tip",
    sessionId: "main",
    sessionName: "Main",
    message: "CABLE Input runs at 44.1 kHz.",
    fix: "Set CABLE Input to 48000 Hz.",
  },
]);

describe("setupCheckSummary", () => {
  it("invalidates on playback and full device format changes, without polling on equivalent snapshots", () => {
    const devices = [
      {
        id: "d",
        name: "Device",
        direction: "render",
        state: "active",
        defaultRoles: [],
        format: {
          channels: 2,
          sampleRateHz: 48000,
          bitsPerSample: 32,
          bytesPerFrame: 8,
          formatTag: 3,
          isFloat32: true,
        },
        periods: { default100ns: 100000, minimum100ns: 30000 },
      },
    ] satisfies DeviceListItem[];
    const before = setupCheckRefreshKey(devices, [demoSession], []);
    expect(setupCheckRefreshKey(structuredClone(devices), [structuredClone(demoSession)], [])).toBe(before);
    expect(setupCheckRefreshKey(devices, [demoSession], [demoSession.id])).not.toBe(before);
    expect(
      setupCheckRefreshKey([{ ...devices[0], format: { ...devices[0].format, isFloat32: false } }], [demoSession], []),
    ).not.toBe(before);
  });
  it("counts each severity and the sessions", () => {
    expect(setupCheckSummary(null)).toBe("Checking your sessions and devices…");
    expect(setupCheckSummary(result([]))).toBe("No problems found in 2 sessions.");
    expect(setupCheckSummary(surround)).toBe("1 problem, 1 tip in 2 sessions.");
    expect(setupCheckSummary({ ...surround, truncated: 3 })).toBe("1 problem, 1 tip in 2 sessions (3 more not shown).");
  });
});

describe("SetupCheck", () => {
  it("does not keep announcing an initial failed request as checking", async () => {
    render(
      <SetupCheck
        connected
        checkSetup={async () => {
          throw new Error("denied");
        }}
        refreshKey="a"
      />,
    );
    await screen.findByText("Last check failed: denied");
    expect(screen.getByText("Setup check unavailable.")).toBeTruthy();
    expect(screen.queryByText("Checking your sessions and devices…")).toBeNull();
  });
  it("discards a response after disconnect and stops showing a pending check", async () => {
    let finish!: (result: SetupCheckResult) => void;
    const checkSetup = vi.fn(
      () =>
        new Promise<SetupCheckResult>((resolve) => {
          finish = resolve;
        }),
    );
    const { rerender } = render(<SetupCheck connected checkSetup={checkSetup} refreshKey="a" />);
    rerender(<SetupCheck connected={false} checkSetup={checkSetup} refreshKey="a" />);
    await act(async () => {
      finish(surround);
    });
    expect(screen.queryByText(/"Siege" uses Surround/)).toBeNull();
    expect(screen.getByText(/Not connected/)).toBeTruthy();
  });

  it("does not claim devices fit when Windows device enumeration failed", async () => {
    render(
      <SetupCheck
        connected
        checkSetup={async () => ({
          ...result([]),
          devicesChecked: 0,
          notChecked: ["Audio devices: the Windows device list could not be read."],
        })}
        refreshKey="a"
      />,
    );
    await screen.findByText(/Windows device list could not be read/);
    expect(screen.queryByText("Every device your sessions use fits how they use it.")).toBeNull();
    expect(screen.queryByText("No problems found in 2 sessions.")).toBeNull();
  });

  it("shows each finding with its fix and session", async () => {
    const checkSetup = vi.fn(async () => surround);
    render(<SetupCheck connected checkSetup={checkSetup} refreshKey="a" />);
    const problem = await screen.findByText(/"Siege" uses Surround/);
    expect(problem.textContent).toMatch(/^Problem: "Siege" uses Surround/);
    expect(screen.getByText(/set CABLE-B Input to 7.1 Surround/)).toBeTruthy();
    expect(screen.getByText("Session Main · Siege")).toBeTruthy();
    expect(screen.getByText(/CABLE Input runs at 44.1 kHz/).textContent).toMatch(/^Tip: /);
    expect(screen.getByText(/Not checked: Settings inside other apps/)).toBeTruthy();
    expect(checkSetup).toHaveBeenCalledTimes(1);
  });

  it("runs again when devices or sessions change and on Check again, not on re-render", async () => {
    const checkSetup = vi.fn(async () => result([]));
    const { rerender } = render(<SetupCheck connected checkSetup={checkSetup} refreshKey="a" />);
    await screen.findByText("No problems found in 2 sessions.");
    rerender(<SetupCheck connected checkSetup={checkSetup} refreshKey="a" />);
    expect(checkSetup).toHaveBeenCalledTimes(1);
    rerender(<SetupCheck connected checkSetup={checkSetup} refreshKey="b" />);
    await waitFor(() => expect(checkSetup).toHaveBeenCalledTimes(2));
    await waitFor(() =>
      expect((screen.getByRole("button", { name: "Check again" }) as HTMLButtonElement).disabled).toBe(false),
    );
    fireEvent.click(screen.getByRole("button", { name: "Check again" }));
    await waitFor(() => expect(checkSetup).toHaveBeenCalledTimes(3));
  });

  it("keeps the last result through a failed check", async () => {
    let fail = false;
    const checkSetup = vi.fn(async () => {
      if (fail) throw new Error("backend busy");
      return surround;
    });
    render(<SetupCheck connected checkSetup={checkSetup} refreshKey="a" />);
    await screen.findByText(/"Siege" uses Surround/);
    fail = true;
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Check again" }));
    });
    expect(await screen.findByText("Last check failed: backend busy")).toBeTruthy();
    expect(screen.getByText(/"Siege" uses Surround/)).toBeTruthy();
    expect(screen.getByText("1 problem, 1 tip in 2 sessions.")).toBeTruthy();
  });

  it("waits while disconnected and hides without a backend method", async () => {
    const checkSetup = vi.fn(async () => result([]));
    render(<SetupCheck connected={false} checkSetup={checkSetup} refreshKey="a" />);
    expect(screen.getByText(/Not connected/)).toBeTruthy();
    expect((screen.getByRole("button", { name: "Check again" }) as HTMLButtonElement).disabled).toBe(true);
    expect(checkSetup).not.toHaveBeenCalled();
    cleanup();
    const { container } = render(<SetupCheck connected refreshKey="a" />);
    expect(container.innerHTML).toBe("");
  });
});
