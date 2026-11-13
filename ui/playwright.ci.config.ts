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
  use: { ...base.use, bundledAssets: fileURLToPath(new URL("../target/browser-acceptance-ui", import.meta.url)) },
  webServer: undefined,
});
