import { describe, expect, it } from "vitest";
import { selectNativePump } from "./nativePump";

describe("native pump selection", () => {
  it("requires an authoritative kind before dispatching", () => {
    expect(selectNativePump(null, true, true)).toBeNull();
    expect(selectNativePump(undefined, true, true)).toBeNull();
    expect(selectNativePump("unknown", true, true)).toBeNull();
  });

  it("selects only the available matching adapter", () => {
    expect(selectNativePump("endpoint", true, false)).toBe("endpoint");
    expect(selectNativePump("endpoint", false, true)).toBeNull();
    expect(selectNativePump("duplex", false, true)).toBe("duplex");
    expect(selectNativePump("duplex", true, false)).toBeNull();
  });
});
