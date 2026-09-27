import { test, expect, type Locator, type Page } from "./real-backend";
import { libraryEntries } from "../src/library";

// Editing a tool must never move the Properties content (a jumping sidebar
// while dragging or typing was reported on 2026-09-27).

async function top(locator: Locator) {
  const box = await locator.boundingBox();
  expect(box, "element is laid out").not.toBeNull();
  return Math.round(box!.y);
}

async function addAndOpen(page: Page, label: string, kind: string) {
  await page.goto("/backend-harness.html");
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await page.locator(".tool-card").filter({ has: page.getByText(label, { exact: true }) }).click();
  await page.getByTestId(`rf__node-${kind}-1`).locator(".flow-node-title").click();
  return page.locator(".main-content > .inspector");
}

for (const entry of libraryEntries.filter((item) => item.kind && item.kind !== "parametricEq")) {
  test(`${entry.label}: editing a value does not move the Properties content`, async ({ page }) => {
    const inspector = await addAndOpen(page, entry.label, entry.kind!);
    const fields = inspector.getByRole("spinbutton").filter({ hasNot: page.locator("[disabled]") });
    if ((await fields.count()) === 0) test.skip(true, "no numeric settings");
    const first = fields.first();
    await first.scrollIntoViewIfNeeded();
    const heading = inspector.getByRole("heading", { level: 2 }).first();
    const before = { heading: await top(heading), field: await top(first) };
    const current = Number(await first.inputValue());
    const min = Number(await first.getAttribute("aria-valuemin") ?? Number.NEGATIVE_INFINITY);
    const max = Number(await first.getAttribute("aria-valuemax") ?? Number.POSITIVE_INFINITY);
    const next = current + 1 <= max ? current + 1 : Math.max(min, current - 1);
    await first.fill(String(next));
    await page.waitForTimeout(300);
    expect({ heading: await top(heading), field: await top(first) }).toEqual(before);
  });
}

test("Advanced EQ: adding, editing and removing points keeps the sidebar still, and fields accept retyping and negatives", async ({ page }) => {
  const inspector = await addAndOpen(page, "Advanced EQ", "parametricEq");
  const graph = inspector.locator(".advanced-eq-graph");
  const heading = inspector.getByRole("heading", { level: 2 }).first();
  // Relative to the panel heading: scrolling moves both, a layout jump does not.
  const offset = async () => (await top(graph)) - (await top(heading));
  const graphTop = await offset();
  await inspector.getByRole("button", { name: "Add point", exact: true }).click();
  const frequency = inspector.getByRole("spinbutton", { name: "EQ frequency Hz" });
  const gain = inspector.getByRole("spinbutton", { name: "EQ gain dB" });
  await frequency.click();
  // Delete and retype through values below the 20 Hz minimum.
  await frequency.press("Control+A");
  await frequency.press("Backspace");
  await expect(frequency).toHaveValue("");
  await frequency.pressSequentially("250");
  await expect(frequency).toHaveValue("250");
  await gain.click();
  await gain.press("Control+A");
  await gain.pressSequentially("-6.5");
  await expect(gain).toHaveValue("-6.5");
  await inspector.getByRole("combobox", { name: "EQ filter type" }).selectOption("notch");
  await expect(gain).toBeDisabled();
  await page.waitForTimeout(300);
  expect(await offset()).toBe(graphTop);
  await inspector.getByRole("button", { name: "Remove point", exact: true }).click();
  await page.waitForTimeout(300);
  expect(await offset()).toBe(graphTop);
  await page.locator(".topbar").getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.locator(".global-action-message")).toContainText(/saved.*revision/i);
});
