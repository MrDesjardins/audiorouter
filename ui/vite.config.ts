import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  // Tauri serves the bundled frontend from its app protocol rather than a
  // host-rooted web server. Relative assets are required for the shell to
  // load the entry module and reach the native IPC bridge.
  base: "./",
  plugins: [react()],
});
