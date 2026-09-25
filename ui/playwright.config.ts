import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: "**/*.pw.ts",
  fullyParallel: false,
  reporter: [["list"]],
  outputDir: `${process.env.TEMP ?? "."}/audiorouter-playwright-results`,
  use: { baseURL: "http://127.0.0.1:4174", browserName: "chromium", channel: "msedge", viewport: { width: 1440, height: 1000 }, trace: "retain-on-failure", screenshot: "only-on-failure" },
  webServer: { command: "npm run dev -- --host 127.0.0.1 --port 4174 --strictPort", url: "http://127.0.0.1:4174/harness.html", reuseExistingServer: !process.env.CI, timeout: 30_000 },
});
