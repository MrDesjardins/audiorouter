/** @vitest-environment jsdom */
import { afterEach, describe, expect, it } from "vitest";
import type { DeviceListItem } from "@audiorouter/contracts";
import {
  findReplacementCaptureEndpoint,
  rememberDeviceNameBinding,
  rememberDeviceNames,
  rememberSessionDeviceNames,
  rememberedDeviceName,
} from "./endpointBinding";

afterEach(() => window.localStorage.clear());

const input = (id: string, name: string, state: "active" | "disabled" = "active"): DeviceListItem =>
  state === "active"
    ? {
        id,
        name,
        direction: "capture",
        state,
        defaultRoles: [],
        format: {
          sampleRateHz: 48000,
          channels: 1,
          bitsPerSample: 32,
          formatTag: 3,
          isFloat32: true,
          bytesPerFrame: 4,
        },
        periods: { default100ns: 100000, minimum100ns: 30000 },
      }
    : { id, name, direction: "capture", state, defaultRoles: [] };

const render = (id: string, name: string): DeviceListItem => ({
  id,
  name,
  direction: "render",
  state: "active",
  defaultRoles: [],
  format: {
    sampleRateHz: 48000,
    channels: 2,
    bitsPerSample: 32,
    formatTag: 3,
    isFloat32: true,
    bytesPerFrame: 8,
  },
  periods: { default100ns: 100000, minimum100ns: 30000 },
});

describe("physical input name recovery", () => {
  it("proposes one exact case-insensitive active capture match", () => {
    const old = input("old-id", "USB Microphone");
    rememberDeviceNames([old]);
    const current = input("new-id", "usb microphone");
    expect(findReplacementCaptureEndpoint([current], "old-id")?.id).toBe("new-id");
    expect(rememberedDeviceName("old-id")).toBe("USB Microphone");
  });

  it("does not guess when no active exact capture match exists", () => {
    rememberDeviceNames([input("old-id", "USB Microphone")]);
    expect(findReplacementCaptureEndpoint([input("new-id", "USB Microphone", "disabled")], "old-id")).toBeNull();
    expect(findReplacementCaptureEndpoint([input("new-id", "USB Microphone X")], "old-id")).toBeNull();
    expect(findReplacementCaptureEndpoint([render("render-id", "USB Microphone")], "old-id")).toBeNull();
  });

  it("does not guess between duplicate endpoint names or without cached identity", () => {
    rememberDeviceNames([input("old-id", "USB Microphone")]);
    expect(
      findReplacementCaptureEndpoint([input("new-a", "USB Microphone"), input("new-b", "USB Microphone")], "old-id"),
    ).toBeNull();
    expect(findReplacementCaptureEndpoint([input("new-id", "USB Microphone")], "unknown-id")).toBeNull();
  });

  it("uses the Windows recovery name supplied by the read-only Setup Check", () => {
    expect(findReplacementCaptureEndpoint([input("new-id", "Studio Mic")], "old-id", "studio mic")?.id).toBe("new-id");
  });

  it("does not suggest a different endpoint when the exact saved ID is active again", () => {
    rememberDeviceNames([input("old-id", "Studio Microphone")]);
    expect(
      findReplacementCaptureEndpoint(
        [input("old-id", "Studio Microphone"), input("new-id", "Studio Microphone")],
        "old-id",
      ),
    ).toBeNull();
  });

  it("remembers a manually confirmed replacement name for sibling saved sessions", () => {
    rememberDeviceNameBinding("old-id", "Studio Mic", "capture");
    expect(findReplacementCaptureEndpoint([input("new-id", "studio mic")], "old-id")?.id).toBe("new-id");
  });

  it("seeds legacy session endpoint names while the exact IDs are present", () => {
    const savedDevice = input("legacy-mic-id", "Conference Microphone");
    rememberSessionDeviceNames(
      [
        {
          id: "saved-session",
          name: "Saved session",
          schemaVersion: 1,
          revision: 1,
          nodes: [
            {
              id: "mic-node",
              kind: "physicalInput",
              typeVersion: 1,
              name: "Microphone",
              enabled: true,
              bypass: false,
              parameters: { endpointId: "legacy-mic-id" },
              ports: [{ name: "out", direction: "output", channels: 1 }],
            },
          ],
          edges: [],
        },
      ],
      [savedDevice],
    );
    expect(rememberedDeviceName("legacy-mic-id")).toBe("Conference Microphone");
    expect(findReplacementCaptureEndpoint([input("new-id", "conference microphone")], "legacy-mic-id")?.id).toBe(
      "new-id",
    );
  });
});
