import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";

// A stereo audio interface with one microphone: the Channels select sends
// that input to both ears. It must use the app field style in every theme,
// keep the layout fixed when its guidance changes, and edit only the draft.
const session = {
  ...demoSession,
  nodes: demoSession.nodes.map((node) =>
    node.kind === "physicalInput"
      ? { ...node, name: "Interface mic", parameters: { endpointId: "capture-preview" } }
      : node,
  ),
};
const inputId = session.nodes.find((node) => node.kind === "physicalInput")!.id;

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`input channels inspector in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(
      ({ session, theme }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, { __routeFixtureSession: session });
      },
      { session, theme },
    );
    await page.goto("/route-harness.html");
    await page.getByTestId(`rf__node-${inputId}`).click();
    const select = page.getByLabel("Input channels");
    const field = page.getByLabel("Input channel settings");
    await expect(select).toHaveValue("stereo");
    await expect(field).toContainText("only in one ear");
    await field.scrollIntoViewIfNeeded();
    const below = page.getByLabel("Spatial audio settings");
    const before = await below.boundingBox();
    await select.selectOption("left");
    await expect(select).toHaveValue("left");
    await expect(field).toContainText("only the left channel to both ears");
    await field.screenshot({ path: testInfo.outputPath(`input-channels-${theme}.png`) });
    // Turning on surround disables the choice.
    await page.getByLabel("Spatial audio mode").selectOption("headphones");
    await expect(select).toBeDisabled();
    await page.getByLabel("Spatial audio mode").selectOption("off");
    await select.selectOption("mono");
    expect((await below.boundingBox())?.y).toBe(before?.y);
    expect(
      await page.evaluate(() => (window as any).__routeFixtureCalls().filter((call: string) => call === "commit")),
    ).toEqual([]);
  });
}
