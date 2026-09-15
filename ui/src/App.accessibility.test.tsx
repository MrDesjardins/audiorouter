/** @vitest-environment jsdom */

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { App, DIAGNOSTICS_REFRESH_INTERVAL_MS, findVbCableCaptureEndpointId, findVbCableEndpointPair, formatNativePumpSummary, formatRecordingDuration, WORKSPACE_EVENT_CATEGORIES } from "./App";
import { createDisconnectedBackend } from "./backend";
import { DraftConnectionList, insertMixerActionId, removeMixerActionId } from "./DraftConnectionList";
import { appendDraftConnection, insertDraftMixer } from "./draft";
import { demoSession } from "./fixtures";
import { BackendConnectionContext } from "./backendConnectionContext";
import { GraphList } from "./GraphList";
import type { ProcessorDescriptor } from "./processorCatalog";
import { AudioRouterRpcError } from "@audiorouter/contracts";
import type { EventsSubscribeResult, RecordingRow, VirtualDeviceInfo } from "@audiorouter/contracts";

function connectedPreviewBackend() {
  return { ...createDisconnectedBackend(), connected: true };
}

beforeAll(() => {
  Object.defineProperty(globalThis, "ResizeObserver", {
    configurable: true,
    value: class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  });
});

afterEach(() => {
  cleanup();
  window.localStorage.clear();
});

describe("VB-Cable endpoint selection", () => {
  it("shows native startup registration status independently of backend capability", async () => {
    render(<App backend={{
      ...connectedPreviewBackend(),
      getStartup: async () => ({ enabled: true, registration: "unavailable", reason: "portable backend state" }),
      startupRegistrationStatus: async () => "registered",
    }} />);
    expect(await screen.findByText("Native registration: registered")).toBeTruthy();
  });

  it("keeps the native startup result visible while reconciling observed state", async () => {
    let desired = false;
    const backend = {
      ...connectedPreviewBackend(),
      getStartup: vi.fn(async () => ({ enabled: desired, registration: "unavailable" as const, reason: "shell-owned" })),
      planStartup: vi.fn(async () => ({ planId: "startup-plan", enabled: true, registration: "unavailable" as const, reason: "shell-owned", requiredScopes: ["startupWrite" as const], warnings: [] })),
      applyStartup: vi.fn(async () => ({ planId: "startup-plan", state: "unavailable" as const, registration: "unavailable" as const, reason: "shell-owned" })),
      registerStartup: vi.fn(async (enabled: boolean) => { desired = enabled; return '"C:\\Program Files\\AudioRouter\\audiorouter-shell.exe"'; }),
      startupRegistrationStatus: vi.fn(async () => (desired ? "registered" as const : "unregistered" as const)),
    };
    render(<App backend={backend} />);
    await waitFor(() => expect(backend.getStartup).toHaveBeenCalled());
    fireEvent.change(screen.getByLabelText("Desired sign-in startup policy"), { target: { value: "enabled" } });
    fireEvent.click(screen.getByRole("button", { name: "Plan startup policy" }));
    await screen.findByRole("button", { name: "Apply planned policy" });
    fireEvent.click(screen.getByRole("button", { name: "Apply planned policy" }));
    expect(await screen.findByText(/Startup registration enabled:/)).toBeTruthy();
    expect(await screen.findByText("Native registration: registered")).toBeTruthy();
  });

  it("applies the backend-approved startup plan value to native registration", async () => {
    const registerStartup = vi.fn(async () => "startup-command");
    const backend = {
      ...connectedPreviewBackend(),
      getStartup: vi.fn(async () => ({ enabled: false, registration: "unavailable" as const, reason: "shell-owned" })),
      planStartup: vi.fn(async () => ({ planId: "startup-plan", enabled: false, registration: "unavailable" as const, reason: "normalized by backend", requiredScopes: ["startupWrite" as const], warnings: [] })),
      applyStartup: vi.fn(async () => ({ planId: "startup-plan", state: "unavailable" as const, registration: "unavailable" as const, reason: "shell-owned" })),
      registerStartup,
    };
    render(<App backend={backend} />);
    fireEvent.change(screen.getByLabelText("Desired sign-in startup policy"), { target: { value: "enabled" } });
    fireEvent.click(screen.getByRole("button", { name: "Plan startup policy" }));
    fireEvent.click(await screen.findByRole("button", { name: "Apply planned policy" }));
    await waitFor(() => expect(registerStartup).toHaveBeenCalledWith(false));
    expect(await screen.findByText(/Startup registration disabled/)).toBeTruthy();
  });

  it("does not register native startup when backend apply fails", async () => {
    const registerStartup = vi.fn(async () => "startup-command");
    const backend = {
      ...connectedPreviewBackend(),
      getStartup: vi.fn(async () => ({ enabled: false, registration: "unavailable" as const, reason: "shell-owned" })),
      planStartup: vi.fn(async () => ({ planId: "startup-plan", enabled: true, registration: "unavailable" as const, reason: "shell-owned", requiredScopes: ["startupWrite" as const], warnings: [] })),
      applyStartup: vi.fn(async () => { throw new Error("backend apply rejected"); }),
      registerStartup,
    };
    render(<App backend={backend} />);
    fireEvent.change(screen.getByLabelText("Desired sign-in startup policy"), { target: { value: "enabled" } });
    fireEvent.click(screen.getByRole("button", { name: "Plan startup policy" }));
    fireEvent.click(await screen.findByRole("button", { name: "Apply planned policy" }));
    expect(await screen.findByText(/backend apply rejected/)).toBeTruthy();
    expect(registerStartup).not.toHaveBeenCalled();
  });

  it("disables startup apply after the backend disconnects", async () => {
    const applyStartup = vi.fn(async () => ({ planId: "startup-plan", state: "unavailable" as const, registration: "unavailable" as const, reason: "shell-owned" }));
    const backend = {
      ...connectedPreviewBackend(),
      getStartup: vi.fn(async () => ({ enabled: false, registration: "unavailable" as const, reason: "shell-owned" })),
      planStartup: vi.fn(async () => ({ planId: "startup-plan", enabled: true, registration: "unavailable" as const, reason: "shell-owned", requiredScopes: ["startupWrite" as const], warnings: [] })),
      applyStartup,
    };
    const { rerender } = render(<App backend={backend} />);
    fireEvent.change(screen.getByLabelText("Desired sign-in startup policy"), { target: { value: "enabled" } });
    fireEvent.click(screen.getByRole("button", { name: "Plan startup policy" }));
    const apply = await screen.findByRole("button", { name: "Apply planned policy" });
    backend.connected = false;
    rerender(<App backend={backend} />);
    expect((apply as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(apply);
    expect(applyStartup).not.toHaveBeenCalled();
  });

  it("prevents duplicate startup apply while the first request is pending", async () => {
    let releaseApply!: (value: { planId: string; state: "unavailable"; registration: "unavailable"; reason: string }) => void;
    const applyResult = new Promise<{ planId: string; state: "unavailable"; registration: "unavailable"; reason: string }>((resolve) => { releaseApply = resolve; });
    const applyStartup = vi.fn(() => applyResult);
    const backend = {
      ...connectedPreviewBackend(),
      getStartup: vi.fn(async () => ({ enabled: false, registration: "unavailable" as const, reason: "shell-owned" })),
      planStartup: vi.fn(async () => ({ planId: "startup-plan", enabled: true, registration: "unavailable" as const, reason: "shell-owned", requiredScopes: ["startupWrite" as const], warnings: [] })),
      applyStartup,
    };
    render(<App backend={backend} />);
    fireEvent.change(screen.getByLabelText("Desired sign-in startup policy"), { target: { value: "enabled" } });
    fireEvent.click(screen.getByRole("button", { name: "Plan startup policy" }));
    const apply = await screen.findByRole("button", { name: "Apply planned policy" });
    fireEvent.click(apply);
    expect((apply as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(apply);
    expect(applyStartup).toHaveBeenCalledTimes(1);
    releaseApply({ planId: "startup-plan", state: "unavailable", registration: "unavailable", reason: "shell-owned" });
    await waitFor(() => expect(applyStartup).toHaveBeenCalledTimes(1));
  });

  it("refreshes startup state when the existing backend reconnects", async () => {
    const backend = {
      ...connectedPreviewBackend(),
      connected: false,
      getStartup: vi.fn(async () => ({ enabled: false, registration: "unavailable" as const, reason: "reconnected" })),
    };
    const { rerender } = render(<App backend={backend} />);
    await waitFor(() => expect(backend.getStartup).toHaveBeenCalledTimes(1));
    backend.connected = true;
    rerender(<App backend={backend} />);
    await waitFor(() => expect(backend.getStartup).toHaveBeenCalledTimes(2));
  });

  it("does not let an older startup refresh overwrite newer observed state", async () => {
    let releaseInitial!: (value: { enabled: boolean; registration: "unavailable"; reason: string }) => void;
    const initial = new Promise<{ enabled: boolean; registration: "unavailable"; reason: string }>((resolve) => { releaseInitial = resolve; });
    const backend = {
      ...connectedPreviewBackend(),
      getStartup: vi.fn()
        .mockReturnValueOnce(initial)
        .mockResolvedValueOnce({ enabled: true, registration: "unavailable" as const, reason: "newer observed state" }),
    };
    render(<App backend={backend} />);
    const startupPanel = screen.getByRole("region", { name: "Start at sign-in" });
    fireEvent.click(within(startupPanel).getByRole("button", { name: "Refresh" }));
    expect(await screen.findByText("newer observed state")).toBeTruthy();
    releaseInitial({ enabled: false, registration: "unavailable", reason: "stale observed state" });
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(screen.queryByText("stale observed state")).toBeNull();
    expect(screen.getByText("newer observed state")).toBeTruthy();
  });

  it("keeps workspace events bounded to state categories and excludes meters", () => {
    expect(WORKSPACE_EVENT_CATEGORIES).toContain("graph.committed");
    expect(WORKSPACE_EVENT_CATEGORIES).toContain("recording.recycled");
    expect(WORKSPACE_EVENT_CATEGORIES).toContain("devices.changed");
    expect(WORKSPACE_EVENT_CATEGORIES).toContain("recovery.safeModeCleared");
    expect(WORKSPACE_EVENT_CATEGORIES.some((category) => category.startsWith("meter"))).toBe(false);
  });

  it("forwards the bounded workspace categories through the live event loop", async () => {
    const subscribe = vi.fn(async (_afterSequence?: number, _sessionId?: string, _backendEpoch?: number, _categories?: string[]) => ({ backendEpoch: 0, events: [], nextSequence: 0 }));
    render(<App backend={{ ...connectedPreviewBackend(), subscribe }} />);
    await waitFor(() => expect(subscribe).toHaveBeenCalled());
    expect(subscribe.mock.calls[0]?.[3]).toEqual([...WORKSPACE_EVENT_CATEGORIES]);
  });

  it("refreshes the recording library when a live state event arrives", async () => {
    const recording: RecordingRow = { id: "take-live", sessionId: demoSession.id, recorderId: "recorder-1", path: "C:\\Audio\\take-live.wav", format: "wav", channels: 2, sampleRate: 48000, frames: 480, fileBytes: 1964, startTime: "2026-09-14T01:00:00Z", state: "completed", missing: false, title: null, artist: null, comment: null, dither: true, conversion: "targetSampleRate=48000;channels=2;format=wav" };
    let recordingListCalls = 0;
    const listRecordings = vi.fn(async () => {
      recordingListCalls += 1;
      return recordingListCalls < 2 ? [] : [recording];
    });
    let eventSent = false;
    const subscribe = vi.fn(async (): Promise<EventsSubscribeResult> => {
      if (eventSent) return { backendEpoch: 0, events: [], nextSequence: 1 };
      eventSent = true;
      return { backendEpoch: 0, events: [{ sequence: 1, backendEpoch: 0, resourceRevision: 1, operationId: null, category: "recorder.changed", sessionId: demoSession.id }], nextSequence: 1 };
    });
    render(<App backend={{ ...connectedPreviewBackend(), listRecordings, subscribe }} />);
    await waitFor(() => expect(screen.getByText("C:\\Audio\\take-live.wav")).toBeTruthy());
    expect(listRecordings).toHaveBeenCalledTimes(2);
  });

  it("shows a bounded recovery notice for a bridge event", async () => {
    let eventSent = false;
    const subscribe = vi.fn(async (): Promise<EventsSubscribeResult> => {
      if (eventSent) return { backendEpoch: 0, events: [], nextSequence: 1 };
      eventSent = true;
      return { backendEpoch: 0, events: [{ sequence: 1, backendEpoch: 0, resourceRevision: 0, operationId: "voice-bus", category: "virtualBridge.expired", sessionId: null }], nextSequence: 1 };
    });
    render(<App backend={{ ...connectedPreviewBackend(), subscribe }} />);
    expect(await screen.findByText(/Virtual bridge lease expired for voice-bus; the route is silenced/i)).toBeTruthy();
  });

  it("formats recorder drain telemetry only for a running native route", () => {
    const stats = { sessionId: demoSession.id, generation: 1, packets: 1, capturedFrames: 128, processedQuanta: 1, renderedFrames: 128, droppedRenderFrames: 0, renderBackpressureEvents: 0, recorderChunksDrained: 3 };
    expect(formatNativePumpSummary(stats, true)).toBe("native 128 in / 128 out / 1 quanta / 3 recorder chunks");
    expect(formatNativePumpSummary({ ...stats, recorderChunksDrained: 0 }, true)).toBe("native 128 in / 128 out / 1 quanta");
    expect(formatNativePumpSummary({ ...stats, droppedRenderFrames: 2, renderBackpressureEvents: 1 }, true)).toBe("native 128 in / 128 out / 1 quanta / 3 recorder chunks / 2 dropped / 1 backpressure");
    expect(formatNativePumpSummary({ sessionId: "demo-session", generation: 1, input: { packets: 1, capturedFrames: 128, processedQuanta: 1, renderedFrames: 128, droppedRenderFrames: 0, renderBackpressureEvents: 0 }, output: { packets: 1, capturedFrames: 0, processedQuanta: 1, renderedFrames: 128, droppedRenderFrames: 0, renderBackpressureEvents: 0 } }, true)).toBe("native 128 in / 128 out / 2 quanta");
    expect(formatNativePumpSummary(stats, false)).toBeNull();
  });

  it("formats recording duration from bounded frame metadata", () => {
    expect(formatRecordingDuration(480, 48000)).toBe("00:00:00.010");
    expect(formatRecordingDuration(180000, 48000)).toBe("00:00:03.750");
    expect(formatRecordingDuration(-1, 48000)).toBe("unknown");
  });

  it("keeps diagnostics refresh at the 20 Hz default below the 30 Hz ceiling", () => {
    expect(DIAGNOSTICS_REFRESH_INTERVAL_MS).toBe(50);
  });

  it("exposes the human-testable route sequence without duplicating controls", async () => {
    render(<App backend={connectedPreviewBackend()} />);
    const quickRoute = await screen.findByRole("region", { name: "Quick route" });
    expect(within(quickRoute).getByRole("link", { name: "Select endpoints" }).getAttribute("href")).toBe("#native-endpoint-panel");
    expect(within(quickRoute).getByRole("link", { name: "Build the graph" }).getAttribute("href")).toBe("#signal-flow-panel");
    expect(within(quickRoute).getByRole("link", { name: "Start the session" }).getAttribute("href")).toBe("#native-endpoint-panel");
  });

  it("keeps the backend-authored audio reason visible in the status summary", async () => {
    render(<App backend={createDisconnectedBackend()} />);
    expect(await screen.findByText(/unavailable audio \(The control backend is disconnected\.\)/)).toBeTruthy();
  });

  it("shows backend-authored recovery safe mode and recent crash count", async () => {
    const disconnected = createDisconnectedBackend();
    let safeMode = true;
    const clearRecoverySafeMode = vi.fn(async () => {
      safeMode = false;
      return { safeMode: false as const, recentCrashes: 0 as const, persistence: "memory" as const };
    });
    const backend = {
      ...disconnected,
      connected: true,
      clearRecoverySafeMode,
      snapshot: async () => {
        const current = await disconnected.snapshot();
        return {
          ...current,
          status: {
            ...current.status,
            recovery: { safeMode, recentCrashes: safeMode ? 3 : 0, persistence: "memory" as const },
          },
        };
      },
    };
    render(<App backend={backend} />);
    expect(await screen.findByRole("heading", { name: "Safe mode is active" })).toBeTruthy();
    expect(screen.getByText("3 recent crashes")).toBeTruthy();
    expect(screen.getByText("Recovery state is held in memory for this preview.")).toBeTruthy();
    const clearButton = screen.getByRole("button", { name: "Clear safe mode" });
    expect(clearButton.hasAttribute("disabled")).toBe(false);
    fireEvent.click(clearButton);
    expect(clearRecoverySafeMode).toHaveBeenCalledWith(expect.any(String));
    expect(await screen.findByRole("heading", { name: "Normal startup mode" })).toBeTruthy();
    expect(screen.getByText("0 recent crashes")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Clear safe mode" }).hasAttribute("disabled")).toBe(true);
  });

  it("offers a persisted compact route status view with live controls", async () => {
    render(<App backend={connectedPreviewBackend()} />);
    const toggle = await screen.findByRole("button", { name: "Compact status" });
    expect(toggle.getAttribute("aria-pressed")).toBe("false");
    fireEvent.click(toggle);
    expect(screen.getByRole("region", { name: "Compact route status" })).toBeTruthy();
    expect(within(screen.getByRole("region", { name: "Compact route status" })).getByRole("button", { name: "Start session" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Full workspace" }).getAttribute("aria-pressed")).toBe("true");
    expect(window.localStorage.getItem("audiorouter.ui.compact-status")).toBe("true");
    fireEvent.click(screen.getByRole("button", { name: "Full workspace" }));
    expect(screen.queryByRole("region", { name: "Compact route status" })).toBeNull();
    expect(window.localStorage.getItem("audiorouter.ui.compact-status")).toBe("false");
  });

  it("forwards the selected application capture policy", async () => {
    const prepareNativeApplication = vi.fn(async () => ({ sessionId: demoSession.id, state: "configured-stopped" as const, processId: 42, executable: "Music.exe", executablePath: "C:\\Apps\\Music.exe", creationTime100ns: "123", mode: "exclude" as const, renderEndpointId: "render-test" }));
    const backend = {
      ...connectedPreviewBackend(),
      listApplications: async () => [{
        processId: 42,
        executable: "Music.exe",
        executablePath: "C:\\Apps\\Music.exe",
        audioDisplayNames: ["Music"],
        audioActivity: "active" as const,
        captureCapability: "observed" as const,
        audioSessionCount: 1,
        activeAudioSessionCount: 1,
        captureSessionCount: 1,
        renderSessionCount: 1,
        creationTime100ns: "123",
      }],
      prepareNativeApplication,
    };
    window.localStorage.setItem(`audiorouter.ui.endpoint-binding.${demoSession.id}`, JSON.stringify({ renderEndpointId: "render-test" }));
    render(<App backend={backend} />);
    await screen.findByRole("button", { name: "Prepare application worker" });
    fireEvent.click(screen.getByRole("button", { name: "Prepare application worker" }));
    await waitFor(() => expect(prepareNativeApplication).toHaveBeenCalledWith(expect.objectContaining({ mode: "include" })));
    prepareNativeApplication.mockClear();
    fireEvent.change(screen.getByRole("combobox", { name: "Application capture policy" }), { target: { value: "exclude" } });
    fireEvent.click(screen.getByRole("button", { name: "Prepare application worker" }));
    await waitFor(() => expect(prepareNativeApplication).toHaveBeenCalledWith(expect.objectContaining({ mode: "exclude" })));
  });

  it("adds a verified scanned plugin as a stopped draft placeholder", async () => {
    const backend = {
      ...connectedPreviewBackend(),
      scanPlugins: async () => ({
        directory: "C:\\Plugins",
        entries: [{
          path: "C:\\Plugins\\Effect.dll",
          identity: {
            path: "C:\\Plugins\\Effect.dll",
            binaryPath: "C:\\Plugins\\Effect.dll",
            format: "vst2" as const,
            architecture: "x64" as const,
            fileBytes: 4096,
            sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            vendor: "Test Vendor",
            version: "1.0",
            classIds: ["test-class"],
            compatibility: "supportedVst2X64Gated" as const,
          },
          error: null,
          errorCode: null,
        }],
      }),
    };
    render(<App backend={backend} />);
    fireEvent.change(await screen.findByRole("textbox", { name: "Absolute plugin directory" }), { target: { value: "C:\\Plugins" } });
    fireEvent.click(screen.getByRole("button", { name: "Scan directory" }));
    fireEvent.click(await screen.findByRole("button", { name: "Select for inspection" }));
    expect(screen.queryByRole("button", { name: "Test Vendor 1 plugin" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Add to draft: C:\\Plugins\\Effect.dll" }));
    fireEvent.click(screen.getByRole("button", { name: "List view" }));
    const pluginNode = await screen.findByRole("button", { name: /Test Vendor.*plugin/ });
    expect(pluginNode).toBeTruthy();
    fireEvent.click(pluginNode);
    expect(screen.getByText(/Plugin parameters unavailable|Loading bounded parameters/i)).toBeTruthy();
    expect(screen.getByText(/added a stopped plugin placeholder/i)).toBeTruthy();
  });

  it("clears stale plugin inspection when selecting another scan result", async () => {
    let scanCount = 0;
    const backend = {
      ...connectedPreviewBackend(),
      scanPlugins: async () => {
        scanCount += 1;
        if (scanCount > 1) throw new Error("refresh failed");
        return {
        directory: "C:\\Plugins",
        entries: [
          { path: "C:\\Plugins\\one.dll", identity: null, error: "first", errorCode: "io" as const },
          { path: "C:\\Plugins\\two.dll", identity: null, error: "second", errorCode: "io" as const },
        ],
        };
      },
      inspectPlugin: vi.fn(async (path: string) => ({ path, identity: null, error: `inspected ${path}`, errorCode: "io" as const })),
    };
    render(<App backend={backend} />);
    fireEvent.change(await screen.findByRole("textbox", { name: "Absolute plugin directory" }), { target: { value: "C:\\Plugins" } });
    fireEvent.click(screen.getByRole("button", { name: "Scan directory" }));
    const selections = await screen.findAllByRole("button", { name: "Select for inspection" });
    fireEvent.click(selections[0]);
    fireEvent.click(screen.getByRole("button", { name: "Inspect path" }));
    expect(await screen.findByText(/inspected C:\\Plugins\\one\.dll/)).toBeTruthy();
    fireEvent.click(screen.getAllByRole("button", { name: "Select for inspection" })[1]);
    expect(screen.queryByText(/inspected C:\\Plugins\\one\.dll/)).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Inspect path" }));
    expect(await screen.findByText(/inspected C:\\Plugins\\two\.dll/)).toBeTruthy();
    fireEvent.change(screen.getByRole("textbox", { name: "Absolute plugin path" }), { target: { value: "C:\\Plugins\\manual.dll" } });
    expect(screen.queryByText(/inspected C:\\Plugins\\two\.dll/)).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Scan directory" }));
    expect(screen.queryByText(/inspected C:\\Plugins\\two\.dll/)).toBeNull();
  });

  it("returns exact IDs only for one active, unambiguous pair", () => {
    const format = { sampleRateHz: 48000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 };
    expect(findVbCableEndpointPair([
      { id: "capture-vb", name: "CABLE Output (VB-Audio Virtual Cable)", direction: "capture", state: "active", defaultRoles: [], format, periods: { default100ns: 100000, minimum100ns: 30000 } },
      { id: "render-vb", name: "CABLE Input (VB-Audio Virtual Cable)", direction: "render", state: "active", defaultRoles: [], format, periods: { default100ns: 100000, minimum100ns: 30000 } },
      { id: "inactive-vb", name: "CABLE Output (VB-Audio Virtual Cable)", direction: "capture", state: "unplugged", defaultRoles: [] },
    ])).toEqual({ captureEndpointId: "capture-vb", renderEndpointId: "render-vb" });
    expect(findVbCableCaptureEndpointId([
      { id: "capture-vb", name: "CABLE Output (VB-Audio Virtual Cable)", direction: "capture", state: "active", defaultRoles: [], format, periods: { default100ns: 100000, minimum100ns: 30000 } },
    ])).toBe("capture-vb");
    expect(findVbCableEndpointPair([{ id: "duplicate", name: "CABLE Output (VB-Audio Virtual Cable)", direction: "capture", state: "active", defaultRoles: [], format, periods: { default100ns: 100000, minimum100ns: 30000 } }])).toBeNull();
  });

  it("selects the detected pair without changing defaults or device state", async () => {
    const format = { sampleRateHz: 48000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 };
    const devices = [
      { id: "capture-vb", name: "CABLE Output (VB-Audio Virtual Cable)", direction: "capture" as const, state: "active" as const, defaultRoles: [], format, periods: { default100ns: 100000, minimum100ns: 30000 } },
      { id: "render-vb", name: "CABLE Input (VB-Audio Virtual Cable)", direction: "render" as const, state: "active" as const, defaultRoles: [], format, periods: { default100ns: 100000, minimum100ns: 30000 } },
    ];
    render(<App backend={{ ...connectedPreviewBackend(), listDevices: async () => devices }} />);
    const button = await screen.findByRole("button", { name: "Select VB-Cable loopback pair" });
    expect(button).toHaveProperty("disabled", false);
    fireEvent.click(button);
    await waitFor(() => expect((screen.getByRole("combobox", { name: "Native capture endpoint" }) as HTMLSelectElement).value).toBe("capture-vb"));
    expect((screen.getByRole("combobox", { name: "Native render endpoint" }) as HTMLSelectElement).value).toBe("render-vb");
    expect(screen.getByRole("option", { name: /CABLE Input.*48000 Hz.*2 ch/ })).toBeTruthy();
    expect(JSON.parse(window.localStorage.getItem("audiorouter.ui.endpoint-binding.demo-session") ?? "null")).toEqual({ captureEndpointId: "capture-vb", renderEndpointId: "render-vb" });
    expect(screen.getByText("VB-Cable pair selected. Review the graph, then prepare and start the session.")).toBeTruthy();
  });

  it("selects VB-Cable capture without silently selecting a render monitor", async () => {
    const format = { sampleRateHz: 48000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 };
    const devices = [
      { id: "capture-vb", name: "CABLE Output (VB-Audio Virtual Cable)", direction: "capture" as const, state: "active" as const, defaultRoles: [], format, periods: { default100ns: 100000, minimum100ns: 30000 } },
      { id: "render-monitor", name: "Headphones", direction: "render" as const, state: "active" as const, defaultRoles: ["console" as const], format, periods: { default100ns: 100000, minimum100ns: 30000 } },
    ];
    render(<App backend={{ ...connectedPreviewBackend(), listDevices: async () => devices }} />);
    fireEvent.click(await screen.findByRole("button", { name: "Select VB-Cable capture" }));
    await waitFor(() => expect((screen.getByRole("combobox", { name: "Native capture endpoint" }) as HTMLSelectElement).value).toBe("capture-vb"));
    expect((screen.getByRole("combobox", { name: "Native render endpoint" }) as HTMLSelectElement).value).toBe("");
    expect(screen.getByText("VB-Cable capture selected. Choose the physical render output, then prepare and start the session.")).toBeTruthy();
  });

  it("keeps a physical render ownership failure actionable", async () => {
    const format = { sampleRateHz: 48000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 };
    const devices = [
      { id: "capture-vb", name: "CABLE Output (VB-Audio Virtual Cable)", direction: "capture" as const, state: "active" as const, defaultRoles: [], format, periods: { default100ns: 100000, minimum100ns: 30000 } },
      { id: "render-focusrite", name: "Speakers (Focusrite USB Audio)", direction: "render" as const, state: "active" as const, defaultRoles: ["console" as const], format, periods: { default100ns: 100000, minimum100ns: 30000 } },
    ];
    const prepareNativeEndpoint = vi.fn(async () => {
      throw new AudioRouterRpcError({
        code: -32010,
        message: "IAudioClient::Initialize(render) failed.",
        data: { code: "deviceInUse", fieldPath: null, resourceIds: ["render-focusrite"], retryable: true, remediation: "Identify the owning stream, select another endpoint, or close it and retry.", hresult: 0x8889000A },
      });
    });
    render(<App backend={{ ...connectedPreviewBackend(), listDevices: async () => devices, prepareNativeEndpoint }} />);
    fireEvent.click(await screen.findByRole("button", { name: "Select VB-Cable capture" }));
    fireEvent.change(screen.getByRole("combobox", { name: "Native render endpoint" }), { target: { value: "render-focusrite" } });
    fireEvent.click(screen.getByRole("button", { name: "Prepare native endpoints" }));
    await waitFor(() => expect(prepareNativeEndpoint).toHaveBeenCalledWith("demo-session", "capture-vb", "render-focusrite"));
    expect(await screen.findByText(/\[deviceInUse, HRESULT 0x8889000A\]/)).toBeTruthy();
    expect(screen.getByText(/select another endpoint, or close it and retry/i)).toBeTruthy();
  });

  it("detaches a stopped native worker before deliberate endpoint replacement", async () => {
    const detachNativeEndpoint = vi.fn(async () => ({ sessionId: "demo-session", state: "detached" as const }));
    render(<App backend={{ ...connectedPreviewBackend(), detachNativeEndpoint }} />);
    fireEvent.click(await screen.findByRole("button", { name: "Detach stopped worker" }));
    await waitFor(() => expect(detachNativeEndpoint).toHaveBeenCalledWith("demo-session"));
    expect(await screen.findByText(/Native worker detached; select new endpoints/i)).toBeTruthy();
  });

  it("prevents duplicate native endpoint preparation while the first request is pending", async () => {
    const format = { sampleRateHz: 48000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 };
    const devices = [
      { id: "capture-vb", name: "CABLE Output (VB-Audio Virtual Cable)", direction: "capture" as const, state: "active" as const, defaultRoles: [], format, periods: { default100ns: 100000, minimum100ns: 30000 } },
      { id: "render-vb", name: "CABLE Input (VB-Audio Virtual Cable)", direction: "render" as const, state: "active" as const, defaultRoles: [], format, periods: { default100ns: 100000, minimum100ns: 30000 } },
    ];
    let releasePrepare!: (value: { sessionId: string; state: "configured-stopped"; captureEndpointId: string; renderEndpointId: string }) => void;
    const result = new Promise<{ sessionId: string; state: "configured-stopped"; captureEndpointId: string; renderEndpointId: string }>((resolve) => { releasePrepare = resolve; });
    const prepareNativeEndpoint = vi.fn(() => result);
    render(<App backend={{ ...connectedPreviewBackend(), listDevices: async () => devices, prepareNativeEndpoint }} />);
    fireEvent.click(await screen.findByRole("button", { name: "Select VB-Cable loopback pair" }));
    fireEvent.click(screen.getByRole("button", { name: "Prepare native endpoints" }));
    await screen.findByText("Preparing exact endpoints in stopped state...");
    fireEvent.click(screen.getByRole("button", { name: "Prepare native endpoints" }));
    expect(prepareNativeEndpoint).toHaveBeenCalledTimes(1);
    releasePrepare({ sessionId: "demo-session", state: "configured-stopped", captureEndpointId: "capture-vb", renderEndpointId: "render-vb" });
    await waitFor(() => expect(screen.getByText(/Prepared configured-stopped/)).toBeTruthy());
  });
});

describe("keyboard connection dialog", () => {
  it("adds a real processor draft through the canvas drag-and-drop shelf", async () => {
    render(<App backend={connectedPreviewBackend()} />);
    const dropSource = await screen.findByRole("button", { name: /^Gain$/ });
    const canvas = screen.getByLabelText("Signal-flow graph");
    const values = new Map<string, string>();
    const dataTransfer = {
      types: ["application/x-audiorouter-library-kind"],
      effectAllowed: "copy",
      setData: (type: string, value: string) => values.set(type, value),
      getData: (type: string) => values.get(type) ?? "",
    };
    fireEvent.dragStart(dropSource, { dataTransfer });
    fireEvent.drop(canvas, { dataTransfer, clientX: 240, clientY: 180 });
    await waitFor(() => expect(screen.getByText("Gain 1 added to the draft. Review and plan the changes before committing.")).toBeTruthy());
    expect(JSON.parse(window.localStorage.getItem("audiorouter.ui.layout.demo-session") ?? "null")).toMatchObject({ "gain-1": { x: 0, y: 0 } });
  });

  it("rejects canvas library drops while disconnected", async () => {
    render(<App backend={createDisconnectedBackend()} />);
    const dropSource = await screen.findByRole("button", { name: /^Gain$/ });
    expect(dropSource).toHaveProperty("disabled", true);
    const canvas = screen.getByLabelText("Signal-flow graph");
    const dataTransfer = {
      types: ["application/x-audiorouter-library-kind"],
      setData: vi.fn(),
      getData: (type: string) => type === "application/x-audiorouter-library-kind" ? "gain" : "",
    };
    fireEvent.drop(canvas, { dataTransfer });
    expect(screen.queryByText("Gain 1 added to the draft. Review and plan the changes before committing.")).toBeNull();
    expect(await screen.findByText("Connect the backend before changing the draft.")).toBeTruthy();
  });

  it("binds only explicitly selected active endpoints and preserves the no-defaults boundary", async () => {
    const capture = {
      id: "capture-active",
      name: "Active capture",
      direction: "capture" as const,
      state: "active" as const,
      defaultRoles: [],
      format: { sampleRateHz: 48000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 },
      periods: { default100ns: 100000, minimum100ns: 30000 },
    };
    const renderDevice = { ...capture, id: "render-active", name: "Active render", direction: "render" as const };
    const inactive = { id: "capture-inactive", name: "Inactive capture", direction: "capture" as const, state: "unplugged" as const, defaultRoles: [] };
    const prepareNativeEndpoint = vi.fn(async (sessionId: string, captureEndpointId: string, renderEndpointId: string) => ({
      sessionId,
      state: "configured-stopped" as const,
      captureEndpointId,
      renderEndpointId,
    }));
    const startSession = vi.fn(async () => ({ sessionId: "demo-session", state: "running" as const, runtime: "fake" as const, generation: 1 }));
    const backend = { ...connectedPreviewBackend(), listDevices: async () => [inactive, capture, renderDevice], prepareNativeEndpoint, startSession };

    render(<App backend={backend} />);

    const endpointPanel = screen.getByRole("region", { name: "Endpoint binding" });
    const captureSelect = await screen.findByRole("combobox", { name: "Native capture endpoint" });
    const renderSelect = screen.getByRole("combobox", { name: "Native render endpoint" });
    expect(within(captureSelect).queryByRole("option", { name: /Inactive capture/ })).toBeNull();
    fireEvent.change(captureSelect, { target: { value: "capture-active" } });
    fireEvent.change(renderSelect, { target: { value: "render-active" } });
    expect((captureSelect as HTMLSelectElement).value).toBe("capture-active");
    expect((renderSelect as HTMLSelectElement).value).toBe("render-active");

    fireEvent.click(screen.getByRole("button", { name: "Prepare native endpoints" }));
    await waitFor(() => expect(prepareNativeEndpoint).toHaveBeenCalledWith("demo-session", "capture-active", "render-active"));
    expect(await screen.findByText("Prepared configured-stopped; start the session to activate audio.")).toBeTruthy();
    expect(screen.getByText(/Endpoint defaults, volume, and mute are never changed/)).toBeTruthy();
    fireEvent.click(within(endpointPanel).getByRole("button", { name: "Start session" }));
    await waitFor(() => expect(startSession).toHaveBeenCalledWith("demo-session", expect.any(String)));
  });

  it("restores endpoint choices only when the exact IDs are still active", async () => {
    window.localStorage.setItem("audiorouter.ui.endpoint-binding.demo-session", JSON.stringify({ captureEndpointId: "capture-saved", renderEndpointId: "render-saved" }));
    const capture = { id: "capture-saved", name: "Saved capture", direction: "capture" as const, state: "active" as const, defaultRoles: [], format: { sampleRateHz: 48000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 }, periods: { default100ns: 100000, minimum100ns: 30000 } };
    const renderDevice = { ...capture, id: "render-saved", name: "Saved render", direction: "render" as const };
    const backend = { ...connectedPreviewBackend(), listDevices: async () => [capture, renderDevice] };
    render(<App backend={backend} />);
    await waitFor(() => expect((screen.getByRole("combobox", { name: "Native capture endpoint" }) as HTMLSelectElement).value).toBe("capture-saved"));
    expect((screen.getByRole("combobox", { name: "Native render endpoint" }) as HTMLSelectElement).value).toBe("render-saved");
    window.localStorage.removeItem("audiorouter.ui.endpoint-binding.demo-session");
  });

  it("does not silently replace a missing saved endpoint binding", async () => {
    window.localStorage.setItem("audiorouter.ui.endpoint-binding.demo-session", JSON.stringify({ captureEndpointId: "capture-gone", renderEndpointId: "render-gone" }));
    const capture = { id: "capture-current", name: "Current capture", direction: "capture" as const, state: "active" as const, defaultRoles: [], format: { sampleRateHz: 48000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 }, periods: { default100ns: 100000, minimum100ns: 30000 } };
    const renderDevice = { ...capture, id: "render-current", name: "Current render", direction: "render" as const };
    const prepareNativeEndpoint = vi.fn(async (sessionId: string, captureEndpointId: string, renderEndpointId: string) => ({ sessionId, state: "configured-stopped" as const, captureEndpointId, renderEndpointId }));
    const backend = { ...connectedPreviewBackend(), listDevices: async () => [capture, renderDevice], prepareNativeEndpoint };
    render(<App backend={backend} />);
    const captureSelect = await screen.findByRole("combobox", { name: "Native capture endpoint" });
    const renderSelect = screen.getByRole("combobox", { name: "Native render endpoint" });
    await waitFor(() => expect((captureSelect as HTMLSelectElement).value).toBe(""));
    expect((renderSelect as HTMLSelectElement).value).toBe("");
    expect(screen.getByText("Saved capture endpoint is unavailable. Select a replacement deliberately.")).toBeTruthy();
    expect(screen.getByText("Saved render endpoint is unavailable. Select a replacement deliberately.")).toBeTruthy();
    fireEvent.change(captureSelect, { target: { value: "capture-current" } });
    fireEvent.change(renderSelect, { target: { value: "render-current" } });
    expect(JSON.parse(window.localStorage.getItem("audiorouter.ui.endpoint-binding.demo-session") ?? "null")).toEqual({ captureEndpointId: "capture-current", renderEndpointId: "render-current" });
    fireEvent.click(screen.getByRole("button", { name: "Prepare native endpoints" }));
    await waitFor(() => expect(prepareNativeEndpoint).toHaveBeenCalledWith("demo-session", "capture-current", "render-current"));
    window.localStorage.removeItem("audiorouter.ui.endpoint-binding.demo-session");
  });

  it("opens with focus, wraps focus, retains validation errors, and restores focus", async () => {
    const backend = connectedPreviewBackend();
    render(<App backend={backend} />);

    const opener = screen.getByRole("button", { name: "Keyboard connection dialog" });
    opener.focus();
    fireEvent.click(opener);

    const dialog = await screen.findByRole("dialog", { name: "Keyboard connection" });
    const source = within(dialog).getByRole("combobox", { name: "Keyboard source output port" });
    expect(document.activeElement).toBe(source);

    const close = within(dialog).getByRole("button", { name: "Close keyboard connection dialog" });
    const cancel = within(dialog).getByRole("button", { name: "Cancel" });
    cancel.focus();
    fireEvent.keyDown(window, { key: "Tab" });
    expect(document.activeElement).toBe(close);
    close.focus();
    fireEvent.keyDown(window, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(cancel);

    const add = within(dialog).getByRole("button", { name: "Add connection to draft" });
    fireEvent.click(add);
    expect(screen.getByRole("dialog", { name: "Keyboard connection" })).toBeTruthy();
    expect(screen.getByText("Choose an output and input port first.")).toBeTruthy();

    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Keyboard connection" })).toBeNull());
    await waitFor(() => expect(document.activeElement).toBe(opener));
  });

  it("closes after a valid draft connection", async () => {
    const backend = connectedPreviewBackend();
    render(<App backend={backend} />);

    fireEvent.click(screen.getByRole("button", { name: "Keyboard connection dialog" }));
    const dialog = await screen.findByRole("dialog", { name: "Keyboard connection" });
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Keyboard source output port" }), { target: { value: "mic::out" } });
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Keyboard destination input port" }), { target: { value: "voice::in" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Add connection to draft" }));

    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Keyboard connection" })).toBeNull());
    expect(screen.getByText("Connection added to the draft. Review and plan the changes before committing.")).toBeTruthy();
  });

  it("plans a keyboard-created connection through the graph backend", async () => {
    const planGraph = vi.fn(async (candidate: typeof demoSession) => ({
      planId: "connection-plan",
      baseRevision: candidate.revision,
      expiresInMs: 30_000,
      diff: [],
      affectedDestinations: [],
      warnings: [],
      requiredScopes: ["graphWrite"],
    }));
    const commitGraph = vi.fn(async () => ({ sessionId: demoSession.id, revision: demoSession.revision + 1 }));
    render(<App backend={{ ...connectedPreviewBackend(), planGraph, commitGraph }} />);

    fireEvent.click(screen.getByRole("button", { name: "Keyboard connection dialog" }));
    const dialog = await screen.findByRole("dialog", { name: "Keyboard connection" });
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Keyboard source output port" }), { target: { value: "mic::out" } });
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Keyboard destination input port" }), { target: { value: "voice::in" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Add connection to draft" }));
    fireEvent.click(screen.getByRole("button", { name: "Plan changes" }));

    await waitFor(() => expect(commitGraph).toHaveBeenCalledWith("connection-plan", demoSession.revision, expect.any(String)));
    expect(planGraph).toHaveBeenCalledWith(expect.objectContaining({
      edges: expect.arrayContaining([
        expect.objectContaining({ sourceNode: "mic", sourcePort: "out", destinationNode: "voice", destinationPort: "in" }),
      ]),
    }));
  });

  it("renders named canvas handles for connected graph editing", async () => {
    render(<App backend={connectedPreviewBackend()} />);

    expect(await screen.findByLabelText("Microphone out output")).toBeTruthy();
    expect(await screen.findByLabelText("Voice gain in input")).toBeTruthy();
  });

  it("renders the read-only persisted recovery checkpoint panel", async () => {
    const backend = {
      ...createDisconnectedBackend(),
      connected: true,
      listRecordingRecovery: async () => ({
        items: [{ recordingId: "take-recovery", status: "invalid" as const }],
        nextCursor: null,
      }),
    };
    render(<App backend={backend} />);
    expect(await screen.findByRole("heading", { name: "Recovery checkpoints" })).toBeTruthy();
    expect(await screen.findByText("take-recovery")).toBeTruthy();
    expect(await screen.findByText(/invalid/)).toBeTruthy();
    expect(screen.getByText("This list is read-only. Recovery inspection does not open, repair, play, or delete audio files.")).toBeTruthy();
  });

  it("keeps the newest recording inspection result when responses arrive out of order", async () => {
    const recording: RecordingRow = { id: "inspection-take", sessionId: demoSession.id, recorderId: "recorder-1", path: "C:\\Audio\\inspection.wav", format: "wav", channels: 2, sampleRate: 48000, frames: 480, fileBytes: 1000, startTime: "2026-09-14T01:00:00Z", state: "completed", missing: false, title: null, artist: null, comment: null, dither: true, conversion: "targetSampleRate=48000;channels=2;format=wav" };
    let releaseOld!: (value: { preview: { status: string } }) => void;
    const oldResult = new Promise<{ preview: { status: string } }>((resolve) => { releaseOld = resolve; });
    const previewRecording = vi.fn().mockReturnValueOnce(oldResult).mockResolvedValueOnce({ preview: { status: "new" } });
    const backend = { ...connectedPreviewBackend(), listRecordings: async () => [recording], previewRecording };
    render(<App backend={backend} />);
    const previewButtons = await screen.findAllByRole("button", { name: "Preview" });
    fireEvent.click(previewButtons[0]);
    fireEvent.click(previewButtons[0]);
    expect(await screen.findByText("new recording preview loaded.")).toBeTruthy();
    releaseOld({ preview: { status: "old" } });
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(screen.queryByText("old recording preview loaded.")).toBeNull();
  });

  it("exposes revisioned virtual-route editing in the connected UI", async () => {
    const replaceVirtualRoutes = vi.fn(async (baseRevision: number, routes: unknown[]) => ({
      state: "applied" as const,
      revision: baseRevision + 1,
      routes: routes as [],
    }));
    const backend = {
      ...createDisconnectedBackend(),
      connected: true,
      listVirtualRoutes: async () => ({ revision: 2, routes: [] }),
      replaceVirtualRoutes,
    };
    render(<App backend={backend} />);
    expect(await screen.findByRole("heading", { name: "Virtual-bus routes" })).toBeTruthy();
    expect((screen.getByRole("textbox", { name: "Virtual-route base revision" }) as HTMLInputElement).value).toBe("2");
    fireEvent.click(screen.getByRole("button", { name: "Replace routes" }));
    await waitFor(() => expect(replaceVirtualRoutes).toHaveBeenCalledWith(2, [], expect.any(String)));
    expect(await screen.findByText("Virtual routes applied at revision 3.")).toBeTruthy();
  });

  it("refreshes virtual-route inventory when the same backend reconnects", async () => {
    const listVirtualRoutes = vi.fn(async () => ({ revision: 4, routes: [] }));
    const backend = { ...createDisconnectedBackend(), connected: false, listVirtualRoutes };
    const { rerender } = render(<App backend={backend} />);
    await waitFor(() => expect(listVirtualRoutes).toHaveBeenCalledTimes(0));
    backend.connected = true;
    rerender(<App backend={backend} />);
    await waitFor(() => expect(listVirtualRoutes).toHaveBeenCalledTimes(1));
    expect((screen.getByRole("textbox", { name: "Virtual-route base revision" }) as HTMLInputElement).value).toBe("4");
  });

  it("clears a virtual-device selection that disappears from inventory", async () => {
    const makeDevice = (id: string, name: string): VirtualDeviceInfo => ({ id, name, direction: "bidirectional", channels: 2, enabled: true, availability: { status: "unavailable", reason: "managed driver unavailable" }, endpointIds: { render: null, capture: null }, capabilities: { render: false, capture: false, channels: 2 }, privilege: "deviceAdministration", restartRequired: false, clientImpacts: [], leaseOwner: null });
    const listVirtualDevices = vi.fn().mockResolvedValueOnce([makeDevice("old-bus", "Old bus")]).mockResolvedValueOnce([makeDevice("new-bus", "New bus")]);
    const backend = { ...connectedPreviewBackend(), listVirtualDevices };
    render(<App backend={backend} />);
    const panel = await screen.findByRole("region", { name: "Virtual-device lifecycle" });
    await waitFor(() => expect(listVirtualDevices).toHaveBeenCalledTimes(1));
    fireEvent.change(within(panel).getByRole("combobox", { name: "Virtual-device action" }), { target: { value: "rename" } });
    expect((within(panel).getByRole("combobox", { name: "Existing virtual-device target" }) as HTMLSelectElement).value).toBe("old-bus");
    fireEvent.click(within(panel).getByRole("button", { name: "Refresh" }));
    await waitFor(() => expect(listVirtualDevices).toHaveBeenCalledTimes(2));
    expect((within(panel).getByRole("combobox", { name: "Existing virtual-device target" }) as HTMLSelectElement).value).toBe("new-bus");
  });

  it("prevents duplicate virtual-route replacement while the first request is pending", async () => {
    let releaseReplace!: (value: { state: "applied"; revision: number; routes: never[] }) => void;
    const result = new Promise<{ state: "applied"; revision: number; routes: never[] }>((resolve) => { releaseReplace = resolve; });
    const replaceVirtualRoutes = vi.fn(() => result);
    const backend = {
      ...createDisconnectedBackend(),
      connected: true,
      listVirtualRoutes: async () => ({ revision: 2, routes: [] }),
      replaceVirtualRoutes,
    };
    render(<App backend={backend} />);
    const panel = await screen.findByRole("region", { name: "Virtual-bus routes" });
    const replace = within(panel).getByRole("button", { name: "Replace routes" });
    fireEvent.click(replace);
    await waitFor(() => expect((replace as HTMLButtonElement).disabled).toBe(true));
    fireEvent.click(replace);
    expect(replaceVirtualRoutes).toHaveBeenCalledTimes(1);
    releaseReplace({ state: "applied", revision: 3, routes: [] });
    await waitFor(() => expect(within(panel).getByText("Virtual routes applied at revision 3.")).toBeTruthy());
  });

  it("prevents duplicate graph planning while the first request is pending", async () => {
    let releasePlan!: (value: { planId: string; baseRevision: number; expiresInMs: number; diff: never[]; warnings: never[]; affectedDestinations: string[]; requiredScopes: string[] }) => void;
    const result = new Promise<{ planId: string; baseRevision: number; expiresInMs: number; diff: never[]; warnings: never[]; affectedDestinations: string[]; requiredScopes: string[] }>((resolve) => { releasePlan = resolve; });
    const planGraph = vi.fn(() => result);
    const commitGraph = vi.fn(async () => ({ sessionId: demoSession.id, revision: demoSession.revision + 1 }));
    render(<App backend={{ ...connectedPreviewBackend(), planGraph, commitGraph }} />);
    const plan = screen.getByRole("button", { name: "Plan changes" });
    fireEvent.click(plan);
    await waitFor(() => expect(planGraph).toHaveBeenCalledTimes(1));
    fireEvent.click(plan);
    expect(planGraph).toHaveBeenCalledTimes(1);
    releasePlan({ planId: "graph-plan", baseRevision: demoSession.revision, expiresInMs: 30000, diff: [], warnings: [], affectedDestinations: [], requiredScopes: [] });
    await waitFor(() => expect(commitGraph).toHaveBeenCalledTimes(1));
  });

  it("creates an unarmed recorder with the explicit UI configuration", async () => {
    const createRecorder = vi.fn(async (params: { recorderId: string }) => ({
      sessionId: demoSession.id,
      nodeId: null,
      recorderId: params.recorderId,
      format: "wavPcm24" as const,
      path: "C:\\Audio\\take.wav",
      state: "idle" as const,
      armed: false as const,
    }));
    const backend = { ...connectedPreviewBackend(), createRecorder };
    render(<App backend={backend} />);
    fireEvent.change(await screen.findByRole("textbox", { name: "Recorder ID" }), { target: { value: "voice-take" } });
    const dither = screen.getByRole("checkbox", { name: "TPDF dither" });
    expect((dither as HTMLInputElement).checked).toBe(true);
    fireEvent.click(dither);
    fireEvent.click(screen.getByRole("button", { name: "Create recorder" }));
    await waitFor(() => expect(createRecorder).toHaveBeenCalledWith(expect.objectContaining({ recorderId: "voice-take", format: "wavPcm24", channels: 2, sampleRate: 48000, sequence: 1, dither: false })));
    expect(await screen.findByText(/Recorder voice-take created unarmed/)).toBeTruthy();
  });

  it("disables dither for Float32 recorder output", async () => {
    const createRecorder = vi.fn(async (params: { recorderId: string }) => ({
      sessionId: demoSession.id,
      nodeId: null,
      recorderId: params.recorderId,
      format: "wavFloat32" as const,
      path: "C:\\Audio\\float.wav",
      state: "idle" as const,
      armed: false as const,
    }));
    const backend = { ...connectedPreviewBackend(), createRecorder };
    render(<App backend={backend} />);
    fireEvent.change(await screen.findByRole("combobox", { name: "Recorder format" }), { target: { value: "wavFloat32" } });
    const dither = screen.getByRole("checkbox", { name: "TPDF dither" }) as HTMLInputElement;
    expect(dither.checked).toBe(false);
    expect(dither.disabled).toBe(true);
    expect(await screen.findByText("Float32 output is not dithered.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Create recorder" }));
    await waitFor(() => expect(createRecorder).toHaveBeenCalledWith(expect.objectContaining({ format: "wavFloat32", dither: false })));
  });

  it("hydrates the recorder panel from the authoritative live state", async () => {
    const backend = {
      ...connectedPreviewBackend(),
      listRecorders: async () => [{ sessionId: demoSession.id, nodeId: "recorder-node", state: "recording" as const, lastFrame: 480 }],
    };
    render(<App backend={backend} />);
    const panel = await screen.findByRole("region", { name: "Recorder" });
    await waitFor(() => expect(within(panel).getByText("recording")).toBeTruthy());
    expect(within(panel).getByText("Attached recorder node: recorder-node")).toBeTruthy();
    expect(within(panel).getByText("Backend last frame: 480")).toBeTruthy();
  });

  it("clears recorder state when the selected session has no live recorder", async () => {
    const activeBackend = { ...connectedPreviewBackend(), listRecorders: async () => [{ sessionId: demoSession.id, state: "recording" as const, lastFrame: 480 }] };
    const view = render(<App backend={activeBackend} />);
    const panel = await screen.findByRole("region", { name: "Recorder" });
    await waitFor(() => expect(within(panel).getByText("recording")).toBeTruthy());
    view.rerender(<App backend={{ ...connectedPreviewBackend(), listRecorders: async () => [] }} />);
    await waitFor(() => expect(within(screen.getByRole("region", { name: "Recorder" })).getByText("idle")).toBeTruthy());
    expect(screen.queryByText("Backend last frame: 480")).toBeNull();
  });

  it("shows a failed recorder state when a lifecycle operation is rejected", async () => {
    const backend = { ...connectedPreviewBackend(), armRecorder: async () => { throw new Error("disk full"); } };
    render(<App backend={backend} />);
    const panel = await screen.findByRole("region", { name: "Recorder" });
    fireEvent.click(within(panel).getByRole("button", { name: "Arm" }));
    await waitFor(() => expect(within(panel).getByText("failed")).toBeTruthy());
    expect(within(panel).getByText("disk full")).toBeTruthy();
  });

  it("does not guess idle when recorder state cannot be read", async () => {
    const backend = { ...connectedPreviewBackend(), listRecorders: async () => { throw new Error("control pipe unavailable"); } };
    render(<App backend={backend} />);
    const panel = await screen.findByRole("region", { name: "Recorder" });
    await waitFor(() => expect(within(panel).getByText("unavailable")).toBeTruthy());
  });

  it("offers bounded slider and precise entry for numeric processor parameters", async () => {
    const processor: ProcessorDescriptor = {
      id: "gain",
      version: 1,
      category: "effect",
      availability: { status: "available" },
      latencySamples: 0,
      parameters: [{ name: "gainDb", type: "number", unit: "dB", minimum: -60, maximum: 24, default: 0 }],
    };
    const backend = { ...createDisconnectedBackend(), connected: true, listProcessors: async () => [processor] };
    render(<App backend={backend} />);
    fireEvent.click(await screen.findByLabelText("Voice gain, gain"));

    const slider = await screen.findByRole("slider", { name: "gainDb slider" });
    const precise = screen.getByRole("spinbutton", { name: "gainDb precise value" });
    expect(slider.getAttribute("min")).toBe("-60");
    expect(slider.getAttribute("max")).toBe("24");
    expect((precise as HTMLInputElement).value).toBe("0");
    fireEvent.change(slider, { target: { value: "-6" } });
    expect((precise as HTMLInputElement).value).toBe("-6");
    expect(screen.getByText("Draft effect: gainDb: 0 → -6. Plan changes to validate and commit.")).toBeTruthy();
  });

  it("renders the backend-derived EQ response for a draft EQ node", async () => {
    const processorResponse = vi.fn(async () => ({ frequenciesHz: [20, 1000, 20000], magnitudeDb: [0, -6, 0] }));
    render(<App backend={{ ...connectedPreviewBackend(), processorResponse }} />);
    fireEvent.click(await screen.findByRole("button", { name: "Parametric EQ, Effect" }));

    expect(await screen.findByRole("img", { name: "Parametric EQ magnitude response" })).toBeTruthy();
    expect(processorResponse).toHaveBeenCalledWith(expect.objectContaining({ sampleRateHz: 48000, frequenciesHz: expect.any(Array), bands: expect.any(Array) }));
  });

  it("commits a dropped gate parameter through the graph backend", async () => {
    const planGraph = vi.fn(async (candidate: typeof demoSession) => ({
      planId: "gate-plan",
      baseRevision: candidate.revision,
      expiresInMs: 30_000,
      diff: [],
      affectedDestinations: [],
      warnings: [],
      requiredScopes: ["graphWrite"],
    }));
    const commitGraph = vi.fn(async () => ({ sessionId: demoSession.id, revision: demoSession.revision + 1 }));
    const gate = {
      id: "gate",
      version: 1,
      category: "dynamics" as const,
      availability: { status: "available" as const },
      latencySamples: 0,
      parameters: [
        { name: "thresholdDb", type: "number" as const, unit: "dB", minimum: -80, maximum: 0, default: -45 },
      ],
    };
    const backend = {
      ...connectedPreviewBackend(),
      listProcessors: async () => [gate],
      planGraph,
      commitGraph,
    };
    render(<App backend={backend} />);

    const dropSource = await screen.findByRole("button", { name: /^Gate$/ });
    const canvas = screen.getByLabelText("Signal-flow graph");
    const values = new Map<string, string>();
    const dataTransfer = {
      types: ["application/x-audiorouter-library-kind"],
      effectAllowed: "copy",
      setData: (type: string, value: string) => values.set(type, value),
      getData: (type: string) => values.get(type) ?? "",
    };
    fireEvent.dragStart(dropSource, { dataTransfer });
    fireEvent.drop(canvas, { dataTransfer });
    fireEvent.click(await screen.findByLabelText("Gate 1, gate"));
    fireEvent.change(await screen.findByRole("spinbutton", { name: "thresholdDb precise value" }), { target: { value: "-30" } });
    fireEvent.click(screen.getByRole("button", { name: "Plan changes" }));

    await waitFor(() => expect(commitGraph).toHaveBeenCalledWith("gate-plan", demoSession.revision, expect.any(String)));
    expect(planGraph).toHaveBeenCalledWith(expect.objectContaining({
      nodes: expect.arrayContaining([
        expect.objectContaining({ kind: "gate", parameters: expect.objectContaining({ thresholdDb: -30 }) }),
      ]),
    }));
    expect(await screen.findByText("Committed revision 8. Reconnect to refresh the authoritative view.")).toBeTruthy();
  });

  it("commits a dropped pitch parameter through the graph backend", async () => {
    const planGraph = vi.fn(async (candidate: typeof demoSession) => ({
      planId: "pitch-plan",
      baseRevision: candidate.revision,
      expiresInMs: 30_000,
      diff: [],
      affectedDestinations: [],
      warnings: [],
      requiredScopes: ["graphWrite"],
    }));
    const commitGraph = vi.fn(async () => ({ sessionId: demoSession.id, revision: demoSession.revision + 1 }));
    const pitch = {
      id: "pitch",
      version: 1,
      category: "pitch" as const,
      availability: { status: "available" as const },
      latencySamples: 1024,
      parameters: [
        { name: "semitones", type: "number" as const, unit: "st", minimum: -12, maximum: 12, default: 0 },
      ],
    };
    const backend = { ...connectedPreviewBackend(), listProcessors: async () => [pitch], planGraph, commitGraph };
    render(<App backend={backend} />);

    const dropSource = await screen.findByRole("button", { name: /^Pitch shift$/ });
    const canvas = screen.getByLabelText("Signal-flow graph");
    const values = new Map<string, string>();
    const dataTransfer = {
      types: ["application/x-audiorouter-library-kind"],
      effectAllowed: "copy",
      setData: (type: string, value: string) => values.set(type, value),
      getData: (type: string) => values.get(type) ?? "",
    };
    fireEvent.dragStart(dropSource, { dataTransfer });
    fireEvent.drop(canvas, { dataTransfer });
    fireEvent.click(await screen.findByLabelText("Pitch shift 1, pitch"));
    fireEvent.change(await screen.findByRole("spinbutton", { name: "semitones precise value" }), { target: { value: "5" } });
    fireEvent.click(screen.getByRole("button", { name: "Plan changes" }));

    await waitFor(() => expect(commitGraph).toHaveBeenCalledWith("pitch-plan", demoSession.revision, expect.any(String)));
    expect(planGraph).toHaveBeenCalledWith(expect.objectContaining({
      nodes: expect.arrayContaining([
        expect.objectContaining({ kind: "pitch", parameters: expect.objectContaining({ semitones: 5 }) }),
      ]),
    }));
  });

  it("commits a dropped compressor parameter through the graph backend", async () => {
    const planGraph = vi.fn(async (candidate: typeof demoSession) => ({
      planId: "compressor-plan",
      baseRevision: candidate.revision,
      expiresInMs: 30_000,
      diff: [],
      affectedDestinations: [],
      warnings: [],
      requiredScopes: ["graphWrite"],
    }));
    const commitGraph = vi.fn(async () => ({ sessionId: demoSession.id, revision: demoSession.revision + 1 }));
    const compressor = {
      id: "compressor",
      version: 1,
      category: "dynamics" as const,
      availability: { status: "available" as const },
      latencySamples: 0,
      parameters: [
        { name: "ratio", type: "number" as const, unit: ":1", minimum: 1, maximum: 20, default: 3 },
      ],
    };
    const backend = { ...connectedPreviewBackend(), listProcessors: async () => [compressor], planGraph, commitGraph };
    render(<App backend={backend} />);

    const dropSource = await screen.findByRole("button", { name: /^Compressor$/ });
    const canvas = screen.getByLabelText("Signal-flow graph");
    const values = new Map<string, string>();
    const dataTransfer = {
      types: ["application/x-audiorouter-library-kind"],
      effectAllowed: "copy",
      setData: (type: string, value: string) => values.set(type, value),
      getData: (type: string) => values.get(type) ?? "",
    };
    fireEvent.dragStart(dropSource, { dataTransfer });
    fireEvent.drop(canvas, { dataTransfer });
    fireEvent.click(await screen.findByLabelText("Compressor 1, compressor"));
    fireEvent.change(await screen.findByRole("spinbutton", { name: "ratio precise value" }), { target: { value: "6" } });
    fireEvent.click(screen.getByRole("button", { name: "Plan changes" }));

    await waitFor(() => expect(commitGraph).toHaveBeenCalledWith("compressor-plan", demoSession.revision, expect.any(String)));
    expect(planGraph).toHaveBeenCalledWith(expect.objectContaining({
      nodes: expect.arrayContaining([
        expect.objectContaining({ kind: "compressor", parameters: expect.objectContaining({ ratio: 6 }) }),
      ]),
    }));
  });

  it("commits a dropped limiter parameter through the graph backend", async () => {
    const planGraph = vi.fn(async (candidate: typeof demoSession) => ({
      planId: "limiter-plan",
      baseRevision: candidate.revision,
      expiresInMs: 30_000,
      diff: [],
      affectedDestinations: [],
      warnings: [],
      requiredScopes: ["graphWrite"],
    }));
    const commitGraph = vi.fn(async () => ({ sessionId: demoSession.id, revision: demoSession.revision + 1 }));
    const limiter = {
      id: "limiter",
      version: 1,
      category: "dynamics" as const,
      availability: { status: "available" as const },
      latencySamples: 240,
      parameters: [
        { name: "ceilingDb", type: "number" as const, unit: "dBFS", minimum: -12, maximum: 0, default: -1 },
      ],
    };
    const backend = { ...connectedPreviewBackend(), listProcessors: async () => [limiter], planGraph, commitGraph };
    render(<App backend={backend} />);

    const dropSource = await screen.findByRole("button", { name: /^Limiter$/ });
    const canvas = screen.getByLabelText("Signal-flow graph");
    const values = new Map<string, string>();
    const dataTransfer = {
      types: ["application/x-audiorouter-library-kind"],
      effectAllowed: "copy",
      setData: (type: string, value: string) => values.set(type, value),
      getData: (type: string) => values.get(type) ?? "",
    };
    fireEvent.dragStart(dropSource, { dataTransfer });
    fireEvent.drop(canvas, { dataTransfer });
    fireEvent.click(await screen.findByLabelText("Limiter 1, limiter"));
    fireEvent.change(await screen.findByRole("spinbutton", { name: "ceilingDb precise value" }), { target: { value: "-3" } });
    fireEvent.click(screen.getByRole("button", { name: "Plan changes" }));

    await waitFor(() => expect(commitGraph).toHaveBeenCalledWith("limiter-plan", demoSession.revision, expect.any(String)));
    expect(planGraph).toHaveBeenCalledWith(expect.objectContaining({
      nodes: expect.arrayContaining([
        expect.objectContaining({ kind: "limiter", parameters: expect.objectContaining({ ceilingDb: -3 }) }),
      ]),
    }));
  });

  it("expands an authoritative EQ preset into a draft node", async () => {
    const backend = {
      ...connectedPreviewBackend(),
      listPresets: async () => ({
        voiceChains: [],
        eq: [{ id: "hum50Hz", version: 1, name: "50 Hz hum notch", description: "Narrow 50 Hz notch starting point for mains hum." }],
      }),
    };
    render(<App backend={backend} />);
    fireEvent.click(await screen.findByRole("button", { name: "Add EQ to draft" }));
    expect(screen.getByText("Parametric EQ 1 added to the draft. Review and plan the changes before committing.")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Parametric EQ 1" })).toBeTruthy();
  });

  it("expands a voice-chain preset into ordinary draft processors", async () => {
    const backend = {
      ...connectedPreviewBackend(),
      listPresets: async () => ({
        voiceChains: [{ id: "voiceGateAndCompression", version: 1, name: "Voice gate and compression", description: "Voice neutral with a conservative gate and compression." }],
        eq: [],
      }),
    };
    render(<App backend={backend} />);
    fireEvent.click(await screen.findByRole("button", { name: "Add voice chain to draft" }));
    expect(screen.getByText("Gate 1, Compressor 1, Limiter 1 added to the draft. Review and plan the changes before committing.")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Gate 1" })).toBeTruthy();
  });

  it("persists tidy layout positions as presentation state", async () => {
    render(<App backend={connectedPreviewBackend()} />);

    fireEvent.click(await screen.findByRole("button", { name: "Tidy layout" }));
    expect(JSON.parse(window.localStorage.getItem("audiorouter.ui.layout.demo-session") ?? "null")).toMatchObject({
      mic: { x: 0, y: 0 },
      voice: { x: 260, y: 0 },
    });
  });

  it("exposes previewable mixer topology actions from the draft connection list", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const inserted = insertDraftMixer(connected, "edge-1");
    const onRemove = vi.fn();
    const onToggle = vi.fn();
    render(<BackendConnectionContext.Provider value={true}><DraftConnectionList session={inserted} onRemove={onRemove} onToggle={onToggle} /></BackendConnectionContext.Provider>);

    fireEvent.click(screen.getAllByRole("button", { name: /Insert mixer on/ })[0]);
    expect(onRemove).toHaveBeenCalledWith(insertMixerActionId("edge-1"));
    fireEvent.click(screen.getByRole("button", { name: "Remove and reconnect Mixer 1" }));
    expect(onRemove).toHaveBeenCalledWith(removeMixerActionId("mixer-1"));
  });

  it("executes mixer topology previews through the connected App draft boundary", async () => {
    render(<App backend={connectedPreviewBackend()} />);
    fireEvent.click(screen.getByRole("button", { name: "Keyboard connection dialog" }));
    const dialog = await screen.findByRole("dialog", { name: "Keyboard connection" });
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Keyboard source output port" }), { target: { value: "mic::out" } });
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Keyboard destination input port" }), { target: { value: "voice::in" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Add connection to draft" }));

    fireEvent.click(screen.getAllByRole("button", { name: /Insert mixer on/ })[0]);
    expect(screen.getByText("Mixer inserted into the draft. Review and plan the changes before committing.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Remove and reconnect Mixer 1" }));
    expect(screen.getByText("Mixer removed and its single path reconnected in the draft. Review and plan the changes before committing.")).toBeTruthy();
  });

  it("inserts a built-in processor directly from a connected draft path", async () => {
    render(<App backend={connectedPreviewBackend()} />);
    fireEvent.click(screen.getByRole("button", { name: "Keyboard connection dialog" }));
    const dialog = await screen.findByRole("dialog", { name: "Keyboard connection" });
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Keyboard source output port" }), { target: { value: "mic::out" } });
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Keyboard destination input port" }), { target: { value: "voice::in" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Add connection to draft" }));

    fireEvent.click(screen.getByRole("button", { name: "Insert Gate" }));
    expect(screen.getByText("Gate 1 inserted into the draft. Review and plan the changes before committing.")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Gate 1" })).toBeTruthy();
    for (const label of ["Gain", "Mute", "Parametric EQ", "Graphic EQ", "Compressor", "Gate", "Limiter", "Delay", "Pitch"]) {
      expect(screen.getAllByRole("button", { name: `Insert ${label}` })).toHaveLength(2);
    }
  });

  it("disables topology mutations when no backend connection context exists", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const inserted = insertDraftMixer(connected, "edge-1");
    render(<DraftConnectionList session={inserted} onRemove={vi.fn()} onToggle={vi.fn()} />);

    expect(screen.getAllByRole("button", { name: /Insert mixer on/ })[0]).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "Remove and reconnect Mixer 1" })).toHaveProperty("disabled", true);
  });

  it("keeps topology actions available in the structured list view", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const onRemove = vi.fn();
    const onToggle = vi.fn();
    const onInsertProcessor = vi.fn();
    render(<BackendConnectionContext.Provider value={true}><GraphList session={connected} selectedNodeId="mic" onSelect={vi.fn()} onRemoveConnection={onRemove} onToggleConnection={onToggle} onInsertProcessor={onInsertProcessor} /></BackendConnectionContext.Provider>);

    fireEvent.click(screen.getByRole("button", { name: "Insert mixer on Microphone to Voice gain" }));
    expect(onRemove).toHaveBeenCalledWith(insertMixerActionId("edge-1"));
    for (const label of ["Gain", "Mute", "Parametric EQ", "Graphic EQ", "Compressor", "Gate", "Limiter", "Delay", "Pitch"]) {
      expect(screen.getByRole("button", { name: `Insert ${label}` })).toBeTruthy();
    }
    fireEvent.click(screen.getByRole("button", { name: "Insert Parametric EQ" }));
    expect(onInsertProcessor).toHaveBeenCalledWith("edge-1", "parametricEq");
  });

  it("renders backend route provenance as an accessible path list", async () => {
    const inspectRoute = vi.fn(async () => ({
      destinationNode: "voice",
      reachable: true,
      complete: true,
      paths: [{ nodes: ["mic", "voice"], edges: ["edge-1"], channelMaps: [[1]], latencySamples: 48 }],
    }));
    render(<App backend={{ ...connectedPreviewBackend(), inspectRoute }} />);

    const routePanel = screen.getByRole("region", { name: "Receives audio from" });
    fireEvent.click(within(routePanel).getByRole("button", { name: "Refresh" }));
    await waitFor(() => expect(inspectRoute).toHaveBeenCalledWith("mic"));
    expect(await within(routePanel).findByRole("list", { name: "Reported audio paths" })).toBeTruthy();
    expect(within(routePanel).getByText(/Path 1: Microphone \[enabled\] → Voice gain \[enabled\]/)).toBeTruthy();
    expect(within(routePanel).getByText(/Channel map: \[1\]/)).toBeTruthy();
  });

  it("keeps the newest route inspection when refreshes resolve out of order", async () => {
    let releaseOld!: (value: { destinationNode: string; reachable: boolean; complete: boolean; paths: never[] }) => void;
    const oldResult = new Promise<{ destinationNode: string; reachable: boolean; complete: boolean; paths: never[] }>((resolve) => { releaseOld = resolve; });
    const inspectRoute = vi.fn().mockReturnValueOnce(oldResult).mockResolvedValueOnce({ destinationNode: "voice", reachable: false, complete: true, paths: [] });
    render(<App backend={{ ...connectedPreviewBackend(), inspectRoute }} />);
    const routePanel = screen.getByRole("region", { name: "Receives audio from" });
    const refresh = within(routePanel).getByRole("button", { name: "Refresh" });
    fireEvent.click(refresh);
    fireEvent.click(refresh);
    expect(await within(routePanel).findByText("No reachable route reported by the backend.")).toBeTruthy();
    releaseOld({ destinationNode: "voice", reachable: true, complete: true, paths: [] });
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(within(routePanel).getByText("No reachable route reported by the backend.")).toBeTruthy();
    expect(within(routePanel).queryByText(/reachable path/)).toBeNull();
  });

  it("prevents duplicate managed-bus planning while the first request is pending", async () => {
    let releasePlan!: (value: { planId: string; expiresInMs: number; operation: { action: "create"; id: string; name: string }; availability: { status: "unavailable"; reason: string }; requiredScopes: string[]; warnings: string[] }) => void;
    const planResult = new Promise<{ planId: string; expiresInMs: number; operation: { action: "create"; id: string; name: string }; availability: { status: "unavailable"; reason: string }; requiredScopes: string[]; warnings: string[] }>((resolve) => { releasePlan = resolve; });
    const planVirtualDevice = vi.fn(() => planResult);
    const backend = { ...connectedPreviewBackend(), planVirtualDevice };
    render(<App backend={backend} />);
    const panel = screen.getByRole("region", { name: "Virtual-device lifecycle" });
    const plan = within(panel).getByRole("button", { name: "Plan create" });
    fireEvent.click(plan);
    await waitFor(() => expect((plan as HTMLButtonElement).disabled).toBe(true));
    fireEvent.click(plan);
    expect(planVirtualDevice).toHaveBeenCalledTimes(1);
    releasePlan({ planId: "virtual-plan", expiresInMs: 30000, operation: { action: "create", id: "virtual-bus", name: "AudioRouter Bus" }, availability: { status: "unavailable", reason: "managed driver unavailable" }, requiredScopes: ["deviceAdministration"], warnings: [] });
    await waitFor(() => expect(within(panel).getByText("managed driver unavailable")).toBeTruthy());
  });

  it("shows persisted recording encoding details", async () => {
    const recording: RecordingRow = { id: "encoded-take", sessionId: demoSession.id, recorderId: "recorder-1", path: "C:\\Audio\\encoded.flac", format: "flac", channels: 2, sampleRate: 44100, frames: 4410, fileBytes: 12000, startTime: "2026-09-14T01:00:00Z", state: "completed", missing: false, title: null, artist: null, comment: null, dither: true, conversion: "targetSampleRate=44100;channels=2;bitsPerSample=16" };
    render(<App backend={{ ...connectedPreviewBackend(), listRecordings: async () => [recording] }} />);
    const panel = await screen.findByRole("region", { name: "Recording encoding" });
    expect(within(panel).getByText(/flac - 44100 Hz - 2 channels - TPDF dither/)).toBeTruthy();
    expect(within(panel).getByText(/targetSampleRate=44100;channels=2;bitsPerSample=16/)).toBeTruthy();
  });

  it("edits title, artist, and comment metadata together", async () => {
    const recording: RecordingRow = { id: "metadata-take", sessionId: demoSession.id, recorderId: "recorder-1", path: "C:\\Audio\\metadata.wav", format: "wav", channels: 1, sampleRate: 48000, frames: 480, fileBytes: 1000, startTime: "2026-09-14T01:00:00Z", state: "completed", missing: false, title: "Old title", artist: "Old artist", comment: "Old comment", dither: true, conversion: "targetSampleRate=48000;channels=1;format=wav" };
    const setRecordingMetadata = vi.fn(async () => ({ updated: true as const, recordingId: recording.id, title: "New title", artist: "New artist", comment: "New comment" }));
    render(<App backend={{ ...connectedPreviewBackend(), listRecordings: async () => [recording], setRecordingMetadata }} />);
    await screen.findByDisplayValue("Old title");
    fireEvent.change(screen.getByRole("textbox", { name: "Title for metadata-take" }), { target: { value: "New title" } });
    fireEvent.change(screen.getByRole("textbox", { name: "Artist for metadata-take" }), { target: { value: "New artist" } });
    fireEvent.change(screen.getByRole("textbox", { name: "Comment for metadata-take" }), { target: { value: "New comment" } });
    fireEvent.click(screen.getByRole("button", { name: "Save metadata" }));
    await waitFor(() => expect(setRecordingMetadata).toHaveBeenCalledWith(recording.id, expect.objectContaining({ title: "New title", artist: "New artist", comment: "New comment", idempotencyKey: expect.any(String) })));
    expect(await screen.findByText("Recording metadata saved; the audio file was unchanged.")).toBeTruthy();
  });

  it("validates and explicitly commits a stopped session import", async () => {
    const imported = { ...demoSession, id: "imported-session", name: "Imported voice setup" };
    const planSessionImport = vi.fn(async () => ({ planId: "import-plan", expiresInMs: 300000, session: imported }));
    const commitSessionImport = vi.fn(async () => ({ session: imported, state: "stopped" as const, imported: true as const }));
    const backend = { ...connectedPreviewBackend(), planSessionImport, commitSessionImport };
    render(<App backend={backend} />);

    const transferPanel = screen.getByRole("region", { name: "Session transfer" });
    const file = new File([JSON.stringify(demoSession)], "voice.audiorouter.json", { type: "application/json" });
    fireEvent.change(within(transferPanel).getByLabelText("Import session configuration"), { target: { files: [file] } });
    await waitFor(() => expect(planSessionImport).toHaveBeenCalledWith(demoSession));
    expect(await within(transferPanel).findByText(/Validated import: Imported voice setup/)).toBeTruthy();

    fireEvent.click(within(transferPanel).getByRole("button", { name: "Commit stopped import" }));
    await waitFor(() => expect(commitSessionImport).toHaveBeenCalledWith("import-plan", expect.any(String)));
    expect(await within(transferPanel).findByText(/Imported stopped session Imported voice setup/)).toBeTruthy();
  });
});
