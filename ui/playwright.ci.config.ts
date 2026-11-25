import { defineConfig } from "@playwright/test";
import base from "./playwright.config";
import { fileURLToPath } from "node:url";
import type { BrowserOptions } from "./e2e/browser-test";
import { backendSpecs } from "../tools/quality/browser-scope.mjs";

// Same full scenario discovery, assertions and failure policy as development;
// compiled fixture assets are delivered directly, without hosted loopback TCP.
// The actual packaged embedded-asset path is exercised by its separate gate.
export default defineConfig<BrowserOptions>({
  ...base,
  testMatch: process.env.BROWSER_SCOPE === "backend" ? backendSpecs(fileURLToPath(new URL("./e2e", import.meta.url))) : base.testMatch,
  // Q5.1: a shard keeps a machine-readable report so the nightly trend records
  // first-pass failures separately from a later green retry.
  reporter: process.env.PLAYWRIGHT_JSON_OUTPUT_NAME
    ? [["list"], ["json", { outputFile: process.env.PLAYWRIGHT_JSON_OUTPUT_NAME }]]
    : base.reporter,
  use: { ...base.use, bundledAssets: fileURLToPath(new URL("../target/browser-acceptance-ui", import.meta.url)) },
  webServer: undefined,
});
