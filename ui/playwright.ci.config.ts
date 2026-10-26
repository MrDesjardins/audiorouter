import { defineConfig } from "@playwright/test";
import base from "./playwright.config";
import path from "node:path";
import type { BrowserOptions } from "./e2e/browser-test";

// Same full scenario discovery, assertions and failure policy as development;
// compiled fixture assets are delivered directly, without hosted loopback TCP.
// The actual packaged embedded-asset path is exercised by its separate gate.
export default defineConfig<BrowserOptions>({
  ...base,
  use: { ...base.use, bundledAssets: path.resolve("../target/browser-acceptance-ui") },
  webServer: undefined,
});
