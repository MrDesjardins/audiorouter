import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";

const session = {
  ...demoSession,
  edges: [
    {
      id: "mic-voice",
      sourceNode: "mic",
      sourcePort: "out",
      destinationNode: "voice",
      destinationPort: "in",
      matrix: [1],
      enabled: true,
    },
    {
      id: "voice-output",
      sourceNode: "voice",
      sourcePort: "out",
      destinationNode: "headphones",
      destinationPort: "in",
      matrix: [1, 1],
      enabled: true,
    },
  ],
};

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`cold start repairs missed connector bounds in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(
      ({ session, theme }) => {
        Object.assign(window, { __routeFixtureSession: session, __routeFixtureDelayMs: 100 });
        localStorage.setItem("audiorouter.ui.theme", theme);
        // Model a first WebView layout pass which sizes the cards before their
        // connector elements are available. Later DOM queries are normal.
        const query = Element.prototype.querySelectorAll;
        const missed = new WeakMap<Element, Set<string>>();
        Element.prototype.querySelectorAll = function (selectors: string) {
          if (this.classList.contains("react-flow__node") && [".source", ".target"].includes(selectors)) {
            const seen = missed.get(this) ?? new Set<string>();
            if (!seen.has(selectors)) {
              seen.add(selectors);
              missed.set(this, seen);
              return query.call(this, ":not(*)");
            }
          }
          return query.call(this, selectors);
        } as typeof query;
      },
      { session, theme },
    );
    await page.goto("/route-harness.html");
    for (const edge of session.edges)
      await expect(page.getByTestId(`rf__edge-${edge.id}`).locator("path").first()).toHaveAttribute("d", /^M-?\d/);
    await page.getByRole("button", { name: "Remove connection voice-output" }).click({ force: true });
    const source = page.getByTestId("rf__node-voice").locator('.source[data-debug-side="right"]');
    const target = page.getByTestId("rf__node-headphones").locator('.target[data-debug-side="left"]');
    const from = await source.boundingBox(),
      to = await target.boundingBox();
    if (!from || !to) throw new Error("Connector geometry is missing");
    await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
    await page.mouse.down();
    await page.mouse.move(to.x + to.width / 2, to.y + to.height / 2, { steps: 12 });
    await page.mouse.up();
    await expect(page.locator(".global-action-message")).toContainText("Connection added to the draft");
    await expect(page.locator(".react-flow__edges .react-flow__edge")).toHaveCount(2);
    await page.screenshot({ path: testInfo.outputPath(`cold-start-${theme}.png`) });
  });
}
