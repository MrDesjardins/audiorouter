import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: "**/*.pw.ts",
  fullyParallel: false,
  // CI runs one worker: two continuously animated live tests side by side
  // stalled Edge's "element is stable" waits (run 37560544251). PowerShell
  // drops a `--` separator, so this cannot be passed through `npm run e2e`.
  workers: process.env.CI ? 1 : undefined,
  reporter: [["list"]],
  outputDir: `${process.env.TEMP ?? "."}/audiorouter-playwright-results`,
  use: { baseURL: "http://127.0.0.1:4186", browserName: "chromium", channel: "msedge", viewport: { width: 1440, height: 1000 }, trace: "retain-on-failure", screenshot: "only-on-failure" },
  webServer: { command: "npm run dev -- --host 127.0.0.1 --port 4186 --strictPort", url: "http://127.0.0.1:4186/harness.html", reuseExistingServer: false, timeout: 30_000 },
});
