import { expect } from "@playwright/test";
import type { Session } from "../../contracts/src/index";
import { test } from "./real-backend";
import { setColorTheme } from "./workbench";

test("copy and paste a configured node across sessions with menus and shortcuts", async ({
  page,
  backend,
}, testInfo) => {
  await page.goto("/backend-harness.html");
  await setColorTheme(page, "dark");
  const source = page.getByTestId("rf__node-voice");
  await source.click();
  await page.keyboard.press("Control+c");

  await page.getByRole("tab", { name: "Session", exact: true }).click();
  page.once("dialog", (dialog) => dialog.accept("Paste target"));
  await page.getByRole("button", { name: "New", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Paste target", exact: true })).toBeVisible();
  await page.keyboard.press("Control+v");
  const firstCopy = page.getByTestId("rf__node-voice-copy-1");
  await expect(firstCopy).toBeVisible();
  await expect(page.getByRole("spinbutton", { name: "Gain precise value" })).toHaveValue("0");

  await firstCopy.click({ button: "right" });
  const copyMenu = page.getByRole("menu", { name: "Tool actions" });
  await expect(copyMenu.getByRole("menuitem", { name: "Copy tool" })).toBeFocused();
  await page.screenshot({ path: testInfo.outputPath("copy-menu-dark.png") });
  await copyMenu.getByRole("menuitem", { name: "Copy tool" }).click();

  const canvas = page.locator(".session-flow-canvas");
  const bounds = await canvas.boundingBox();
  if (!bounds) throw new Error("Canvas is not visible");
  await canvas.click({
    button: "right",
    position: { x: Math.max(40, bounds.width - 35), y: Math.max(40, bounds.height - 35) },
  });
  const pasteMenu = page.getByRole("menu", { name: "Canvas actions" });
  await expect(pasteMenu.getByRole("menuitem", { name: "Paste tool" })).toBeEnabled();
  await page.screenshot({ path: testInfo.outputPath("paste-menu-dark.png") });
  await pasteMenu.getByRole("menuitem", { name: "Paste tool" }).click();
  const menuPasted = page.getByTestId("rf__node-voice-copy-2");
  await expect(menuPasted).toBeVisible();
  const canvasBounds = await canvas.boundingBox();
  const pastedBounds = await menuPasted.boundingBox();
  if (!canvasBounds || !pastedBounds) throw new Error("Pasted node bounds are unavailable");
  expect(pastedBounds.x).toBeGreaterThanOrEqual(canvasBounds.x);
  expect(pastedBounds.y).toBeGreaterThanOrEqual(canvasBounds.y);
  expect(pastedBounds.x + pastedBounds.width).toBeLessThanOrEqual(canvasBounds.x + canvasBounds.width);
  expect(pastedBounds.y + pastedBounds.height).toBeLessThanOrEqual(canvasBounds.y + canvasBounds.height);

  for (const theme of ["light", "high-contrast"]) {
    await setColorTheme(page, theme);
    await canvas.click({
      button: "right",
      position: { x: Math.max(40, bounds.width - 35), y: 40 },
    });
    await expect(page.getByRole("menu", { name: "Canvas actions" })).toBeVisible();
    await page.screenshot({ path: testInfo.outputPath(`paste-menu-${theme}.png`) });
    await page.keyboard.press("Escape");
  }

  await page.locator(".topbar").getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.locator(".global-action-message")).toContainText(/saved.*revision/i);
  await page.getByRole("tab", { name: "Session", exact: true }).click();
  const targetId = await page.getByLabel("Choose session").inputValue();
  const saved = await backend.call<Session>("sessions.get", { sessionId: targetId });
  const copiedNodes = saved.nodes.filter((node) => node.id.startsWith("voice-copy"));
  expect(copiedNodes).toHaveLength(2);
  expect(copiedNodes.every((node) => node.kind === "gain" && node.parameters.gainDb === 0)).toBe(true);
  expect(
    saved.edges.some((edge) =>
      copiedNodes.some((node) => edge.sourceNode === node.id || edge.destinationNode === node.id),
    ),
  ).toBe(false);
});
