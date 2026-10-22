import { expect, test as base } from "@playwright/test";
export { expect } from "@playwright/test";
export type { Page, Locator } from "@playwright/test";

/** Uncaught application errors fail the scenario even if its UI assertions pass. */
export const test = base.extend<{ pageErrorGuard: void }>({
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
