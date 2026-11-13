import path from "node:path";
import { readFile } from "node:fs/promises";
import { test, expect, initialSession } from "./real-backend";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`invalid saved session stays recoverable in ${theme}`, async ({ page, backend }) => {
    await backend.call("sessions.create", {
      session: { ...initialSession, id: "broken", name: "Unavailable setup" },
      idempotencyKey: "create-broken",
    });
    await backend.call("fixture.corruptSessionForStartup", { sessionId: "broken" });
    await backend.restart();
    expect((await backend.call("status.get")).quarantinedSessions[0].sessionId).toBe("broken");
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/backend-harness.html");
    await expect(page.getByRole("heading", { name: initialSession.name, exact: true })).toBeVisible();
    const recovery = page.getByRole("button", { name: "Open session recovery", exact: true });
    await expect(recovery).toBeVisible({ timeout: 15_000 });
    await recovery.click();
    const panel = page.getByRole("region", { name: "Unavailable sessions", exact: true });
    await expect(panel).toContainText("broken");
    const downloading = page.waitForEvent("download");
    await panel.getByRole("button", { name: "Export original file", exact: true }).click();
    const download = await downloading;
    expect(await readFile((await download.path())!, "utf8")).toBe("not json");
    await panel.getByRole("button", { name: "Delete unavailable session", exact: true }).click();
    await expect(panel.getByRole("button", { name: "Confirm deletion", exact: true })).toBeVisible();
    await panel.screenshot({ path: path.resolve(`../target/audit-evidence/recovery-${theme}.png`) });
    await panel.getByRole("button", { name: "Keep session", exact: true }).click();
    expect((await backend.call("status.get")).quarantinedSessions).toHaveLength(1);
    await panel.getByRole("button", { name: "Delete unavailable session", exact: true }).click();
    await panel.getByRole("button", { name: "Confirm deletion", exact: true }).click();
    await expect(panel).toBeHidden();
    expect((await backend.call("sessions.list")).items).toHaveLength(1);
    expect((await backend.call("status.get")).quarantinedSessions).toEqual([]);
  });
}
