import { defineConfig } from "@playwright/test";
import base from "./playwright.config";

// Build first: vite build --config vite.canvas-production.config.ts
// --configLoader runner --outDir ../target/usability-20261002-ui
export default defineConfig({
  ...base,
  testMatch: ["**/smart-layout.pw.ts", "**/drag-place.pw.ts", "**/usability-followups.pw.ts", "**/spatial-audio.pw.ts", "**/duck-icon.pw.ts", "**/duck-siege-round.pw.ts", "**/duck-widget.pw.ts", "**/eq-spectrum.pw.ts"],
  webServer: {
    command: "npx vite preview --configLoader runner --outDir ../target/usability-20261002-ui --host 127.0.0.1 --port 4186 --strictPort",
    url: "http://127.0.0.1:4186/route-harness.html",
    reuseExistingServer: false,
    timeout: 30_000,
  },
});
