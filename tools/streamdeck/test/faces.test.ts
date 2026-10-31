import { describe, expect, it } from "vitest";
import { dataUrl, fit, levelColor, messageFace, meterFace, meterFraction, privacyFace, soundFace, targetLabel, toggleFace } from "../src/faces.js";

describe("level colours and meter scale", () => {
  it("matches the canvas colours: cold, warm, hot, clip", () => {
    expect(levelColor(null)).toBe("#3fd2c7");
    expect(levelColor(-50)).toBe("#3fd2c7");
    expect(levelColor(-20)).toBe("#ffd166");
    expect(levelColor(-6)).toBe("#ff9640");
    expect(levelColor(0)).toBe("#ff5d6c");
    expect(levelColor(-30)).not.toBe("#3fd2c7");
  });

  it("fills −60 dB to 0 dB and never leaves the meter", () => {
    expect(meterFraction(-60)).toBe(0);
    expect(meterFraction(-30)).toBe(0.5);
    expect(meterFraction(6)).toBe(1);
    expect(meterFraction(null)).toBe(0);
    expect(meterFraction(Number.NaN)).toBe(0);
  });
});

describe("faces", () => {
  it("shortens long labels and escapes text", () => {
    expect(fit("Microphone input chain", 12)).toBe("Microphone …");
    expect(messageFace("<b>", "a & b")).toContain("&lt;b&gt;");
    expect(dataUrl("<svg/>")).toBe("data:image/svg+xml;charset=utf8,%3Csvg%2F%3E");
  });

  it("draws a switch that says what it does", () => {
    expect(toggleFace({ label: "Voice EQ", detail: "Enabled", target: "enabled", on: true, pending: false })).toContain(">ON<");
    expect(toggleFace({ label: "Voice EQ", detail: "Enabled", target: "enabled", on: false, pending: false })).toContain(">OFF<");
    expect(toggleFace({ label: "Voice EQ", detail: null, target: "bypass", on: true, pending: false })).toContain(">BYPASS<");
    expect(toggleFace({ label: "Switch", detail: null, target: "selected", on: true, pending: false, valueText: "B" })).toContain(">B<");
    expect(targetLabel("phaseInvert")).toBe("Phase Invert");
  });

  it("draws meters for playing, muted and stopped paths", () => {
    const loud = meterFace({ label: "Voice", peakDb: -3, rmsDb: -12, clipped: true, muted: false, playing: true });
    expect(loud).toContain("-12 dB");
    expect(loud).toContain("<circle");
    expect(meterFace({ label: "Voice", peakDb: -3, rmsDb: -12, clipped: false, muted: true, playing: true })).toContain(">muted<");
    expect(meterFace({ label: "Voice", peakDb: null, rmsDb: null, clipped: false, muted: false, playing: false })).toContain(">stopped<");
  });

  it("makes privacy mute unmistakable", () => {
    expect(privacyFace({ muted: true, mode: "toggle", pending: false })).toContain(">MUTED<");
    expect(privacyFace({ muted: false, mode: "unmute", pending: false })).toContain(">LIVE<");
    expect(privacyFace({ muted: false, mode: "unmute", pending: false })).toContain("UNMUTE MIC");
  });
});

describe("sound face", () => {
  it("shows a name without the numbered fallback, and preserves behavior and pending state", () => {
    const face = soundFace({ label: "Airhorn <1>", named: true, slot: 3, pending: false });
    expect(face).toContain("Airhorn &lt;1&gt;");
    expect(face).not.toContain("SOUND 3");
    expect(soundFace({ label: null, named: false, slot: 9, pending: true })).toContain("opacity=\"0.55\"");
    for (const behavior of ["once", "loop", "hold"] as const) {
      expect(soundFace({ label: "Horn", named: true, slot: 2, pending: false, behavior })).toContain(`>${behavior.toUpperCase()}<`);
      expect(soundFace({ label: null, named: false, slot: 2, pending: false, behavior })).toContain(`SOUND 2 · ${behavior.toUpperCase()}`);
    }
    expect(soundFace({ label: null, named: true, slot: 2, pending: false })).not.toContain("<text");
    expect(soundFace({ label: null, named: false, slot: 2, pending: false })).toContain(">SOUND 2<");
  });
});
