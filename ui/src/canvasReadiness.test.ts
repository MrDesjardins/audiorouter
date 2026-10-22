import { expect, it } from "vitest";
import { connectorGeometryReady } from "./canvasReadiness";
const expected = { id: "tool", source: ["out:left", "out:right"], target: ["in:left"] };
const measured = { width: 220, height: 180 };
const handle = (id: string) => ({ id, x: 1, y: 2, width: 10, height: 10 });
const bounds = () => ({ source: expected.source.map(handle), target: expected.target.map(handle) });
it("requires the measured card and every expected connector", () => {
  expect(connectorGeometryReady(measured, bounds(), expected)).toBe(true);
  expect(connectorGeometryReady(undefined, bounds(), expected)).toBe(false);
  expect(connectorGeometryReady(measured, null, expected)).toBe(false);
  for (const bad of [0, -1, NaN, Infinity])
    expect(connectorGeometryReady({ ...measured, width: bad }, bounds(), expected)).toBe(false);
  const missing = bounds();
  missing.source.pop();
  expect(connectorGeometryReady(measured, missing, expected)).toBe(false);
  const wrong = bounds();
  wrong.target[0].id = "old-session-port";
  expect(connectorGeometryReady(measured, wrong, expected)).toBe(false);
});
it("rejects nonfinite coordinates and zero-size handles", () => {
  for (const patch of [{ x: NaN }, { y: Infinity }, { width: 0 }, { height: -1 }]) {
    const invalid = bounds();
    Object.assign(invalid.source[0], patch);
    expect(connectorGeometryReady(measured, invalid, expected)).toBe(false);
  }
});
it("an empty-port card still needs usable dimensions", () => {
  const empty = { id: "annotation", source: [], target: [] };
  expect(connectorGeometryReady(measured, null, empty)).toBe(true);
  expect(connectorGeometryReady({}, null, empty)).toBe(false);
});
