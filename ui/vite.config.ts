import os from "node:os";
import path from "node:path";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  // Tauri serves the bundled frontend from its app protocol rather than a
  // host-rooted web server. Relative assets are required for the shell to
  // load the entry module and reach the native IPC bridge.
  base: "./",
  // Keep the dependency optimizer out of node_modules. Some managed or
  // copied workspaces expose dependencies read-only, while the repository
  // itself remains writable for local UI inspection.
  cacheDir: process.env.AUDIOROUTER_VITE_CACHE ?? path.join(os.tmpdir(), "audiorouter-vite-cache"),
  plugins: [react()],
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
