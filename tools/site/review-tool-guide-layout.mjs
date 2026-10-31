import { chromium } from "../../ui/node_modules/playwright-core/index.mjs";
import { mkdirSync } from "node:fs";
mkdirSync("target/tool-guide-layout", { recursive: true });
const browser = await chromium.launch({ channel: "msedge" });
try {
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  for (const kind of [
    "gate",
    "parametric-eq",
    "soundboard",
    "physical-input",
  ]) {
    await page.goto(`http://127.0.0.1:4187/site/tools/${kind}.html#in-the-app`);
    await page
      .locator("#in-the-app")
      .evaluate((e) => e.scrollIntoView({ block: "start" }));
    const state = await page.evaluate(() => ({
      overflow: document.documentElement.scrollWidth > innerWidth,
      videos: [...document.querySelectorAll("#in-the-app video")].map((v) => ({
        controls: v.controls,
        preload: v.preload,
        autoplay: v.autoplay,
        box: v.getBoundingClientRect().width,
      })),
    }));
    if (
      state.overflow ||
      state.videos.some(
        (v) => !v.controls || v.autoplay || v.preload !== "none" || v.box > 390,
      )
    )
      throw new Error(`${kind}: ${JSON.stringify(state)}`);
    await page.screenshot({ path: `target/tool-guide-layout/${kind}.png` });
    console.log(`${kind}: phone layout PASS`);
  }
  await page.goto("http://127.0.0.1:4187/site/tools/gate.html");
  const play = page.getByRole("button", { name: "Play example", exact: true });
  await play.click();
  await page.waitForFunction(
    () =>
      document.querySelector("[data-guide-demo]").dataset.audioState ===
      "playing",
  );
  await page
    .locator(".guide-recording video")
    .first()
    .evaluate((v) => v.play());
  await page.waitForFunction(
    () =>
      document.querySelector("[data-guide-demo]").dataset.audioState ===
      "stopped",
  );
  await play.click();
  await page.waitForFunction(
    () => document.querySelector(".guide-recording video").paused,
  );
  console.log("Interactive preview and recording share playback: PASS");
} finally {
  await browser.close();
}
