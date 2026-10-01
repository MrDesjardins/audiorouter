import { defineConfig, mergeConfig } from "vite";
import base from "./vite.config";

// Exercise the same optimized graph/vendor chunks as the desktop build,
// using the deterministic no-device backend already used by UI acceptance.
export default mergeConfig(base, defineConfig({
  build: {
    outDir: "../target/canvas-production-ui",
    rollupOptions: { input: "route-harness.html" },
  },
}));
