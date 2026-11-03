import { test, expect, initialSession } from "./real-backend";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`unsaved session selection is safe in ${theme}`, async ({ page, backend }, info) => {
    await backend.call("sessions.create", {
      session: { ...initialSession, id: "other-session", name: "Other saved setup" },
      idempotencyKey: "create-other",
    });
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/backend-harness.html");
    await expect(page.getByRole("heading", { name: initialSession.name, exact: true })).toBeVisible();
    await page.getByRole("tab", { name: "Session", exact: true }).click();
    const panel = page.locator(".right-workbench");
    await panel.getByRole("button", { name: "Rename", exact: true }).click();
    await panel.getByLabel("Session name", { exact: true }).fill("Unsaved voice setup");
    await panel.getByLabel("Session name", { exact: true }).press("Enter");
    const canvas = page.getByLabel("Signal-flow graph", { exact: true });
    const before = await canvas.boundingBox();
    await page.getByLabel("Choose session").selectOption("other-session");
    await expect(page.locator(".global-action-message")).toContainText(
      "Save or discard your draft in Session before switching sessions.",
    );
    await expect(page.getByLabel("Choose session")).toHaveValue(initialSession.id);
    await expect(page.getByRole("heading", { name: "Unsaved voice setup", exact: true })).toBeVisible();
    await backend.call("sessions.active.set", { sessionId: "other-session", idempotencyKey: "external-select" });
    await expect(page.locator(".global-action-message")).toContainText(
      "Another client selected a different session. Your draft is preserved",
    );
    await expect(page.getByLabel("Choose session")).toHaveValue(initialSession.id);
    await expect(page.getByRole("heading", { name: "Unsaved voice setup", exact: true })).toBeVisible();
    expect(await canvas.boundingBox()).toEqual(before);
    expect(backend.methods).not.toContain("session.stop");
    expect((await backend.call("sessions.get", { sessionId: initialSession.id })).name).toBe(initialSession.name);
    await page.screenshot({ path: info.outputPath(`draft-preserved-${theme}.png`) });
    await panel.getByRole("button", { name: "Revert edits", exact: true }).click();
    await page.getByLabel("Choose session").selectOption("other-session");
    await expect(page.getByRole("heading", { name: "Other saved setup", exact: true })).toBeVisible();
  });
}
