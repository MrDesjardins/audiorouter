import { test, expect } from "./real-backend";

test("group settings, tool sizes and layout stay in their own session", async ({ page }) => {
  await page.goto("/backend-harness.html");
  await page.getByRole("button", { name: "Group A named visual background; no audio routing" }).click();
  await page.getByLabel("Group name").fill("Voice workspace");
  await expect(page.getByLabel("Move contents with group")).toBeChecked();
  await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
  await page.locator(".canvas-group-caption").click({ modifiers: ["Control"] });
  await page.getByRole("button", { name: "Largest width", exact: true }).click();
  await expect
    .poll(() => page.getByTestId("rf__node-voice").evaluate((element) => (element as HTMLElement).offsetWidth))
    .toBe(540);
  await page.getByRole("tab", { name: "Session", exact: true }).click();
  page.once("dialog", (dialog) => dialog.accept("Other layout"));
  await page.getByRole("button", { name: "New", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Other layout", exact: true })).toBeVisible();
  await expect(page.locator(".visual-group-node")).toHaveCount(0);
  await expect
    .poll(() => page.getByTestId("rf__node-voice").evaluate((element) => (element as HTMLElement).offsetWidth))
    .toBeLessThan(540);
  await page.getByLabel("Choose session").selectOption("e2e-session");
  await expect(page.locator(".canvas-group-caption")).toHaveText("Voice workspace");
  await expect
    .poll(() => page.getByTestId("rf__node-voice").evaluate((element) => (element as HTMLElement).offsetWidth))
    .toBe(540);
  await expect(page.getByRole("region", { name: "Selection layout" })).toHaveCount(0);
});

test("wide media tools share sizes without clipping and connectors follow tool resizing", async ({ page }) => {
  await page.goto("/backend-harness.html");
  const beforePath = await page
    .locator(".react-flow__edges .react-flow__edge .react-flow__edge-path")
    .last()
    .getAttribute("d");
  await page.getByRole("button", { name: "Group A named visual background; no audio routing" }).click();
  await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
  await page.locator(".canvas-group-caption").click({ modifiers: ["Control"] });
  await page.getByRole("button", { name: "Largest width", exact: true }).click();
  await expect
    .poll(() => page.locator(".react-flow__edges .react-flow__edge .react-flow__edge-path").last().getAttribute("d"))
    .not.toBe(beforePath);
  await expect(page.locator(".react-flow__edges .react-flow__edge")).toHaveCount(2);
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  for (const label of ["Test Signal", "Audio file"])
    await page
      .locator(".tool-card")
      .filter({ has: page.getByText(label, { exact: true }) })
      .click();
  await page.locator(".react-flow__controls-fitview").click();
  await page.getByTestId("rf__node-testSignal-1").locator(".flow-node-title").click();
  await page
    .getByTestId("rf__node-audioFile-1")
    .locator(".flow-node-title")
    .click({ modifiers: ["Control"] });
  await page.locator(".canvas-group-caption").click({ modifiers: ["Control"] });
  await page.getByRole("button", { name: "Largest width", exact: true }).click();
  const widths = await page
    .locator(".has-layout-size")
    .evaluateAll((elements) => elements.map((element) => (element as HTMLElement).offsetWidth));
  expect(new Set(widths).size).toBe(1);
  await page.getByRole("button", { name: "Undo layout", exact: true }).click();
  await page.getByRole("button", { name: "Smallest width", exact: true }).click();
  for (const id of ["testSignal-1", "audioFile-1"]) {
    const node = page.getByTestId("rf__node-" + id);
    expect(
      await node.evaluate((element) => {
        const bounds = element.getBoundingClientRect();
        return [...element.querySelectorAll("button, input, select")].every((control) => {
          const rect = control.getBoundingClientRect();
          return rect.left >= bounds.left - 1 && rect.right <= bounds.right + 1;
        });
      }),
    ).toBe(true);
  }
});
