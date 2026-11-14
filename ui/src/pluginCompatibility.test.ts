import { describe, expect, it } from "vitest";
import { SUPPORTED_PLUGIN_COMPATIBILITY } from "./draft";

// The backend reports these for scan results; a 32-bit plugin runs in the
// 32-bit worker, so it must be addable just like a 64-bit one.
describe("plugin compatibility", () => {
  it("lists every format and bitness the backend can host", () => {
    for (const value of ["supportedVst2X64Gated", "supportedVst3X64", "supportedVst2X86", "supportedVst3X86"]) {
      expect(SUPPORTED_PLUGIN_COMPATIBILITY).toContain(value);
    }
  });

  it("never lists a refusal", () => {
    expect(SUPPORTED_PLUGIN_COMPATIBILITY).not.toContain("unsupportedFormat");
  });
});
