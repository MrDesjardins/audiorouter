/// <reference types="vitest/config" />
import os from "node:os";
import path from "node:path";
import { readFileSync } from "node:fs";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The app version shown in the header; release bumps keep it equal to the
// shell and backend versions.
const appVersion = (JSON.parse(readFileSync(new URL("./package.json", import.meta.url), "utf8")) as { version: string }).version;

export default defineConfig({
  define: { __APP_VERSION__: JSON.stringify(appVersion) },
  // Tauri serves the bundled frontend from its app protocol rather than a
  // host-rooted web server. Relative assets are required for the shell to
  // load the entry module and reach the native IPC bridge.
  base: "./",
  // Keep the dependency optimizer out of node_modules. Some managed or
  // copied workspaces expose dependencies read-only, while the repository
  // itself remains writable for local UI inspection.
  cacheDir: process.env.AUDIOROUTER_VITE_CACHE ?? path.join(os.tmpdir(), "audiorouter-vite-cache"),
  plugins: [react()],
  // Used only with `npm test -- --coverage` (nightly quality workflow); the
  // report is informational and never gates a change.
  test: {
    coverage: {
      provider: "v8",
      include: ["src/**"],
      exclude: ["src/**/*.test.{ts,tsx}"],
      reporter: ["text-summary", "json-summary", "html"],
      reportsDirectory: "coverage",
    },
  },
  build: {
    rollupOptions: {
      output: {
        // Keep the graph-editor dependency out of the initial application
        // chunk. This makes the packaged shell's startup payload explicit
        // and prevents a single entry chunk from crossing the warning
        // threshold as the editor grows.
        manualChunks: {
          "xyflow-vendor": ["@xyflow/react"],
        },
      },
    },
  },
});
