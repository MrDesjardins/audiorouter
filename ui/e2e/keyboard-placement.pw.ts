import { expect, test } from "./browser-test";

// Drag/drop criterion: an equivalent keyboard path for placing a new tool.
test("a tool added with Enter can be placed with the arrow keys alone", async ({ page }) => {
  await page.setViewportSize({ width: 1500, height: 950 });
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  const before = await page.locator(".react-flow__node").count();
  const tool = page.locator(".tool-card").filter({ has: page.getByText("Compressor", { exact: true }) });
  await tool.focus();
  await page.keyboard.press("Enter");
  await expect(page.locator(".react-flow__node")).toHaveCount(before + 1);
  const added = page.locator(".react-flow__node").last();
  const focusedId = await page.evaluate(() =>
    document.activeElement?.closest(".react-flow__node")?.getAttribute("data-id"),
  );
  const addedId = await added.getAttribute("data-id");
  expect(focusedId, "focus moves to the new card").toBe(addedId);
  const start = await added.evaluate((element) => (element as HTMLElement).style.transform);
  await page.keyboard.press("ArrowRight");
  await expect.poll(() => added.evaluate((element) => (element as HTMLElement).style.transform)).not.toBe(start);
  // Escape hands focus back to the tool that added the card (owner, Round 28 attended check).
  await page.keyboard.press("Escape");
  await expect(tool).toBeFocused();
  await expect(page.locator(".react-flow__node")).toHaveCount(before + 1);
});
