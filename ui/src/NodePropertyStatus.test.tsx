/** @vitest-environment jsdom */
import { describe, it, expect } from "vitest";
import { demoSession } from "./fixtures";
import { nodePropertyStatus, nodePropertyStatusTone } from "./NodePropertyStatus";
describe("property status", () => {
  it("distinguishes connectivity, disabled, bypass and playback", () => {
    const node = { ...demoSession.nodes[0], enabled: true, bypass: false };
    expect(nodePropertyStatus(node, false, true, null)).toBe("Disconnected");
    expect(nodePropertyStatus({ ...node, enabled: false, bypass: true }, true, true, null)).toBe("Off");
    expect(nodePropertyStatus({ ...node, bypass: true }, true, true, null)).toBe("Bypass");
    expect(nodePropertyStatus(node, true, false, null)).toBe("Ready");
    expect(nodePropertyStatus(node, true, true, null)).toBe("Active");
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
  });
});
