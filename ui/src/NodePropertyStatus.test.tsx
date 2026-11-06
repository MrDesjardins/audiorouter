/** @vitest-environment jsdom */
import { describe, it, expect } from "vitest";
import { demoSession } from "./fixtures";
import { nodePropertyStatus, nodePropertyStatusTone } from "./NodePropertyStatus";
describe("property status", () => {
  const unchosen = { ...demoSession.nodes[0], enabled: true, bypass: false, parameters: {} };
  const node = { ...unchosen, parameters: { endpointId: "mic-endpoint" } };

  it("distinguishes connectivity, disabled, bypass and playback", () => {
    expect(nodePropertyStatus(node, false, true, null)).toBe("Disconnected");
    expect(nodePropertyStatus({ ...node, enabled: false, bypass: true }, true, true, null)).toBe("Off");
    expect(nodePropertyStatus({ ...node, bypass: true }, true, true, null)).toBe("Bypass");
    expect(nodePropertyStatus(node, true, false, null)).toBe("Ready");
    expect(nodePropertyStatus(node, true, true, null)).toBe("Active");
  });

  it("never calls a device with nothing chosen, or a tool nothing feeds, Ready", () => {
    expect(nodePropertyStatus(unchosen, true, false, null)).toBe("No device");
    expect(nodePropertyStatus(unchosen, true, true, null)).toBe("No device");
    const gain = { ...node, kind: "gain" as const };
    expect(nodePropertyStatus(gain, true, false, null, true)).toBe("No input");
  });
});
describe("property status tone", () => {
  it("uses the canvas node-state colours", () => {
    expect(nodePropertyStatusTone("Active")).toBe("ready");
    expect(nodePropertyStatusTone("Ready")).toBe("ready");
    expect(nodePropertyStatusTone("Bypass")).toBe("bypassed");
    expect(nodePropertyStatusTone("Failed")).toBe("attention");
    expect(nodePropertyStatusTone("Disconnected")).toBe("attention");
    expect(nodePropertyStatusTone("Off")).toBe("disabled");
    expect(nodePropertyStatusTone("No device")).toBe("attention");
    expect(nodePropertyStatusTone("No input")).toBe("attention");
    expect(nodePropertyStatusTone("Muted")).toBe("attention");
  });
});
