import { describe, expect, it } from "vitest";
import { hexToHsv, hexToRgb, hsvToHex, hsvToRgb, rgbToHex } from "./themeColor";

describe("theme colour conversion", () => {
  it("converts primary colours both ways", () => {
    expect(hexToRgb("#ff0000")).toEqual([255, 0, 0]);
    expect(rgbToHex(0, 255, 0)).toBe("#00ff00");
    expect(hexToHsv("#ff0000")).toEqual({ h: 0, s: 1, v: 1 });
    expect(hexToHsv("#00ff00")).toEqual({ h: 120, s: 1, v: 1 });
    expect(hsvToHex(240, 1, 1)).toBe("#0000ff");
    expect(hsvToRgb(0, 0, 1)).toEqual([255, 255, 255]);
  });

  it("round trips arbitrary six-digit colours", () => {
    for (const hex of ["#171b21", "#25b8df", "#f6a928", "#aeb9c8", "#3a86ff", "#ff0080"]) {
      const hsv = hexToHsv(hex)!;
      expect(hsvToHex(hsv.h, hsv.s, hsv.v)).toBe(hex);
    }
  });

  it("reports no hue for grey and rejects non-hex input", () => {
    expect(hexToHsv("#808080")).toMatchObject({ s: 0 });
    expect(hexToHsv("#000000")).toEqual({ h: 0, s: 0, v: 0 });
    expect(hexToHsv("#ffffff")).toEqual({ h: 0, s: 0, v: 1 });
    expect(hexToHsv("red")).toBeNull();
    expect(hexToRgb("#12345")).toBeNull();
  });
});
