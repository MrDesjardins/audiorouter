import { defineConfig } from "@playwright/test";
import base from "./playwright.config";

// Same full scenario discovery, assertions and failure policy as development;
// only the server changes to prebuilt assets without the dev module graph/HMR.
export default defineConfig({
  ...base,
  webServer: {
    command: "npx vite preview --configLoader runner --outDir ../target/browser-acceptance-ui --host 127.0.0.1 --port 4186 --strictPort",
    url: "http://127.0.0.1:4186/harness.html",
    reuseExistingServer: false,
    timeout: 30_000,
  },
});
