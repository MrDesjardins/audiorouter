import { describe, expect, it } from "vitest";
import { formatValue, stepValue } from "../src/values.js";
import { levelColor, meterFace, paletteColor, playFace, privacyFace, sessionFace, stepFace, toggleFace } from "../src/faces.js";

describe("key looks a user can choose", () => {
  it("uses the chosen colours and words, and can drop the second line", () => {
    const on = toggleFace({ label: "Voice", detail: null, target: "enabled", on: true, pending: false, onColor: "purple", onText: "LIVE" });
    expect(on).toContain(paletteColor("purple", ""));
    expect(on).toContain(">LIVE<");
    expect(on).not.toContain("Enabled");
    const off = toggleFace({ label: "Voice", detail: "Enabled", target: "enabled", on: false, pending: false, offColor: "red", offText: "MUTED" });
    expect(off).toContain(paletteColor("red", ""));
    expect(off).toContain(">MUTED<");
    expect(paletteColor("not-a-colour", "#123456")).toBe("#123456");
  });

  it("leaves the label out when Stream Deck draws the title itself", () => {
    expect(toggleFace({ label: null, detail: null, target: "enabled", on: true, pending: false })).not.toContain(">Voice<");
    expect(meterFace({ label: null, peakDb: -10, rmsDb: -20, clipped: false, muted: false, playing: true })).not.toContain("Voice");
  });

  it("offers classic meter colours and hiding the number", () => {
    expect(levelColor(-30, "classic")).toBe(paletteColor("green", ""));
    expect(levelColor(-10, "classic")).toBe(paletteColor("yellow", ""));
    expect(levelColor(-3, "classic")).toBe(paletteColor("red", ""));
    expect(meterFace({ label: "Voice", peakDb: -10, rmsDb: -20, clipped: false, muted: false, playing: true, showValue: false })).not.toContain("-20 dB");
  });

  it("draws privacy, play, session and step keys with their states", () => {
    expect(privacyFace({ muted: true, mode: "toggle", pending: false, mutedText: "OFF AIR", showDetail: false })).toContain(">OFF AIR<");
    expect(privacyFace({ muted: true, mode: "toggle", pending: false, showDetail: false })).not.toContain("PRIVACY");
    expect(playFace({ label: "Gaming", playing: true, pending: false })).toContain(">PLAYING<");
    expect(playFace({ label: null, playing: false, pending: false })).toContain(">STOPPED<");
    expect(sessionFace({ name: "Ranked with Discord", selected: true, playing: false, cycle: false, pending: false })).toContain(">SELECTED<");
    expect(sessionFace({ name: "Chill", selected: false, playing: false, cycle: true, pending: false })).toContain("NEXT");
    expect(stepFace({ label: "Game", detail: "Gain Db", valueText: "-6 dB", step: "+1 dB", pending: false, atLimit: false })).toContain(">-6 dB<");
  });
});

describe("value steps", () => {
  const gain = { name: "gainDb", type: "number", minimum: -60, maximum: 24, unit: "dB" };
  it("adds, sets, stays within the setting's range and never drifts", () => {
    expect(stepValue(-6, 1, "add", gain)).toBe(-5);
    expect(stepValue(23.5, 1, "add", gain)).toBe(24);
    expect(stepValue(-59.5, -1, "add", gain)).toBe(-60);
    expect(stepValue(0.1, 0.2, "add", gain)).toBe(0.3);
    expect(stepValue(5, -12, "set", gain)).toBe(-12);
    expect(stepValue(5, 100, "set", gain)).toBe(24);
  });

  it("shows values with their unit", () => {
    expect(formatValue(-6, "dB")).toBe("-6 dB");
    expect(formatValue(80, "%")).toBe("80%");
    expect(formatValue(1.25, undefined)).toBe("1.3");
  });
});
