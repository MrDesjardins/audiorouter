import { describe, expect, it } from "vitest";
import { demoSession } from "./fixtures";
import { nodeAttention } from "./nodeAttention";

const base = { ...demoSession.nodes[0], parameters: { endpointId: "chosen" } };
const clear = { unfed: false, privacyMuted: false };

describe("nodeAttention", () => {
  it("says no device when a physical input or output has nothing chosen", () => {
    for (const kind of ["physicalInput", "physicalOutput"] as const) {
      expect(nodeAttention({ ...base, kind, parameters: {} }, clear)).toBe("no device");
      expect(nodeAttention({ ...base, kind }, clear)).toBeNull();
    }
  });

  it("says no input when nothing feeds the tool", () => {
    expect(nodeAttention({ ...base, kind: "gain" }, { ...clear, unfed: true })).toBe("no input");
  });

  it("says muted only on sources privacy mute silences", () => {
    const muted = { ...clear, privacyMuted: true };
    for (const kind of ["physicalInput", "applicationCapture", "endpointLoopback", "virtualRenderSource"] as const) {
      expect(nodeAttention({ ...base, kind }, muted)).toBe("muted");
    }
    expect(nodeAttention({ ...base, kind: "gain" }, muted)).toBeNull();
    expect(nodeAttention({ ...base, kind: "testSignal" }, muted)).toBeNull();
  });

  it("puts the fix the user must make before the mute note", () => {
    expect(
      nodeAttention({ ...base, kind: "physicalInput", parameters: {} }, { unfed: false, privacyMuted: true }),
    ).toBe("no device");
  });
});
