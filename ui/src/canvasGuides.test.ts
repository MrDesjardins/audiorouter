import { describe, expect, it } from "vitest";
import { canvasGuides, guideLabel, NO_GUIDES, type GuideRect } from "./canvasGuides";

const card = (id: string, x: number, y: number, width = 240, height = 120): GuideRect => ({
  id,
  name: id,
  x,
  y,
  width,
  height,
});

describe("canvas alignment guides", () => {
  it("finds a shared left edge and spans both cards", () => {
    const { lines, names } = canvasGuides(card("moving", 48, 480), [card("Gain", 48, 96)]);
    expect(names).toEqual(["Gain"]);
    expect(lines).toContainEqual({ axis: "x", at: 48, from: 96, to: 600 });
  });

  it("finds a shared top edge", () => {
    const { lines } = canvasGuides(card("moving", 600, 96), [card("Gain", 48, 96)]);
    expect(lines).toContainEqual({ axis: "y", at: 96, from: 48, to: 840 });
  });

  it("finds lined-up centres between cards of different sizes", () => {
    const { lines } = canvasGuides(card("moving", 600, 120, 240, 120), [card("Gain", 48, 96, 240, 168)]);
    expect(lines.some((line) => line.axis === "y" && line.at === 180)).toBe(true);
  });

  it("shows nothing when nothing lines up, and never matches the moving card itself", () => {
    expect(canvasGuides(card("moving", 10, 10), [card("moving", 10, 10), card("Gain", 400, 500)])).toEqual(NO_GUIDES);
  });

  it("allows only a tiny difference", () => {
    expect(canvasGuides(card("moving", 49, 700), [card("Gain", 48, 96)]).names).toEqual(["Gain"]);
    expect(canvasGuides(card("moving", 52, 700), [card("Gain", 48, 96)]).names).toEqual([]);
  });

  it("merges several cards on one line", () => {
    const { lines, names } = canvasGuides(card("moving", 48, 480), [card("A", 48, 96), card("B", 48, 900)]);
    expect(names).toEqual(["A", "B"]);
    expect(lines.filter((line) => line.axis === "x" && line.at === 48)).toEqual([
      { axis: "x", at: 48, from: 96, to: 1020 },
    ]);
  });

  it("names cards in plain words and keeps the label short", () => {
    expect(guideLabel([])).toBe("");
    expect(guideLabel(["Mixer"])).toBe("Lined up with Mixer");
    expect(guideLabel(["Mixer", "Gain"])).toBe("Lined up with Mixer and Gain");
    expect(guideLabel(["Mixer", "Gain", "Mute", "Duck"])).toBe("Lined up with Mixer, Gain and 2 more");
  });
});
