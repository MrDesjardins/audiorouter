import { expect, test as base, type Route } from "@playwright/test";
import { stat } from "node:fs/promises";
import path from "node:path";
export { expect } from "@playwright/test";
export type { Page, Locator } from "@playwright/test";

async function fulfillBundledAsset(route: Route, response: Parameters<Route["fulfill"]>[0]) {
  try {
    await route.fulfill(response);
  } catch (error) {
    // A page-level route or browser teardown can claim the request after the
    // context handler has started reading the fixture. That is an expected
    // teardown race; do not turn it into a flaky test. Other routing errors
    // still fail the fixture and remain visible.
    if (!(error instanceof Error) || !/route is already handled/i.test(error.message)) throw error;
  }
}

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
      const pending = new Set<Promise<void>>();
      const entries = new Set(["/", "/index.html", "/harness.html", "/route-harness.html", "/backend-harness.html"]);
      // Only compiled fixture assets use this transport. Real HTTP endpoints,
      // file:// guides and the page-level Rust RPC adapter remain untouched.
      const matches = (url: URL) =>
        url.origin === "http://127.0.0.1:4186" && (entries.has(url.pathname) || url.pathname.startsWith("/assets/"));
      const serveBundledAsset = (route: Parameters<Parameters<typeof context.route>[1]>[0]) => {
        const operation = (async () => {
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
            await fulfillBundledAsset(route, {
              status: 404,
              contentType: "text/plain",
              body: "Missing bundled fixture asset",
            });
            return;
          }
          served.push(pathname);
          await fulfillBundledAsset(route, { path: file });
        })();
        pending.add(operation);
        void operation.then(
          () => pending.delete(operation),
          () => pending.delete(operation),
        );
        return operation;
      };
      await context.route(matches, serveBundledAsset);
      try {
        await use(served);
      } finally {
        // Wait for this fixture's in-flight chunks without removing routes
        // independently registered by the scenario.
        await context.unroute(matches, serveBundledAsset);
        await Promise.allSettled([...pending]);
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
      // Scenarios add tools by name, so every Tools group starts open. tools-list.pw.ts
      // removes this and covers the real first-run state.
      await page.addInitScript(() => {
        try {
          // The goal dialog and tour have their own first-run test.
          if (localStorage.getItem("audiorouter.ui.first-run-done") === null)
            localStorage.setItem("audiorouter.ui.first-run-done", "true");
          if (localStorage.getItem("audiorouter.ui.tools-open-groups") === null)
            localStorage.setItem(
              "audiorouter.ui.tools-open-groups",
              JSON.stringify(["input", "output", "clean", "level", "tone", "routing", "canvas", "plugins", "setup"]),
            );
        } catch {
          /* A scenario may deny storage on purpose. */
        }
      });
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
