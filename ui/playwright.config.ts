import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: "**/*.pw.ts",
  fullyParallel: false,
  // CI runs one worker: two continuously animated live tests side by side
  // stalled Edge's "element is stable" waits (run 37560544251). PowerShell
  // drops a `--` separator, so this cannot be passed through `npm run e2e`.
  workers: process.env.CI ? 1 : undefined,
  // Hosted runners stall for seconds at a time (cold Vite compiles, GC): a
  // first page load missed the 5 s default (2026-10-23). One diagnostic
  // retry tells a flake from a steady failure, but a flaky test still fails
  // the job: a retry is never first-pass success (application-quality.md).
  expect: { timeout: process.env.CI ? 10_000 : 5_000 },
  retries: process.env.CI ? 1 : 0,
  failOnFlakyTests: !!process.env.CI,
  reporter: [["list"]],
  outputDir: `${process.env.TEMP ?? "."}/audiorouter-playwright-results`,
  use: { baseURL: "http://127.0.0.1:4186", browserName: "chromium", channel: "msedge", viewport: { width: 1440, height: 1000 }, trace: "retain-on-failure", screenshot: "only-on-failure" },
  webServer: { command: "npm run dev -- --host 127.0.0.1 --port 4186 --strictPort", url: "http://127.0.0.1:4186/harness.html", reuseExistingServer: false, timeout: 30_000 },
});
