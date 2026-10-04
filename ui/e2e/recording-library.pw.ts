import { test, expect } from "@playwright/test";

// The recording library (browse, search, preview, metadata) rendered only in
// an always-hidden panel until 2026-10-04; it now lives in the Recording tab.
const take = (id: string, path: string, title: string | null) => ({
  id, sessionId: "demo-session", recorderId: "voice-recording", path, format: "wav", channels: 2, sampleRate: 48000,
  frames: 48000 * 95, fileBytes: 18_240_044, startTime: "2026-10-04T18:44:19Z", state: "completed", missing: false,
  title, artist: null, comment: null, dither: false, conversion: "none",
});
const recordings = [
  take("take-1", "C:\\Users\\me\\Music\\AudioRouter\\voice-2026-10-04-1844.wav", "Ranked match voice"),
  take("take-2", "C:\\Users\\me\\Music\\AudioRouter\\voice-2026-10-04-1949.wav", null),
];

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`recording library is reachable in the Recording tab in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(({ theme, recordings }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, { __routeFixtureRecordings: recordings });
    }, { theme, recordings });
    await page.goto("/route-harness.html");
    await page.getByRole("tab", { name: "Recording", exact: true }).click();
    const library = page.getByLabel("Recording library");
    await expect(library).toBeVisible();
    await expect(library.getByRole("heading", { name: "Recordings" })).toBeVisible();
    await expect(library).toContainText("voice-2026-10-04-1844.wav");
    await expect(library).toContainText("voice-2026-10-04-1949.wav");
    // An untitled take is named by its file, not its full path.
    await expect(library.getByRole("article", { name: "voice-2026-10-04-1949.wav" })).toBeVisible();
    // Search narrows the list.
    await library.getByLabel("Search recordings").fill("1949");
    await expect(library).not.toContainText("voice-2026-10-04-1844.wav");
    await expect(library).toContainText("1 of 2");
    await library.getByLabel("Search recordings").fill("");
    // The library sits below the recorder in the sidebar's own scroll area.
    await library.getByRole("heading", { name: "Recordings" }).evaluate((heading) => heading.scrollIntoView({ block: "start" }));
    await page.locator(".right-workbench").screenshot({ path: testInfo.outputPath(`recording-library-${theme}.png`) });
  });
}
