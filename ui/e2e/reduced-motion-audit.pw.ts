import type { Page } from "@playwright/test";
import { expect, test } from "./browser-test";
import { settingsPageNames } from "./workbench";

// Motion criterion: with Windows "Animation effects" off (prefers-reduced-motion), nothing
// keeps moving: no running CSS or Web Animations anywhere on the workbench or Settings.
const moving = (page: Page) =>
  page.evaluate(() =>
    document
      .getAnimations()
      .filter((animation) => {
        if (animation.playState !== "running") return false;
        const timing = animation.effect?.getComputedTiming();
        // A near-instant animation (the global reduced-motion rule shortens them) is not motion.
        return Number(timing?.duration ?? 0) > 50 || timing?.iterations === Infinity;
      })
      .map((animation) => {
        const target = (animation.effect as KeyframeEffect | null)?.target as Element | null;
        const name =
          (animation as CSSAnimation).animationName ?? (animation as CSSTransition).transitionProperty ?? "script";
        return `${target?.className || target?.tagName}: ${name}`;
      }),
  );

async function openReduced(page: Page) {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1500, height: 950 });
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
}

test("nothing animates on the workbench or Settings when reduced motion is requested", async ({ page }) => {
  await openReduced(page);
  const found: string[] = [];
  for (const name of ["Properties", "Tools", "Devices", "Status", "Session", "Recording"]) {
    const tab = page.getByRole("tab", { name, exact: true });
    if ((await tab.count()) === 0) continue;
    await tab.click();
    await page.waitForTimeout(100);
    found.push(...(await moving(page)));
  }
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  const list = page.getByRole("tablist", { name: "Settings pages" });
  for (const name of await settingsPageNames(page)) {
    await list.getByRole("tab", { name, exact: true }).click();
    await page.waitForTimeout(100);
    found.push(...(await moving(page)));
  }
  expect([...new Set(found)]).toEqual([]);
});

test("the motion audit reports a looping animation that ignores reduced motion", async ({ page }) => {
  await openReduced(page);
  await page.evaluate(() => {
    const probe = document.createElement("div");
    probe.className = "probe-spinner";
    document.body.append(probe);
    probe.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 2000, iterations: Infinity });
  });
  expect(await moving(page)).toContain("probe-spinner: script");
});
