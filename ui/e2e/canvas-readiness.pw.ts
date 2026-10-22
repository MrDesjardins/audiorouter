import { test, expect } from "./real-backend";

test("readiness identifies the applied session and revision on cold load, reload and commit", async ({ page }) => {
  await page.goto("/backend-harness.html");
  const ready = page.getByTestId("canvas-readiness");
  await expect(ready).toHaveAttribute("data-session-id", "e2e-session");
  await expect(ready).toHaveAttribute("data-revision", "0");
  await expect(ready).toHaveAttribute("data-ready", "true");
  await page.reload();
  await expect(ready).toHaveAttribute("data-ready", "true");
  await expect(ready).toHaveAttribute("data-session-id", "e2e-session");
  await page
    .locator(".tool-card")
    .filter({ has: page.getByText("Gain", { exact: true }) })
    .click();
  await expect(page.getByTestId("rf__node-gain-1")).toBeVisible();
  await expect(ready).toHaveAttribute("data-ready", "true");
  await page.locator(".topbar").getByRole("button", { name: "Save", exact: true }).click();
  await expect(ready).toHaveAttribute("data-revision", "1");
  await expect(ready).toHaveAttribute("data-ready", "true");
});

test("no ready canvas is exposed before the initial backend snapshot", async ({ page }) => {
  let release!: () => void;
  const blocked = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route("**/__e2e_rpc", async (route) => {
    if (route.request().postDataJSON().method === "status.get") await blocked;
    await route.fallback();
  });
  await page.goto("/backend-harness.html");
  await expect(page.getByRole("main", { name: "Loading saved session" })).toBeVisible();
  await expect(page.getByTestId("canvas-readiness")).toHaveCount(0);
  release();
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
});

test("an empty new session has its own ready identity", async ({ page }) => {
  await page.goto("/backend-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await page.getByRole("tab", { name: "Session", exact: true }).click();
  page.once("dialog", (dialog) => dialog.accept("Readiness session"));
  await page.getByRole("button", { name: "New", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Readiness session", exact: true })).toBeVisible();
  const sessionId = await page.getByLabel("Choose session").inputValue();
  expect(sessionId).not.toBe("e2e-session");
  const ready = page.getByTestId("canvas-readiness");
  await expect(ready).toHaveAttribute("data-session-id", sessionId);
  await expect(ready).toHaveAttribute("data-ready", "true");
});

test("exhausted connector measurement cannot report readiness", async ({ page }) => {
  await page.addInitScript(() => {
    const query = Element.prototype.querySelectorAll;
    Element.prototype.querySelectorAll = function (this: Element, selector: string) {
      if (this.classList.contains("react-flow__node") && [".source", ".target"].includes(selector))
        return query.call(this, ":not(*)");
      return query.call(this, selector);
    } as typeof query;
  });
  await page.goto("/backend-harness.html");
  const ready = page.getByTestId("canvas-readiness");
  await expect(ready).toHaveAttribute("data-state", "exhausted");
  await expect(ready).toHaveAttribute("data-ready", "false");
});
