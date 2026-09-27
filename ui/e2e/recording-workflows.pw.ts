import { test, expect } from "./real-backend";
import { readFile } from "node:fs/promises";

test("Recorder UI creates real WAV, arms, pauses, resumes and finalizes synthetic frames", async ({ page, backend }) => {
  await page.goto("/backend-harness.html");
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  await page.getByRole("tab", { name: "Recording", exact: true }).click();
  const panel = page.locator(".right-workbench .recorder-actions");
  await panel.getByLabel("Recorder ID", { exact: true }).fill("synthetic-take");
  await panel.getByLabel("Recorder format", { exact: true }).selectOption("wavFloat32");
  await expect(panel.getByLabel("TPDF dither", { exact: true })).toBeDisabled();
  await expect(panel.getByLabel("TPDF dither", { exact: true })).not.toBeChecked();
  await panel.getByRole("button", { name: "Create recorder", exact: true }).click();
  await expect(panel).toContainText(/created|attached/i);
  const run = async (action: string, frame: number, state: string) => {
    await panel.getByLabel("Recorder engine frame", { exact: true }).fill(String(frame));
    await panel.getByRole("button", { name: action, exact: true }).click();
    try { await expect(panel.locator(".badge")).toHaveText(state); }
    catch (error) { throw new Error(`${action}: ${await panel.innerText()}\n${error}`); }
  };
  await run("Arm", 0, "armed");
  await run("Start", 0, "recording");
  await backend.call("fixture.recordSyntheticQuantum", { sessionId: "e2e-session", frame: 0 });
  await run("Pause", 128, "paused");
  await backend.call("fixture.recordSyntheticQuantum", { sessionId: "e2e-session", frame: 128 });
  await run("Resume", 256, "recording");
  await backend.call("fixture.recordSyntheticQuantum", { sessionId: "e2e-session", frame: 256 });
  await run("Stop", 384, "completed");
  const rows = await backend.call("recordings.list", { sessionId: "e2e-session" });
  expect(rows).toHaveLength(1);
  expect(rows[0]).toMatchObject({ state: "completed", channels: 2, sampleRate: 48000, dither: false, frames: 256 });
  const bytes = await readFile(rows[0].path);
  expect(bytes.subarray(0, 4).toString()).toBe("RIFF");
  expect(bytes.subarray(8, 12).toString()).toBe("WAVE");
  expect(bytes.length).toBeGreaterThanOrEqual(256 * 2 * 4 + 44);
  await backend.restart();
  expect(await backend.call("recordings.list", { sessionId: "e2e-session" })).toMatchObject(rows);
  expect((await backend.call("status.get")).activeSessionCount).toBe(0);
});
