import { chromium } from "../../ui/node_modules/playwright-core/index.mjs";
import { mkdirSync } from "node:fs";
mkdirSync("target/tool-guide-layout", { recursive: true });
const base = "http://127.0.0.1:4187/site/tools";
const browser = await chromium.launch({ channel: "msedge" });
const fail = (message) => {
  throw new Error(message);
};
try {
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  for (const kind of [
    "gate",
    "parametric-eq",
    "soundboard",
    "physical-input",
    "fun-voice-changer",
  ]) {
    await page.goto(`${base}/${kind}.html#in-the-app`);
    await page.waitForFunction(
      () => !document.getElementById("panel-videos").hidden,
    );
    const state = await page.evaluate(() => ({
      overflow: document.documentElement.scrollWidth > innerWidth,
      selected: document.querySelector('[role="tab"][aria-selected="true"]')
        .dataset.tab,
      videos: [...document.querySelectorAll("#in-the-app video")].map((v) => ({
        controls: v.controls,
        preload: v.preload,
        autoplay: v.autoplay,
        box: v.getBoundingClientRect().width,
      })),
    }));
    if (
      state.overflow ||
      state.selected !== "videos" ||
      state.videos.length !== 1 ||
      state.videos.some(
        (v) => !v.controls || v.autoplay || v.preload !== "none" || v.box > 390,
      )
    )
      fail(`${kind}: ${JSON.stringify(state)}`);
    await page.waitForTimeout(700);
    await page.screenshot({ path: `target/tool-guide-layout/${kind}.png` });
    console.log(`${kind}: phone layout PASS`);
  }

  // Lazy media: a page with 37 clips requests one poster and no video until
  // the reader picks a clip.
  const media = [];
  const onRequest = (r) => {
    if (/\/tool-guides\//.test(r.url())) media.push(r.url());
  };
  page.on("request", onRequest);
  await page.goto(`${base}/fun-voice-changer.html`);
  await page.waitForLoadState("networkidle");
  page.off("request", onRequest);
  const posters = media.filter((u) => /\.jpg/.test(u)).length;
  const videos = media.filter((u) => /\.mp4/.test(u)).length;
  if (posters > 1 || videos)
    fail(`fun-voice-changer loaded ${posters} posters, ${videos} videos`);
  console.log(`Lazy media: ${posters} poster, ${videos} videos on load PASS`);

  // A shared property anchor opens Settings; "Watch" opens Videos on its clip.
  await page.goto(`${base}/compressor.html#property-ratio`);
  await page.waitForFunction(
    () =>
      !document.getElementById("panel-settings").hidden &&
      document.getElementById("panel-overview").hidden,
  );
  await page.waitForTimeout(500);
  const inView = await page.evaluate(() => {
    const r = document.getElementById("property-ratio").getBoundingClientRect();
    return r.top >= 0 && r.top < innerHeight;
  });
  if (!inView) fail("compressor: #property-ratio not scrolled into view");
  await page.locator("#property-ratio .property-watch").click();
  await page.waitForFunction(
    () =>
      !document.getElementById("panel-videos").hidden &&
      document
        .querySelector("#in-the-app source")
        .getAttribute("src")
        .includes("compressor--ratio.mp4") &&
      document
        .getElementById("clip-compressor--ratio")
        .hasAttribute("aria-current"),
  );
  if (!page.url().endsWith("#clip-compressor--ratio"))
    fail(`Watch link did not update the address: ${page.url()}`);
  await page.goBack();
  await page.waitForFunction(
    () => !document.getElementById("panel-settings").hidden,
  );
  console.log("Anchors open their tab, Watch selects its clip PASS");

  // Keyboard: arrows move between tabs (roving tabindex).
  await page.goto(`${base}/compressor.html`);
  await page.locator("#tab-overview").focus();
  await page.keyboard.press("ArrowRight");
  const keyed = await page.evaluate(() => ({
    focus: document.activeElement.id,
    shown: !document.getElementById("panel-settings").hidden,
  }));
  if (keyed.focus !== "tab-settings" || !keyed.shown)
    fail(`keyboard tabs: ${JSON.stringify(keyed)}`);
  await page.keyboard.press("End");
  await page.keyboard.press("Home");
  if (
    (await page.evaluate(() => document.activeElement.id)) !== "tab-overview"
  )
    fail("Home did not return to the first tab");
  console.log("Keyboard tab navigation PASS");

  await page.goto(`${base}/gate.html`);
  const play = page.getByRole("button", { name: "Play example", exact: true });
  await play.click();
  await page.waitForFunction(
    () =>
      document.querySelector("[data-guide-demo]").dataset.audioState ===
      "playing",
  );
  await page.locator("#tab-videos").click();
  await page.waitForFunction(
    () =>
      document.querySelector("[data-guide-demo]").dataset.audioState !==
      "playing",
  );
  await page.locator(".guide-player video").evaluate((v) => v.play());
  await page.locator("#tab-overview").click();
  await page.waitForFunction(
    () => document.querySelector(".guide-player video").paused,
  );
  await play.click();
  await page.waitForFunction(
    () =>
      document.querySelector("[data-guide-demo]").dataset.audioState ===
      "playing",
  );
  console.log("Tab switches pause the example and the recording: PASS");

  const desktop = await browser.newPage({
    viewport: { width: 1440, height: 900 },
  });
  for (const [kind, tab] of [
    ["compressor", "overview"],
    ["compressor", "settings"],
    ["fun-voice-changer", "videos"],
  ]) {
    await desktop.goto(`${base}/${kind}.html`);
    await desktop.locator(`#tab-${tab}`).click();
    await desktop.evaluate(() =>
      scrollTo({
        top:
          document.querySelector(".guide-article").getBoundingClientRect()
            .top +
          scrollY -
          90,
        behavior: "instant",
      }),
    );
    await desktop.waitForTimeout(400);
    await desktop.screenshot({
      path: `target/tool-guide-layout/desktop-${kind}-${tab}.png`,
    });
  }
  console.log("Desktop screenshots written to target/tool-guide-layout");
} finally {
  await browser.close();
}
