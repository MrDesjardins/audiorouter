import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";
const session = { ...demoSession, edges: [
  { id: "mic-voice", sourceNode: "mic", sourcePort: "out", destinationNode: "voice", destinationPort: "in", matrix: [1], enabled: true },
  { id: "voice-output", sourceNode: "voice", sourcePort: "out", destinationNode: "headphones", destinationPort: "in", matrix: [1, 1], enabled: true },
], nodes: [...demoSession.nodes].reverse() };
for (const theme of ["dark", "light", "high-contrast"]) {
  test(`smart tidy and reset follow connections in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(({ session, theme }) => { Object.assign(window, { __routeFixtureSession: session }); localStorage.setItem("audiorouter.ui.theme", theme); }, { session, theme });
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("rf__node-mic")).toBeVisible();
    await page.getByRole("button", { name: "Tidy layout", exact: true }).click();
    const position = async (id: string) => page.getByTestId(`rf__node-${id}`).evaluate(el => { const parts = (el as HTMLElement).style.transform.match(/translate\(([-\d.]+)px,\s*([-\d.]+)px\)/); if (!parts) throw Error("missing position"); return { x: Number(parts[1]), y: Number(parts[2]) }; });
    const mic = await position("mic"), voice = await position("voice"), out = await position("headphones");
    expect(mic.x).toBeLessThan(voice.x); expect(voice.x).toBeLessThan(out.x);
    await page.getByRole("button", { name: "Reset layout", exact: true }).click();
    expect(await position("voice")).toEqual(voice);
    await expect(page.locator(".react-flow__edges .react-flow__edge")).toHaveCount(2);
    await page.screenshot({ path: testInfo.outputPath(`smart-layout-${theme}.png`) });
  });
}
