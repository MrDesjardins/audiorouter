import { expect, it } from "vitest";
import { cornerCoversTabs, topPlacement } from "./notificationPlacement";

it("clears the top bar when it is showing, and the window edge when it has scrolled away", () => {
  expect(topPlacement(58)).toBe(66);
  expect(topPlacement(-200)).toBe(16);
});

const VIEWPORT = 360;
const MESSAGE = 60;
// The message then spans 284 to 344 in a 360 px window.

it.each([
  ["tabs far below the window", { top: 610, bottom: 650 }, false],
  ["tabs just below the message", { top: 344, bottom: 384 }, false],
  ["tabs reaching into the message", { top: 330, bottom: 370 }, true],
  ["tabs behind the message", { top: 300, bottom: 340 }, true],
  ["tabs just above the message", { top: 244, bottom: 284 }, false],
  ["tabs scrolled above the window", { top: -50, bottom: -10 }, false],
])("%s", (_label, tabs, covers) => {
  expect(cornerCoversTabs(tabs, MESSAGE, VIEWPORT)).toBe(covers);
});
