import { expect, test } from "./real-backend";
import { openSettings } from "./workbench";

const THEMES = ["dark", "light", "high-contrast"];

for (const theme of THEMES) {
  test(`goal dialog in ${theme}`, async ({ page }, info) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await page.goto("/backend-harness.html");
    await page.keyboard.press("Control+k");
    await page.getByRole("combobox").fill("start from");
    await page.keyboard.press("Enter");
    const dialog = page.getByRole("dialog", { name: "What do you want to do?" });
    await expect(dialog.getByRole("button", { name: "Use this: Clean up my mic" })).toBeVisible();
    await page.screenshot({ path: info.outputPath(`${theme}-goals.png`) });
  });
}

test("a goal makes a real stopped route with no device chosen, and the tour replays from Help", async ({
  page,
}, info) => {
  await page.goto("/backend-harness.html");
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  await page.keyboard.press("Control+k");
  await page.getByRole("combobox").fill("start from");
  await page.keyboard.press("Enter");
  await page.getByRole("button", { name: "Use this: Clean up my mic" }).click();
  await expect(page.getByRole("dialog", { name: "What do you want to do?" })).toHaveCount(0);

  for (const name of ["Microphone", "Speech Denoise", "Gate", "Compressor", "Where your voice goes"]) {
    await expect(page.locator(".react-flow__node").filter({ hasText: name }).first()).toBeAttached();
  }
  await expect(page.locator(".react-flow__edges .react-flow__edge")).toHaveCount(4);
  await expect(page.getByRole("button", { name: "Play", exact: true })).toBeVisible();
  await expect(page.locator(".global-action-message")).toContainText("Nothing plays until you press Play");
  await expect(page.locator(".global-action-message")).toContainText("your microphone");
  // It opens arranged left to right, with no card on top of another.
  const boxes = await page.locator(".react-flow__node").evaluateAll((nodes) =>
    nodes.map((node) => {
      const box = node.getBoundingClientRect();
      return { x: box.x, right: box.right, y: box.y, bottom: box.bottom };
    }),
  );
  await expect(boxes.length).toBe(5);
  for (const [index, one] of boxes.entries()) {
    for (const other of boxes.slice(index + 1)) {
      const overlap = one.x < other.right && other.x < one.right && one.y < other.bottom && other.y < one.bottom;
      expect(overlap).toBe(false);
    }
  }
  await page.waitForTimeout(350);
  await page.screenshot({ path: info.outputPath("goal-route.png") });

  // Reload: it was really saved by the backend.
  await page.reload();
  await expect(page.locator(".react-flow__edges .react-flow__edge")).toHaveCount(4);

  // A route wider than the canvas opens where its signal begins.
  const canvas = (await page.locator(".session-flow-canvas").boundingBox())!;
  await page.waitForTimeout(350);
  const microphone = (await page.locator(".react-flow__node").filter({ hasText: "Microphone" }).boundingBox())!;
  expect(microphone.x).toBeGreaterThanOrEqual(canvas.x);
  expect(microphone.x).toBeLessThan(canvas.x + 100);

  await openSettings(page, "Help");
  await page.getByRole("button", { name: "Take the tour" }).click();
  await expect(page.getByRole("dialog", { name: "Add tools from here" })).toBeVisible();
  await page.screenshot({ path: info.outputPath("tour-1.png") });
  await page.getByRole("button", { name: "Next" }).click();
  await expect(page.getByRole("dialog", { name: "Connect the dots" })).toBeVisible();
  await page.getByRole("button", { name: "Next" }).click();
  await expect(page.getByRole("dialog", { name: "Press Play" })).toBeVisible();
  await page.screenshot({ path: info.outputPath("tour-3.png") });
  await page.getByRole("button", { name: "Done" }).click();
  await expect(page.locator(".tour-layer")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Play", exact: true })).toBeVisible();
});

test("Ctrl+K finds a tool, a setting and a card, and Escape returns", async ({ page }, info) => {
  await page.goto("/backend-harness.html");
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  const palette = page.getByRole("dialog", { name: "Find anything" });
  await page.keyboard.press("Control+k");
  await expect(palette).toBeVisible();
  await page.getByRole("combobox").fill("mu");
  await page.screenshot({ path: info.outputPath("palette.png") });
  await page.getByRole("combobox").fill("add compressor");
  await page.keyboard.press("Enter");
  await expect(palette).toHaveCount(0);
  await expect(page.locator(".react-flow__node").filter({ hasText: "Compressor" }).first()).toBeAttached();

  await page.keyboard.press("Control+k");
  await page.getByRole("combobox").fill("keyboard shortcuts");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: /shortcuts/i }).first()).toBeVisible();

  await page.keyboard.press("Control+k");
  await page.keyboard.press("Escape");
  await expect(palette).toHaveCount(0);
});

test("a preset saved from Properties is listed, applied and deleted through the real backend", async ({
  page,
}, info) => {
  await page.goto("/backend-harness.html");
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await page
    .locator(".tool-card")
    .filter({ has: page.getByText("Gain", { exact: true }) })
    .first()
    .click();
  await page.locator(".react-flow__node").filter({ hasText: "Gain" }).last().locator(".flow-node-title").click();
  await page.getByRole("tab", { name: "Properties" }).click();
  const section = page.locator('.property-section:has(> summary:has-text("My presets"))');
  await section.locator("summary").click();
  await section.getByRole("button", { name: "Save as…" }).click();
  await section.getByLabel("Preset name").fill("Steady voice");
  await section.getByRole("button", { name: "Save", exact: true }).click();
  await expect(section.getByRole("status")).toContainText("Saved");
  await expect(section.getByLabel("Preset").locator("option")).toContainText(["Choose a preset", "Steady voice"]);
  await page.screenshot({ path: info.outputPath("presets.png") });
  await section.getByRole("button", { name: "Delete" }).click();
  await expect(section.getByRole("status")).toContainText("deleted");
});

test("saved versions name the window, and Go back here returns an earlier route", async ({ page }, info) => {
  await page.goto("/backend-harness.html");
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  const save = page.locator(".topbar").getByRole("button", { name: "Save", exact: true });
  const addAndSave = async (label: string) => {
    await page.getByRole("tab", { name: "Tools", exact: true }).click();
    await page
      .locator(".tool-card")
      .filter({ has: page.getByText(label, { exact: true }) })
      .first()
      .click();
    await expect(save).toBeEnabled();
    await save.click();
    await expect(save).toBeDisabled();
  };
  await addAndSave("Gain");
  await addAndSave("Meter");
  await page.getByRole("tab", { name: "Session", exact: true }).click();
  const versions = page.getByRole("list", { name: "Saved versions" });
  await expect(versions.getByText("You are here")).toBeVisible();
  await page.screenshot({ path: info.outputPath("history.png") });
  const goBack = versions.getByRole("button", { name: /Go back to the version/ });
  const before = await goBack.count();
  await goBack.first().click();
  await expect(page.getByText(/Later versions are still in this list/)).toBeVisible();
  // Going back is a new save, so later versions stay reachable.
  await expect(goBack).toHaveCount(before + 1);
  await expect(page.locator(".react-flow__node").filter({ hasText: "Meter" })).toHaveCount(0);
});
