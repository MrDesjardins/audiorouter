import { test, expect, type Page } from "@playwright/test";
import { demoSession } from "../src/fixtures";
const session = { ...demoSession, edges: [
  { id: "mic-voice", sourceNode: "mic", sourcePort: "out", destinationNode: "voice", destinationPort: "in", matrix: [1], enabled: true },
  { id: "voice-output", sourceNode: "voice", sourcePort: "out", destinationNode: "headphones", destinationPort: "in", matrix: [1, 1], enabled: true },
], nodes: [...demoSession.nodes].reverse() };

const position = (page: Page, id: string) => page.getByTestId(`rf__node-${id}`).evaluate((element) => {
  const parts = (element as HTMLElement).style.transform.match(/translate\(([-\d.]+)px,\s*([-\d.]+)px\)/);
  if (!parts) throw Error("missing position");
  return { x: Number(parts[1]), y: Number(parts[2]) };
});

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`Arrange follows connections and can be undone in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(({ session, theme }) => { Object.assign(window, { __routeFixtureSession: session }); localStorage.setItem("audiorouter.ui.theme", theme); }, { session, theme });
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("rf__node-mic")).toBeVisible();
    const before = await position(page, "voice");
    await expect(page.getByRole("button", { name: "Undo arrange", exact: true })).toHaveCount(0);
    await page.getByRole("button", { name: "Arrange", exact: true }).click();
    const mic = await position(page, "mic"), voice = await position(page, "voice"), out = await position(page, "headphones");
    // Input left, tool in the middle, output right, on one line.
    expect(mic.x).toBeLessThan(voice.x); expect(voice.x).toBeLessThan(out.x);
    expect(mic.y).toBe(voice.y); expect(voice.y).toBe(out.y);
    await expect(page.locator(".react-flow__edges .react-flow__edge")).toHaveCount(2);
    await page.screenshot({ path: testInfo.outputPath(`arranged-${theme}.png`) });
    await page.getByRole("button", { name: "Undo arrange", exact: true }).click();
    await expect.poll(() => position(page, "voice")).toEqual(before);
    await expect(page.getByRole("button", { name: "Undo arrange", exact: true })).toHaveCount(0);
  });
}
