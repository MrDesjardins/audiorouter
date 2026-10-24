/** @vitest-environment jsdom */
import { act, cleanup, fireEvent, render, renderHook, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ApplicationInfo, DeviceListItem } from "@audiorouter/contracts";
import { AppsPanel } from "./AppsPanel";
import { AppStatusStrip } from "./AppStatusStrip";
import { pendingDiscordCheck, type AppCheck } from "./appUsage";
import { APP_USAGE_POLL_MS, useApplicationUsage, usageFingerprint } from "./useApplicationUsage";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

const device = (id: string, name: string, direction: "capture" | "render"): DeviceListItem => ({
  id,
  name,
  direction,
  state: "active",
  defaultRoles: [],
  format: { sampleRateHz: 48_000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 },
  periods: { default100ns: 100_000, minimum100ns: 30_000 },
});
const devices = [
  device("cable-a-out", "CABLE-A Output (VB-Audio Virtual Cable A)", "capture"),
  device("cable-b-in", "CABLE-B Input (VB-Audio Virtual Cable B)", "render"),
];
const siege: ApplicationInfo = {
  processId: 7,
  executable: "RainbowSix.exe",
  executablePath: null,
  creationTime100ns: null,
  audioActivity: "active",
  captureCapability: "observed",
  audioSessionCount: 2,
  activeAudioSessionCount: 2,
  captureSessionCount: 1,
  renderSessionCount: 1,
  audioDisplayNames: [],
  audioEndpoints: [
    { deviceId: "cable-a-out", direction: "capture", active: true },
    { deviceId: "cable-b-in", direction: "render", active: true },
  ],
};

const needsCheck: AppCheck = {
  appId: "discord",
  state: "check",
  summary: "1 setting to check",
  rows: [
    {
      id: "microphone",
      label: "Microphone",
      deviceName: "Microphone (PD200X Podcast Microphone)",
      state: "check",
      message: "This is your microphone without AudioRouter's effects.",
      expectedName: "CABLE-A Output (VB-Audio Virtual Cable A)",
    },
    { id: "speaker", label: "Speaker", deviceName: "CABLE Input", state: "ready", message: "Discord plays here." },
  ],
};

function panel(discord: AppCheck, applications: ApplicationInfo[] = []) {
  return (
    <AppsPanel
      discord={discord}
      devices={devices}
      applications={applications}
      stale={false}
      connected
      onRefresh={vi.fn()}
    />
  );
}

describe("AppsPanel", () => {
  it("states each problem with its device and the exact Discord step", () => {
    const writeText = vi.fn(() => Promise.resolve());
    Object.assign(navigator, { clipboard: { writeText } });
    const { container } = render(panel(needsCheck));
    // The state is a word as well as a colour.
    expect(container.querySelector(".app-state-badge.is-check")?.textContent).toBe("! Check");
    expect(screen.getByText("Microphone (PD200X Podcast Microphone)")).toBeTruthy();
    // One "Show me how" only: the ready row has nothing to fix.
    const how = screen.getByRole("button", { name: "Show me how" });
    expect(how.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(how);
    expect(screen.getByText(/set Input Device to/).textContent).toContain("CABLE-A Output (VB-Audio Virtual Cable A)");
    fireEvent.click(screen.getByRole("button", { name: "Copy device name" }));
    expect(writeText).toHaveBeenCalledWith("CABLE-A Output (VB-Audio Virtual Cable A)");
  });

  it("keeps the same two rows while waiting, so the card does not move", () => {
    const { rerender } = render(panel(pendingDiscordCheck()));
    const rows = () => [...document.querySelectorAll(".app-check-row strong")].map((item) => item.textContent);
    expect(rows()).toEqual(["Microphone", "Speaker"]);
    rerender(panel(needsCheck));
    expect(rows()).toEqual(["Microphone", "Speaker"]);
  });

  it("lists other apps with their role, and says plainly when there are none", () => {
    const { rerender } = render(panel(needsCheck));
    expect(screen.getByText("No other app is using an audio device right now.")).toBeTruthy();
    rerender(panel(needsCheck, [siege]));
    expect(screen.getByText("RainbowSix records from it")).toBeTruthy();
    expect(screen.getByText("RainbowSix plays to it or records it")).toBeTruthy();
  });
});

describe("AppStatusStrip", () => {
  it("names the state for screen readers and opens Apps", () => {
    const onOpen = vi.fn();
    render(<AppStatusStrip checks={[needsCheck]} onOpen={onOpen} />);
    const chip = screen.getByRole("button", { name: "Discord: needs a check. Open Apps." });
    expect(chip.getAttribute("title")).toBe("Discord: 1 setting to check");
    fireEvent.click(chip);
    expect(onOpen).toHaveBeenCalledWith(needsCheck);
  });

  it("renders nothing when the session does not involve an app", () => {
    const { container } = render(<AppStatusStrip checks={[]} onOpen={vi.fn()} />);
    expect(container.innerHTML).toBe("");
  });
});

describe("useApplicationUsage", () => {
  it("fingerprints what the checks read, independent of process order", () => {
    expect(usageFingerprint([siege])).not.toBe(
      usageFingerprint([{ ...siege, audioEndpoints: [{ ...siege.audioEndpoints[0], active: false }] }]),
    );
    expect(usageFingerprint([siege, { ...siege, processId: 8 }])).toBe(
      usageFingerprint([{ ...siege, processId: 8 }, siege]),
    );
  });

  it("refreshes on focus and polls only while asked, keeping the last list on failure", async () => {
    vi.useFakeTimers();
    const list = vi.fn(() => Promise.resolve([siege]));
    const { result, rerender } = renderHook(({ polling }) => useApplicationUsage(list, { polling }), {
      initialProps: { polling: false },
    });
    await act(async () => {});
    expect(list).toHaveBeenCalledTimes(1);
    expect(result.current).toMatchObject({ loaded: true, stale: false, applications: [siege] });

    await act(async () => {
      vi.advanceTimersByTime(APP_USAGE_POLL_MS * 3);
    });
    expect(list).toHaveBeenCalledTimes(1);

    await act(async () => {
      window.dispatchEvent(new Event("focus"));
    });
    expect(list).toHaveBeenCalledTimes(2);

    rerender({ polling: true });
    list.mockImplementationOnce(() => Promise.reject(new Error("backend busy")));
    await act(async () => {
      vi.advanceTimersByTime(APP_USAGE_POLL_MS);
    });
    expect(list).toHaveBeenCalledTimes(3);
    expect(result.current).toMatchObject({ stale: true, applications: [siege] });
  });

  it("does nothing without a connected backend", async () => {
    const { result } = renderHook(() => useApplicationUsage(null, { polling: true }));
    await act(async () => {});
    expect(result.current).toMatchObject({ loaded: false, applications: [] });
  });
});
