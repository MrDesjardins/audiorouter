import { defineConfig, mergeConfig } from "vite";
import base from "./vite.config";

// Compile every entry used by the full browser suite before opening Edge.
// Long CI runs must not depend on the dev ESM graph and HMR requests per page.
// Keep this separate from ui/dist, which is embedded in the desktop shell.
export default mergeConfig(base, defineConfig({
  build: {
    outDir: "../target/browser-acceptance-ui",
    emptyOutDir: true,
    rollupOptions: { input: ["index.html", "harness.html", "route-harness.html", "backend-harness.html"] },
  },
}));
