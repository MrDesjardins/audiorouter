import { defineConfig } from "@playwright/test";
import base from "./playwright.config";

export default defineConfig({
  ...base,
  testMatch: [
    "**/playing-canvas-lines.pw.ts", "**/canvas-cold-start.pw.ts", "**/inspector-status.pw.ts",
    "**/live-inspector-regressions.pw.ts", "**/smart-layout.pw.ts", "**/meter-inspector.pw.ts",
    "**/dynamics-editor.pw.ts", "**/drag-place.pw.ts", "**/canvas-layout-review.pw.ts",
    "**/canvas-layout.pw.ts", "**/canvas-layout-session.pw.ts", "**/canvas-group-lock.pw.ts",
    "**/canvas-groups.pw.ts", "**/canvas-copy-paste.pw.ts", "**/canvas-clipboard-regressions.pw.ts",
    "**/session-workflows.pw.ts", "**/recording-workflows.pw.ts",
    "**/canvas-readiness.pw.ts",
  ],
  outputDir: `${process.env.TEMP ?? "."}/audiorouter-production-playwright-results`,
  webServer: {
    command: "npx vite preview --configLoader runner --outDir ../target/canvas-production-ui --host 127.0.0.1 --port 4186 --strictPort",
    url: "http://127.0.0.1:4186/route-harness.html",
    reuseExistingServer: false,
    timeout: 30_000,
  },
});
