import { openSettings } from "./workbench";
import { test, expect } from "./browser-test";

// Dragging the panel handle across the workbench used to select the tab labels it passed over.
test("dragging the panel handle does not select text", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 832 });
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  const handle = page.getByRole("separator", { name: "Resize the right panel" });
  const before = Number(await handle.getAttribute("aria-valuenow"));
  const box = (await handle.boundingBox())!;
  const y = box.y + 90;
  await page.mouse.move(box.x + box.width / 2, y);
  await page.mouse.down();
  await page.mouse.move(box.x + 60, y + 40, { steps: 6 });
  await page.mouse.move(box.x + 120, y + 80, { steps: 6 });
  await page.mouse.up();
  expect(Number(await handle.getAttribute("aria-valuenow"))).toBeLessThan(before);
  expect(await page.evaluate(() => window.getSelection()?.toString() ?? "")).toBe("");
});

test("the browser's resize notice leaves no UI error in the Logs tab", async ({ page }) => {
  // Regression: this harmless notice filled the log with "UI error (unknown)". A panel drag only
  // produces it when the timing happens to line up, so an observer that resizes what it watches forces
  // the browser's real event through the app's real listener.
  await page.setViewportSize({ width: 1280, height: 832 });
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  const notice = await page.evaluate(
    () =>
      new Promise<string>((resolve) => {
        window.addEventListener("error", (event) => resolve(event.message), { once: true });
        const box = document.body.appendChild(document.createElement("div"));
        let size = 1;
        const observer = new ResizeObserver(() => {
          size += 1;
          box.style.width = `${size}px`;
          box.style.height = `${size}px`;
        });
        observer.observe(box);
        window.setTimeout(() => observer.disconnect(), 500);
      }),
  );
  expect(notice).toContain("ResizeObserver loop");
  await openSettings(page, "Logs and support");
  await expect(page.locator(".diagnostic-list").first()).toBeVisible();
  await expect(page.locator(".diagnostic-list").first()).not.toContainText("UI error");
});

test("no selection starts from the tab strip or runs across its labels", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 832 });
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  const strip = (await page.locator(".workbench-tabs").boundingBox())!;
  await page.mouse.move(strip.x + 2, strip.y + strip.height - 2);
  await page.mouse.down();
  await page.mouse.move(strip.x + strip.width - 4, strip.y + 4, { steps: 8 });
  await page.mouse.up();
  expect(await page.evaluate(() => window.getSelection()?.toString() ?? "")).toBe("");
});
