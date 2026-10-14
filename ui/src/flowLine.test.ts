import { describe, expect, it } from "vitest";
import {
  COMET_SPEED,
  LEVEL_STEP_DB,
  MAX_COMETS,
  bezierLength,
  cometKeyframes,
  flowColor,
  flowVisual,
  shownLevel,
} from "./flowLine";

describe("audio connection visuals", () => {
  it("grow in width, glow and moving lights with level, within fixed bounds", () => {
    const quiet = flowVisual(-50),
      speech = flowVisual(-18),
      hot = flowVisual(-3);
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
  it("draws the level in steps, so 20 Hz ticks rarely change the shared edge SVG", () => {
    expect(shownLevel(-19.2)).toBe(-18);
    expect(shownLevel(-19.6)).toBe(-21);
    expect(shownLevel(-18.4)).toBe(shownLevel(-17.6));
    expect(shownLevel(null)).toBeNull();
    expect(Math.abs(shownLevel(-37.4)! % LEVEL_STEP_DB)).toBe(0);
  });
  it("moves comets at constant speed along the curve, turned with it and never spinning", () => {
    const frames = cometKeyframes("M0,0 C100,0 200,0 300,0", 10);
    const points = frames.map((frame) => {
      const [, x, y, angle] = /translate\(([-\d.]+)px, ([-\d.]+)px\) rotate\(([-\d.]+)deg\)/.exec(
        String(frame.transform),
      )!;
      return { x: Number(x), y: Number(y), angle: Number(angle), offset: frame.offset };
    });
    expect(points[0]).toMatchObject({ x: 0, y: 0, offset: 0 });
    expect(points.at(-1)).toMatchObject({ x: 300, y: 0, offset: 1 });
    // Equal distance per keyframe although the control points bunch in t.
    points.slice(1).forEach((point, i) => expect(point.x - points[i].x).toBeCloseTo(30, 0));
    points.forEach((point) => expect(point.angle).toBeCloseTo(0, 5));
    // A loop through ±180° keeps a continuous angle (no 360° jump between frames).
    const loop = cometKeyframes("M0,0 C300,0 300,200 0,200", 24).map((frame) =>
      Number(/rotate\(([-\d.]+)deg\)/.exec(String(frame.transform))![1]),
    );
    loop.slice(1).forEach((angle, i) => expect(Math.abs(angle - loop[i])).toBeLessThan(90));
    expect(loop.at(-1)! - loop[0]).toBeCloseTo(180, -1);
  });
});
