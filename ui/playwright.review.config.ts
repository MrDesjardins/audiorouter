import { defineConfig } from "@playwright/test";
import base from "./playwright.config";

/** Reproducible UI review against disposable fixtures, never the live profile. */
export default defineConfig({
  ...base,
  testMatch: process.env.UI_REVIEW_FULL ? base.testMatch : "**/ui-quality-review.pw.ts",
  workers: 1,
  outputDir: `../target/ui-quality-review/${process.env.UI_REVIEW_PHASE ?? "current"}/${process.env.UI_REVIEW_FULL ? "matrix" : "focused"}-results`,
  webServer: {
    ...base.webServer,
    command: "npm run dev -- --host 127.0.0.1 --port 4186 --strictPort",
    url: "http://127.0.0.1:4186/route-harness.html",
    reuseExistingServer: true,
  },
});
