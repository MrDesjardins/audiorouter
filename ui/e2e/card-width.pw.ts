import { expect, test } from "./browser-test";

// The kind row's icon must pay for its own width: saved routes place cards by position,
// so a wider card lands on top of its neighbour and covers its connection dots.
const WITHOUT_ICON = `
  .node-kind-icon { display: none !important; }
  .flow-node-content .node-kind { letter-spacing: 0.12em !important; gap: 0 !important; }
`;

test("the kind icon does not make a tool card wider", async ({ page }) => {
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  const widths = () =>
    page
      .locator(".react-flow__node .flow-node-content")
      .evaluateAll((cards) => cards.map((card) => card.getBoundingClientRect().width));
  const shown = await widths();
  await page.addStyleTag({ content: WITHOUT_ICON });
  await page.waitForTimeout(100);
  const before = await widths();
  expect(shown.length).toBeGreaterThan(0);
  shown.forEach((width, index) => expect(width).toBeLessThanOrEqual(before[index]! + 0.5));
});
