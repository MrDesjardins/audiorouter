import { describe, expect, it } from "vitest";
import { libraryDropPosition } from "./SessionFlowCanvas";

describe("canvas library drop positions", () => {
  it("converts viewport coordinates into bounded canvas coordinates", () => {
    expect(libraryDropPosition(240, 180, { left: 100, top: 50 })).toEqual({ x: 120, y: 110 });
  });

  it("falls back to the canvas origin for non-finite event coordinates", () => {
    expect(libraryDropPosition(Number.NaN, Number.POSITIVE_INFINITY, { left: 100, top: 50 })).toEqual({ x: 0, y: 0 });
  });
});
