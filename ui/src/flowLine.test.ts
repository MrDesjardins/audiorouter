import { describe, expect, it } from "vitest";
import { COMET_SPEED, MAX_COMETS, bezierLength, flowColor, flowVisual } from "./flowLine";

describe("audio connection visuals", () => {
  it("grow in width, glow and moving lights with level, within fixed bounds", () => {
    const quiet = flowVisual(-50), speech = flowVisual(-18), hot = flowVisual(-3);
    expect(quiet.comets).toBeGreaterThanOrEqual(1);
    expect(speech.coreWidth).toBeGreaterThan(quiet.coreWidth);
    expect(hot.glowOpacity).toBeGreaterThan(speech.glowOpacity);
    expect(hot.comets).toBeGreaterThan(quiet.comets);
    expect(flowVisual(12)).toEqual(flowVisual(0));
    expect(flowVisual(0).coreWidth).toBeLessThanOrEqual(6);
    expect(flowVisual(0).comets).toBe(MAX_COMETS);
    expect(flowVisual(-70).comets).toBe(0);
    expect(flowVisual(null).comets).toBe(0);
  });
  it("colours by level from the theme palette", () => {
    expect(flowColor(-50)).toBe("var(--flow-cold)");
    expect(flowColor(-30)).toBe("color-mix(in oklab, var(--flow-warm) 50%, var(--flow-cold))");
    expect(flowColor(-13)).toBe("color-mix(in oklab, var(--flow-hot) 50%, var(--flow-warm))");
    expect(flowColor(0)).toBe("var(--flow-clip)");
  });
  it("measures bezier paths for a constant travel speed", () => {
    expect(bezierLength("M0,0 C100,0 200,0 300,0")).toBeCloseTo(300, 0);
    expect(COMET_SPEED).toBeGreaterThan(0);
  });
});
