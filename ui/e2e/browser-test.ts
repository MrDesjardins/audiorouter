import { expect, test as base } from "@playwright/test";
import { stat } from "node:fs/promises";
import path from "node:path";
export { expect } from "@playwright/test";
export type { Page, Locator } from "@playwright/test";

/** Uncaught application errors fail the scenario even if its UI assertions pass. */
export type BrowserOptions = { bundledAssets?: string };
export const test = base.extend<BrowserOptions & { pageErrorGuard: void; bundledAssetRequests: string[] }>({
  bundledAssets: [undefined, { option: true }],
  bundledAssetRequests: [
    async ({ context, bundledAssets }, use, testInfo) => {
      const served: string[] = [];
      if (!bundledAssets) {
        await use(served);
        return;
      }
      const root = path.resolve(bundledAssets);
      const missing: string[] = [];
      const entries = new Set(["/", "/index.html", "/harness.html", "/route-harness.html", "/backend-harness.html"]);
      // Only compiled fixture assets use this transport. Real HTTP endpoints,
      // file:// guides and the page-level Rust RPC adapter remain untouched.
      const matches = (url: URL) =>
        url.origin === "http://127.0.0.1:4186" && (entries.has(url.pathname) || url.pathname.startsWith("/assets/"));
      await context.route(matches, async (route) => {
        const pathname = decodeURIComponent(new URL(route.request().url()).pathname);
        const file = path.resolve(root, `.${pathname === "/" ? "/index.html" : pathname}`);
        const inside = file.startsWith(root + path.sep) && !pathname.includes("\\");
        if (
          !inside ||
          !(await stat(file).then(
            (info) => info.isFile(),
            () => false,
          ))
        ) {
          missing.push(pathname);
          await route.fulfill({ status: 404, contentType: "text/plain", body: "Missing bundled fixture asset" });
          return;
        }
        served.push(pathname);
        await route.fulfill({ path: file });
      });
      try {
        await use(served);
      } finally {
        await context.unroute(matches);
        if (missing.length || testInfo.status !== testInfo.expectedStatus)
          await testInfo.attach("bundled-fixture-assets", {
            body: JSON.stringify({ served, missing }),
            contentType: "application/json",
          });
        expect(missing, "No missing bundled fixture assets").toEqual([]);
      }
    },
    { auto: true },
  ],
  pageErrorGuard: [
    async ({ page }, use, testInfo) => {
      const errors: string[] = [];
      const collect = (error: Error) => errors.push(error.message);
      page.on("pageerror", collect);
      try {
        await use();
      } finally {
        page.off("pageerror", collect);
        if (errors.length)
          await testInfo.attach("uncaught-browser-errors", {
            body: JSON.stringify(errors, null, 2),
            contentType: "application/json",
          });
        expect(errors, "No uncaught UI exceptions").toEqual([]);
      }
    },
    { auto: true },
  ],
});
