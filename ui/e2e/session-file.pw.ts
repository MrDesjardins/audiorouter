import path from "node:path";
import { existsSync } from "node:fs";
import { test, expect } from "./real-backend";

// A whole session can be saved to one .audiorouter file and opened again as
// a new session (for backups and moving a setup to another PC).

test("Session file: save to a file and open it as a new session without replacing the original", async ({ page, backend }) => {
  const file = path.join(backend.directory, "My setup.audiorouter");
  await page.goto("/backend-harness.html");
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  await page.getByRole("tab", { name: "Session", exact: true }).click();
  const panel = page.locator(".session-file");
  await expect(panel.getByRole("heading", { name: "Session file" })).toBeVisible();
  await panel.getByRole("textbox", { name: "Session file path" }).fill(file);
  await panel.getByRole("button", { name: "Save to file…" }).click();
  await expect(panel.getByRole("status")).toContainText("Saved \"Offline qualification\"");
  expect(existsSync(file)).toBe(true);

  await panel.getByRole("button", { name: "Save to file…" }).click();
  await expect(panel.getByRole("status")).toContainText("already exists; choose another name");

  await panel.getByRole("button", { name: "Open file…" }).click();
  await expect(panel.getByRole("status")).toContainText("Opened \"Offline qualification (imported)\" as a new session.");
  await expect(panel.getByRole("status")).toContainText("nothing was replaced");
  const sessions = await backend.call("sessions.list", {});
  expect(sessions.items.map((item: { name: string }) => item.name).sort()).toEqual(["Offline qualification", "Offline qualification (imported)"]);
  await expect(page.getByRole("combobox", { name: "Choose session" })).toHaveValue(/imported/);
});
